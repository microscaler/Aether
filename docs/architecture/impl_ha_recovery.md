# High Availability: Fencing, Heartbeat & Recovery

Deep-dive companion to [ARCHITECTURE.md](../../ARCHITECTURE.md). Covers how Aether
detects a failed node, guarantees it is safely dead, and re-places the workloads
it was running — the STONITH-and-recover loop that turns a pile of blades into a
self-healing cluster.

The system is deliberately built from three separable concerns, each behind a
trait so it is testable in isolation:

| Concern | Question it answers | Home |
| :--- | :--- | :--- |
| **Heartbeat / deadman** | Is the node's agent alive? | `aetherd::heartbeat`, `registry` |
| **Fencing (STONITH)** | Make the node *definitely* dead | `aether-fence`, `aggregator::fencing` |
| **Recovery** | Re-run the orphaned workloads elsewhere | `aggregator::recovery`, `aggregator::placement` |

The overriding safety property is **no split-brain**: a workload's disk is a
shared iSCSI LUN (see the [storage-node deep-dive](./impl_storage_node.md)), so
two copies of a VM writing the same LUN corrupts data. Recovery must never start
the replacement until the original is guaranteed powered off.

---

## 1. The control loop

The aggregator runs one background task (`aether-aggregator/src/main.rs`) every
**3 s**:

1. **Prune** — under the registry write lock, drop any node whose last heartbeat
   is older than **15 s** (`NodeRegistry::prune_inactive_nodes`).
2. **Exclude storage** — snapshot the `POOL_STORAGE` node ids *before* pruning
   and remove them from the pruned set. Storage nodes host no workloads and must
   never be STONITH'd (a powered-off storage head takes every VM's disk with it).
3. **Fence** — `NodeFencer::fence_pruned` powers off each remaining pruned node
   out-of-band, subject to the corroboration veto (§3).
4. **Recover** — for each successfully fenced node, `RecoveryService::recover_node`
   re-auctions and re-launches its orphaned workloads.

```text
    heartbeat lapse (15s)
          │
          ▼
   prune_inactive_nodes ──► [is POOL_STORAGE?] ──yes──► skip (never fence)
          │ no
          ▼
   LivenessCorroborator.confirm_dead? ──no──► veto (skip STONITH)
          │ yes
          ▼
   ChassisManager.power_off(slot)   ← the node is now definitely dead
          │
          ▼
   RecoveryService.recover_node ──► re-auction ──► ExecuteVM on the winner
```

---

## 2. Heartbeat / deadman switch

Liveness of the *workload plane* — "is aetherd alive and doing its job?" — is
Aether's own signal, not something borrowed from the hardware. It is what decides
*when* to suspect a node.

**Node side** (`aetherd/src/heartbeat.rs`). After a node registers and receives
its attestation token, `spawn_heartbeat` starts a background task that calls the
`SendHeartbeat` RPC every `DEFAULT_HEARTBEAT_INTERVAL` (**5 s** — comfortably
under the 15 s prune threshold, so a couple of dropped beats don't cause a
false-positive fence). Transient failures are logged and retried on the next
tick rather than aborting the loop. Both entrypoints wire it in: the compute
daemon in `main`, and the storage node in `run_storage_node`.

**Aggregator side** (`aether-aggregator/src/lib.rs`). `send_heartbeat` validates
the ephemeral token and calls `NodeRegistry::renew_heartbeat`, which bumps
`last_seen_heartbeat`. `register_node` issues the token and stores the node's
`NodeInfo { node_id, grpc_endpoint, pool, token, last_seen_heartbeat }`.

> **Why the heartbeat alone isn't enough.** A missed heartbeat is *ambiguous*: a
> control-network partition looks identical to a dead node. Fencing on that
> ambiguity — or worse, re-placing the VM while the "dead" node is still alive
> and writing to the shared LUN — is exactly the split-brain we must avoid. The
> heartbeat therefore *triggers investigation*; the fence is only committed after
> the out-of-band corroboration in §3.

---

## 3. Fencing (STONITH) and out-of-band corroboration

`aether-fence` is the out-of-band power-execution plane. The `ChassisManager`
trait (`power_off(slot)`, `get_power_status(slot)`) is implemented by a shared
`RedfishClient` core and the `HpeIloProvider` / `DellIdracProvider` drivers, with
a `MockChassisManager` for dev/test.

`aggregator::fencing::NodeFencer` maps a pruned `node_id` (`"blade-N"`) to its
chassis slot (`parse_slot_number`) and powers it off. `fence_pruned` is
best-effort: one unreachable BMC can't stall recovery of the others, and it
returns only the ids it actually fenced.

