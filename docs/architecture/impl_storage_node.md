# Storage Node: Disaggregated Volumes, Provisioning & Replication

Deep-dive companion to [ARCHITECTURE.md](../../ARCHITECTURE.md). Covers the
storage-node role: how Aether turns raw JBOD capacity into network-attached VM
disks, exports them over iSCSI, provisions them on demand, and replicates them to
a standby for HA.

## The disaggregated model

Aether does **not** put VM disks on the compute blade. VM disks are ZVOLs owned by
dedicated **storage nodes** (the iSCSI heads that front the JBOD) and exported as
iSCSI LUNs on the storage fabric (VLAN 11). Compute blades are iSCSI *initiators*
that log in and hand the LUN to the hypervisor. This separation is what lets a
fenced compute blade's VM restart elsewhere and re-attach the same disk.

```text
  ┌ storage node (POOL_STORAGE) ────────────────┐        ┌ compute blade ┐
  │  ZFS pool ──► ZVOL ──► iSCSI target/LUN  ────┼─VLAN11─┼─► initiator ─►│ VM
  │       │                                      │        └───────────────┘
  │       └► zfs send/recv ──► standby node      │
  └──────────────────────────────────────────────┘
```

The same `aetherd` binary runs the storage role, selected by `AETHER_ROLE=storage`
(`aetherd/src/main.rs::run_storage_node`).

---

## 1. iSCSI target export (`storage/iscsi_target.rs`)

The initiator side already existed (`storage/iscsi.rs`, `IscsiManager` —
discover/login/logout). The storage node adds the **target** side:

```rust
#[async_trait]
pub trait IscsiTargetManager: Send + Sync {
    async fn create_target(&self, iqn: &str) -> io::Result<()>;
    async fn export_lun(&self, iqn: &str, backing_device: &str) -> io::Result<u32>;
    async fn delete_target(&self, iqn: &str) -> io::Result<()>;
}
```

`MockIscsiTargetManager` is an in-memory implementation for tests;
`RealIscsiTargetManager` is the production path, shelling out to LIO via
`targetcli` (one block backstore + LUN per target, demo-mode ACLs) on a blocking
thread, mirroring the rest of the crate's shell-outs.

## 2. Provisioning orchestrator (`storage/node.rs`)

`StorageNode` composes a `ZvolManager` and an `IscsiTargetManager` behind this
node's fabric identity (`StorageNodeConfig { portal, iqn_prefix }`):

- `provision_volume(name, size_bytes) -> VolumeHandle` — carve the ZVOL, derive a
  per-volume IQN (`<prefix>:<sanitized-name>`), create the target, export the LUN,
  and return `VolumeHandle { name, device_path, iqn, iscsi_uri }`. If the export
  fails after the ZVOL is created, the ZVOL is rolled back so nothing is orphaned.
- `deprovision_volume(name)` — tear down the target, then destroy the ZVOL.

The returned `iscsi_uri` is `iscsi://<portal>/<iqn>` — **exactly** the shape the
aggregator places as a workload's `image_uri` and that the compute blade's
`execute_vm` already splits and logs into. That symmetry is what wires the two
roles together with no glue code in the middle.

## 3. The `AetherStorage` RPC (`storage_service.rs`, `proto/aether.proto`)

Provisioning is exposed over mTLS as a distinct gRPC service so it can be driven
by the aggregator (or an operator):

```proto
service AetherStorage {
    rpc ProvisionVolume(ProvisionVolumeRequest) returns (ProvisionVolumeResponse);
    rpc DeprovisionVolume(DeprovisionVolumeRequest) returns (DeprovisionVolumeResponse);
}
```

`AetherStorageImpl` validates the attestation token against the storage node's id,
calls the `StorageNode`, and maps the `VolumeHandle` (or a backend error) into the
response. It is served by `run_storage_node` alongside registration (§5).

## 4. Async ZVOL replication (EPIC-06.4)

Replication is what makes a VM's *disk* recoverable, not just its identity. It
lives on the storage node (which owns the ZVOLs) and is keyed to the **ZVOL
lifecycle**, not to VM launch.

- `ZfsReplicator` (`storage/replication.rs`) snapshots a ZVOL on a fixed cadence
  (default 5 min, the RPO bound) and ships the delta to a standby — **full** the
  first time, **incremental** (`zfs send -I`) thereafter — through a
  `SnapshotTransport` seam. `RealSnapshotTransport` pipes `zfs send | ssh … zfs
  recv`; the seam is where a purpose-built engine (`zrepl`, or array/DRBD-native
  replication over a dedicated fabric) drops in.
