# Network Identity: Stable MAC → DCops IPAM

Deep-dive companion to [ARCHITECTURE.md](../../ARCHITECTURE.md). Covers how a VM
keeps its network identity across a recovery, and the cross-repo contract between
Aether and **DCops** (the microscaler NetBox/IPAM + DHCP control plane).

## The division of ownership

> **Aether owns the MAC and the disk. DCops owns the IP.**

Aether does not allocate IP addresses — that is a DCops concern (NetBox IPAM +
the Kea DHCP controller). What Aether owns is the VM's **stable MAC address**.
Preserving the MAC across a recovery is precisely what lets a re-launched VM keep
its DHCP-assigned IP, its DNS record, and its L2 identity. IP allocation stays
where the system of record for addresses lives.

This is the third leg of a genuine recovery, alongside the disk (see the
[storage-node deep-dive](./impl_storage_node.md)) and the re-auction (see the
[HA deep-dive](./impl_ha_recovery.md)).

---

## 1. The MAC model (`aggregator/src/identity.rs`)

```rust
#[async_trait]
pub trait NetworkIdentityProvider: Send + Sync {
    /// Idempotent: the same workload always resolves to the same MAC, so
    /// recovery replays the original identity rather than minting a new one.
    async fn reserve_mac(&self, workload_uuid: &str) -> Result<String, String>;
}
```

The MAC is derived **deterministically** from the workload uuid via FNV-1a under
the QEMU locally-administered OUI `52:54:00` (`local_mac_from_uuid`). Determinism
is the key property: recovery replays the exact same MAC with no read-back and no
persisted state.

- `LocalMacAllocator` — the dependency-free dev/default. Derives the MAC and
  returns it, nothing else.
- `DcopsNetworkIdentityProvider<P: IpamClaimPublisher>` — the production provider.
  Derives the same MAC *and* declares a matching claim to DCops before returning
  it, so the IP/DNS/L2 reservation follows the VM.

The provider is injected into `PlacementService`, which calls `reserve_mac`
before placement; the MAC rides on the `ExecuteVM` RPC (`mac_address` field) and
is honoured by aetherd's hypervisor config. It is also stored in the
`PlacementRecord`, so recovery replays it.

## 2. Declaring the claim to DCops

`DcopsNetworkIdentityProvider` renders a DCops **`NetBoxIPAddress`** custom
resource keyed on the MAC and publishes it through a swappable seam:

```rust
#[async_trait]
pub trait IpamClaimPublisher: Send + Sync {
    async fn publish(&self, claim: &NetBoxIpClaim) -> Result<(), String>;
}
```

`NetBoxIpClaim` renders a CR with `status: dhcp`, a `macAddress`, and an
`ipRange` reference but **no `address`** — Aether asserts the MAC and the pool,
DCops owns the actual IP. The resource name is a deterministic, DNS-1123-safe
`aether-<uuid>`, so republishing the same workload converges on the same object
(idempotent — recovery re-declares the same claim safely).

The default concrete publisher is GitOps-idiomatic: `ManifestClaimPublisher`
writes the CR as a YAML manifest into a git-backed directory for Flux/Argo to
reconcile (activated by `AETHER_IPAM_MANIFEST_DIR`, with tenant/range/namespace
from env). The seam means a live `kube` server-side-apply publisher can drop in
later without touching the provider.

```text
DcopsNetworkIdentityProvider.reserve_mac(uuid)
    │  mac = 52:54:00:xx:xx:xx      (deterministic)
    ▼
NetBoxIpClaim { macAddress: mac, status: dhcp, ipRange: <pool>, tenant: … }
    │  (no address — DCops/Kea assign it)
    ▼
IpamClaimPublisher.publish  ──► NetBoxIPAddress CR in the DCops cluster
    │
    ▼
return mac  ──► ExecuteVM(mac_address = mac)  ──► VM NIC uses this MAC
                                                     ──► Kea leases the IP by MAC
```

---

## 3. The DCops side: populated IP ranges

For the handoff to work, DCops must accept a MAC-keyed reservation inside a DHCP
pool without trying to allocate the IP itself. The relevant DCops behaviour (and
a bug fixed as part of this work) concerns **populated** IP ranges.

A DCops `NetBoxIPRange` with `markPopulated: true` tells NetBox the range is
externally managed (by a DHCP server such as Kea). NetBox then **prohibits
creating individual `IPAddress` objects inside that range** — by design.

The `NetBoxIPAddress` reconciler originally always tried to create the IP in
NetBox, so a claim landing in the DHCP pool got a `400`, went to
`Failed(netbox_id=0)`, and the drift-check machinery treated that as "recreate" —
an infinite loop. The fix (`DCops/controllers/netbox/.../ipam/ip_address.rs`):

- When resolving the `ipRange`, capture `mark_populated`.
- If the range is populated, **short-circuit before the drift check**: record the
  address in the CR status as terminally `Created` with **no** NetBox id
  (`create_populated_range_ip_status_patch`), emit an `ExternallyManaged` event,
  and return. No NetBox `IPAddress` object is created; Kea serves the lease by
  MAC. Idempotent, and it self-heals a CR already stuck in the old loop.

This is the repo's own recommended "Option 1" from
`DCops/docs/NETBOX_IP_RANGE_ANALYSIS.md`, now implemented and unit-tested.

> **The cross-repo contract, in one sentence:** Aether declares a
> `NetBoxIPAddress` with a MAC, `status: dhcp`, and an `ipRange` pointing at a
> `markPopulated` DHCP range and no `address`; DCops records it (no NetBox IP
> object) and lets Kea assign the lease keyed on that MAC; on recovery Aether
> replays the same MAC, so the same lease comes back.

---

## 4. Status & gaps

| Piece | Status |
| :--- | :--- |
| MAC model + `LocalMacAllocator` | ✅ Tested (stable, well-formed, idempotent) |
| `DcopsNetworkIdentityProvider` + claim | ✅ Tested (publishes claim, propagates failure, stable MAC) |
| `NetBoxIpClaim` manifest rendering | ✅ Tested (schema, quoted MAC, **no `address`** invariant) |
| `ManifestClaimPublisher` (GitOps) | ✅ Tested (deterministic, idempotent file) |
| Live `kube` apply publisher | ⛔ Deferred behind the `IpamClaimPublisher` seam |
| DCops populated-range reconciler fix | ✅ Implemented + unit-tested (skip create, track in status, no loop) |
| Guest IP discovery (read the leased IP back) | ⛔ Planned (EPIC-09.3 — DHCP snooping / guest agent) |

The remaining loose end is **read-back**: Aether declares the MAC and DCops/Kea
assign the IP, but Aether doesn't yet discover the guest's *actual* leased IP to
report it (EPIC-09.3). Everything up to the lease is wired.

Related: [HA, fencing & recovery](./impl_ha_recovery.md),
[storage node & provisioning](./impl_storage_node.md).