**The corroboration seam.** `NodeFencer` optionally holds a `LivenessCorroborator`:

```rust
#[async_trait]
pub trait LivenessCorroborator: Send + Sync {
    /// true  => confirmed dead/unreachable, proceed with STONITH
    /// false => appears alive out-of-band, veto the fence
    async fn confirm_dead(&self, node_id: &str) -> bool;
}
```

When set (`NodeFencer::with_corroborator`), `fence_pruned` consults it before
powering each node off and **skips** any node the OOB path reports alive. This is
the two-signal design: the heartbeat lapse triggers, and an *independent* signal
confirms. The default (no corroborator) is classic unconditional STONITH.

> **Implementation reality.** The trait and veto path are implemented and tested
> with a mock corroborator. A real `IloPowerCorroborator` (querying Redfish power
> state through the existing `ChassisManager`) or a `OneViewCorroborator`
> (subscribing to the HPE OneView State-Change Message Bus for power/health
> alerts) is the next step — the seam is where it drops in. The aggregator's
> prune loop currently constructs the fencer without a corroborator and with a
> `MockChassisManager`, pending real OOB credentials.

### Why OneView doesn't replace the heartbeat

OneView (and iLO) are *hardware*-aware, not *workload*-aware: they know whether a
blade is powered, healthy, and thermally sane, but not whether your daemon or a
given VM is running. They are the right source for the **corroboration** and the
**actuation** (power-off, and confirming it took effect), and OneView's SCMB can
push a hard-failure alert faster than a heartbeat timeout would notice. But they
are neither a sub-second cluster-membership mechanism nor aware of agent
liveness, so the heartbeat stays Aether's own. See ARCHITECTURE §1.D.

---

## 4. Recovery: re-auction the orphans

Once a node is fenced, its workloads must run somewhere else.
`aggregator::placement::PlacementRegistry` is the state that makes this possible:
a `workload_uuid → PlacementRecord` map recording which node runs each workload
plus the full request spec (cpu/mem/disk, image URI, and the **stable MAC** — see
the [network-identity deep-dive](./impl_network_identity.md)) needed to re-launch
it. `workloads_on_node(dead_node)` yields exactly the orphans.

`RecoveryService::recover_node` (`aggregator/src/recovery.rs`) walks the orphans
and, for each, runs the *same* path a fresh placement uses:

- `WorkloadPlacer::select_target` — a reverse-bid auction across the cluster,
  **excluding the dead node** (`AuctionPlacer` over the `Scheduler`).
- `WorkloadDispatcher::dispatch` — launch on the winner via aetherd's `ExecuteVM`
  RPC (`GrpcWorkloadDispatcher`).
- `PlacementRegistry::reassign` — move the placement record to the new node.

Because the placer and dispatcher are **shared** with the initial-placement
`PlacementService`, a recovered workload converges through exactly the code path
it was first placed by — no separate, under-tested "recovery mode." Recovery is
best-effort per workload (`RecoveryOutcome { recovered, no_capacity, failed }`):
one workload with no capacity doesn't block the others.

Crucially, the re-launch replays the workload's original MAC, so DCops/Kea hand
the recovered VM back its original IP, and the auction can be pointed at the
replicated copy of its disk on the standby storage node. Identity + IP + disk all
follow the VM to its new home — the three legs of a genuine recovery.

---

## 5. Status & gaps

| Piece | Status |
| :--- | :--- |
| Node registry + prune | ✅ Tested |
| Heartbeat loop (`spawn_heartbeat`) | 🟩 Wired into both entrypoints; covered by the aggregator-side heartbeat/prune integration tests |
| `aether-fence` drivers (iLO/iDRAC/Redfish) | ✅ Tested (mock chassis in dev; real OOB creds pending) |
| `NodeFencer` + `fence_pruned` | ✅ Tested, wired into the prune loop |
| `LivenessCorroborator` veto seam | 🟨 Trait + veto tested with a mock; no real iLO/OneView impl yet |
| Storage-pool fencing exclusion | ✅ Implemented + registry-tested |
| Recovery (`RecoveryService`) | ✅ Built & tested; shares placer/dispatcher with initial placement |
| Placement removal on VM teardown | ⛔ Not yet — teardown doesn't yet drop the placement record |
| Node heartbeats in production | 🟨 Loop exists; no jitter/backoff tuning or lease-loss shutdown yet |

Related: [network-identity & DCops IPAM](./impl_network_identity.md),
[storage node & provisioning](./impl_storage_node.md),
[bidding & scheduling](./impl_bidding_scheduling.md).