- Guarantees: **no overlap** (an in-flight run that overruns the interval is
  skipped, not stacked) and **RPO-safe** (the replicated baseline advances *only*
  after a transfer succeeds, so a failed run simply retries from the last good
  snapshot on the next tick — the chain never breaks).
- `ReplicatingZvolManager` is a `ZvolManager` decorator: wrapping the storage
  node's real manager with it starts replication on `create_zvol` and stops it on
  `destroy_zvol`. `StorageNode` stays replication-agnostic; wrapping its manager
  is the only wiring needed.

> **Design note.** An earlier iteration wrongly drove replication from the compute
> daemon's VM-launch path. That was corrected: the compute blade never owns the
> disk (`start_for_image` no-op'd on `iscsi://` images — the tell), so replication
> was relocated to the storage node's ZVOL lifecycle where it belongs.

## 5. Pool-aware registration & discovery

The storage node registers with the aggregator like any node, but as
`pool: "STORAGE"` (`run_storage_node` → `RegisterNode`), then heartbeats to keep
its lease (see [HA deep-dive](./impl_ha_recovery.md)). The registry
(`aggregator::registry`) grew pool-aware lookups and constants
(`POOL_COMPUTE`/`POOL_INFRA`/`POOL_STORAGE`):

- `schedulable_nodes()` — everything *but* STORAGE. The scheduler's auction uses
  this, so a storage node is never sent a bid request it can't answer.
- `pick_in_pool(POOL_STORAGE)` — deterministically (lowest node id) resolve a
  storage node for provisioning.
- Storage nodes are also excluded from the prune/fence loop (never STONITH'd).

**Aggregator-driven provisioning** (`recovery::PlacementService`). When a workload
is submitted without an `image_uri`, `PlacementService::place` provisions a disk
before dispatch. The `StorageProvisioner` seam abstracts this;
`GrpcStorageProvisioner` discovers a storage node from the registry
(`pick_in_pool`), mints a token for it, and calls `ProvisionVolume` — no hardcoded
endpoint, so storage nodes can come and go. If none is registered, it errors
cleanly ("no storage node is registered") before any network I/O.

---

## 6. End-to-end lifecycle

```text
diskless workload → PlacementService.place
    → GrpcStorageProvisioner (pick_in_pool STORAGE)
        → AetherStorage.ProvisionVolume on the storage node
            → StorageNode: create ZVOL → ReplicatingZvolManager starts replication
                          → export iSCSI LUN → return iscsi://portal/iqn
    → reserve stable MAC (NetworkIdentityProvider)
    → auction winner → ExecuteVM(image_uri = iscsi://…, mac_address = …)
        → compute blade logs in as initiator, boots the VM
    → DCops/Kea resolve the IP from the MAC
```

On node failure, recovery re-auctions the workload, replays the same MAC, and
points it at the replicated disk on the standby — the disaggregated model is what
makes that possible.

## 7. Status & gaps

| Piece | Status |
| :--- | :--- |
| `IscsiTargetManager` (mock + `targetcli` real) | 🟩 Mock tested; real is untested shell-out |
| `StorageNode` provision/deprovision + rollback | ✅ Tested |
| `AetherStorage` RPC + service impl | ✅ Tested (token, happy path, deprovision) |
| Storage-node role entrypoint | 🟩 Wired (`AETHER_ROLE=storage`), untestable glue |
| ZVOL replication engine (`ZfsReplicator`) | ✅ Tested (full/incremental, RPO, overlap) + RPO integration test |
| `ReplicatingZvolManager` decorator | ✅ Tested |
| Registry pool lookups + scheduler exclusion | ✅ Tested |
| `GrpcStorageProvisioner` registry discovery | 🟨 Discovery-miss tested; gRPC path is mock-node testable |
| Storage-node registration in registry | 🟩 Wired; no `NodeInfo` capacity/health beyond liveness |
| De-provision on VM teardown | ⛔ RPC exists; nothing calls it on teardown yet |
| Real replication engine (zrepl/array) | ⛔ Deferred behind the `SnapshotTransport` seam |

Related: [HA, fencing & recovery](./impl_ha_recovery.md),
[network-identity & DCops IPAM](./impl_network_identity.md),
[storage & migration](./impl_storage_migration.md).
