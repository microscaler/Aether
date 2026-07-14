# Storage & Live Migration — Implementation Reference

> **Scope.** The ZFS/ZVOL volume manager and iSCSI initiator on `aetherd`, the
> aggregator's CSI driver, and the live-migration state machine (socket
> handshake + attestation, memory pre-copy, NBD block mirror, auto-converge).
> Grounded in source with `file:line` references. Companion design spec:
> [live_migration.md](./live_migration.md). Conceptual overview:
> [ARCHITECTURE.md](../../ARCHITECTURE.md) §3, §6.

## File inventory & verified sizes

| File | Lines | Inline tests |
| :--- | ---: | :--- |
| `crates/aetherd/src/storage/mod.rs` | 38 | — |
| `crates/aetherd/src/storage/zfs.rs` | 346 | **0** |
| `crates/aetherd/src/storage/iscsi.rs` | 530 | 11 |
| `crates/aether-aggregator/src/storage/csi.rs` | 1468 | 39 |
| `crates/aetherd/src/migration/mod.rs` | 300 | — |
| `crates/aetherd/src/migration/socket.rs` | 611 | 13 |
| `crates/aetherd/src/migration/memory.rs` | 59 | **0** |
| `crates/aetherd/src/migration/block.rs` | 49 | **0** |
| `crates/aetherd/src/migration/converge.rs` | 30 | **0** |

---

## 1. Storage traits

Two async traits, both in `aetherd` (`storage/mod.rs`, `storage/iscsi.rs`). There
is **no** generic `VolumeManager` — the abstraction is `ZvolManager`.

```rust
#[async_trait]
pub trait ZvolManager: Send + Sync {                          // storage/mod.rs:13-37
    async fn create_zvol(&self, name: &str, size_bytes: u64) -> io::Result<String>;
    async fn create_snapshot(&self, zvol: &str, snap: &str) -> io::Result<()>;
    async fn clone_zvol(&self, snap: &str, clone: &str) -> io::Result<String>;
    async fn rollback_zvol(&self, zvol: &str, snap: &str) -> io::Result<()>;
    async fn resize_zvol(&self, zvol: &str, new_size_bytes: u64) -> io::Result<()>;
    async fn destroy_zvol(&self, name: &str) -> io::Result<()>;
    async fn configure_arc_cache_limit(&self) -> io::Result<()>;
}

#[async_trait]
pub trait IscsiManager: Send + Sync {                         // iscsi.rs:17-31
    async fn discover_targets(&self, portal_ip: &str) -> io::Result<Vec<String>>;
    async fn login_target(&self, portal_ip: &str, iqn: &str) -> io::Result<String>;
    async fn logout_target(&self, iqn: &str) -> io::Result<()>;
    async fn rescan_session(&self, iqn: &str) -> io::Result<()>;
}
```

Each has a `Real*` implementation (Linux, `cfg(not(tarpaulin))`) and a `Mock*`
implementation used in tests.

---

## 2. ZFS / ZVOL (`storage/zfs.rs`)

`RealZvolManager { pool: String }` uses the **native `zfs_core` / `nvpair`
bindings** (not shelling out) for most operations, gated to
`cfg(all(target_os = "linux", not(tarpaulin)))`. Naming conventions:

- Dataset: `{pool}/{name}` · Device: `/dev/zvol/{pool}/{name}` · Snapshot: `{pool}/{zvol}@{snap}`.

| Method | Mechanism |
| :--- | :--- |
| `create_zvol` | nvlist with `volsize = size_bytes`, `z.create(path, Zvol, props)`. **No compression / thin-provision flags are set** — only `volsize`. |
| `create_snapshot` | `z.snapshot([...])`. |
| `clone_zvol` | `z.clone_dataset(clone, origin, {})` — plain ZFS clone (inherently CoW; "thin clone" in docs = CoW). |
| `rollback_zvol` | `z.rollback_to(fsname, snap)`. |
| `resize_zvol` | **Shells out**: `zfs set volsize=<n> <dataset>`. |
| `destroy_zvol` | `z.destroy(path)`. |
| `configure_arc_cache_limit` | Reads `/proc/meminfo`, caps ARC at **15 % of MemTotal**, writes `/sys/module/zfs/parameters/zfs_arc_max`, persists to `/etc/modprobe.d/zfs.conf`, runs `update-initramfs -u`. Failures are warnings, not fatal. |

Blocking library calls are wrapped in `tokio::task::spawn_blocking`.

> **Maturity.** Real bindings, **zero tests**, Linux-only compile gate. The
> intended production knobs (lz4 compression, explicit thin-provision) described in
> the [README StorageClass spec](../../ARCHITECTURE.md#storage--network-integration)
> are **not yet set** in `create_zvol`.

---

## 3. iSCSI (`storage/iscsi.rs`) — initiator only

`RealIscsiManager` shells out to `iscsiadm`. This file implements the **initiator**
side (a Compute blade logging into a remote target). **Target/LIO export
(`targetcli`, portal/IQN creation) is not implemented anywhere** — that half of the
network-attached-storage story is still to come.

| Method | Command |
| :--- | :--- |
| `discover_targets` | `iscsiadm -m discovery -t st -p <portal>`; IQN = column 2. |
| `login_target` | `iscsiadm -m node -T <iqn> -p <portal> --login`; exit code 15 (session exists) treated as success; then **polls sysfs** for the block device. |
| `logout_target` | `iscsiadm -m node -T <iqn> --logout`. |
| `rescan_session` | `iscsiadm -m node -T <iqn> --rescan`. |

Block-device discovery (`find_iscsi_block_device`, `:96-118`) walks
`<sysfs>/class/iscsi_session/*/targetname`, matches the IQN, and resolves
`.../block/<sdX>` → `/dev/<sdX>`. Login polling: **10 retries × 200 ms = 2000 ms**
max. `sysfs_root` is injectable for tests.

> **Stale-audit note.** An earlier audit flagged a "hardcoded placeholder path" and
> "zero tests." Both are now **out of date**: the real sysfs walk replaced the
> placeholder and there are 11 tests. Do not repeat those claims.

---

## 4. CSI driver (`aether-aggregator/src/storage/csi.rs`)

The largest file in the tree (1468 lines). Implements the CSI v1 **Identity**,
**Controller**, and **Node** services against **in-memory state** — a
high-fidelity mock that stages/publishes files (block) or directories (mount)
rather than driving live ZVOL/iSCSI. `GroupController` and `SnapshotMetadata` are
not implemented.

```rust
pub struct AetherCsiDriver {
    pub node_id: String,
    pub volumes: Arc<RwLock<HashMap<String, VolumeState>>>,  // volume_id → state
    pub name_to_id: Arc<RwLock<HashMap<String, String>>>,    // name → volume_id
}
```

**Identity:** `get_plugin_info` (`"aether-csi-driver"` v`0.1.0`),
`get_plugin_capabilities` (`CONTROLLER_SERVICE`), `probe`.

**Controller:**

| RPC | Behavior |
| :--- | :--- |
| `create_volume` | Validate name; idempotent via `name_to_id` (smaller existing → `already_exists`); size = required→limit→**default 10 GiB**; `volume_id = "vol-{uuid}"`. |
| `delete_volume` | Remove from both maps; missing id = silent success. |
| `controller_publish_volume` | Record node; return `publish_context["device_path"] = "/dev/zvol/tank/{volume_id}"` (**hardcoded pool `tank`**). |
| `controller_unpublish_volume` | Remove node from the set. |
| `validate_volume_capabilities` | Echo caps as `Confirmed` if the volume exists. |
| `list_volumes` | Return all (no pagination). |
| `get_capacity` | Hardcoded **1 TB**. |
| `controller_get_capabilities` | `CREATE_DELETE_VOLUME`, `PUBLISH_UNPUBLISH_VOLUME`. |
| snapshots / expand / modify / get | `Status::unimplemented`. |

**Node:** `node_stage_volume` / `node_publish_volume` create a file (block cap) or
directory (mount cap) at the staging/target path; `node_unstage` / `node_unpublish`
remove them. `node_get_info` reports `max_volumes_per_node: 100`. Stats / expand →
`unimplemented`.

> **Maturity.** 39 tests cover error paths, idempotency, default capacity, block
> staging, and publish context. Still an in-memory mock; the production path
> (ZVOL cut on the storage blade, exported via iSCSI, mapped on the compute blade —
> see [ARCHITECTURE.md](../../ARCHITECTURE.md) §3) is not yet wired end-to-end.

---

## 5. Live migration state machine

`crates/aetherd/src/migration/`. Types (`mod.rs:17-48`):

```rust
pub enum MigrationState { Idle, Preparing, Listening, Active, Completed, Failed(String), Cancelled }
pub struct MigrationParams { destination_node, destination_ip, port, use_tls, max_bandwidth }
```

`RealMigrationManager` holds the QMP socket map, the three mTLS PEM paths, an
`attestation_secret` (dev default `b"aether-migration-secret"`), and an
`active_migrations: RwLock<HashSet<String>>` (the set whose length feeds the bid
penalty in [bidding](./impl_bidding_scheduling.md#33-multiplicative-penalties)).

### Source side — `start_migration` (`mod.rs:140-187`)

1. `migrate-set-parameters` with `max-bandwidth` (if > 0).
2. `ConvergenceManager::enable_auto_converge()`.
3. `BlockReplicator::start_mirroring("drive-root", "nbd:{dest_ip}:{port}")` — block
   device id is **hardcoded `"drive-root"`** (production should enumerate real
   devices).
4. `MemoryMigrator::start_migration(uri)`.

On failure it rolls back (disable auto-converge, complete/cancel the block job).

### Destination side — `prepare_incoming` (`mod.rs:189-233`)

Build a `MigrationSocketManager`; if `use_tls`, require all three cert paths and
`listen_for_incoming_tls`, else `listen_for_incoming`; then
`BlockReplicator::prepare_destination("drive-root", "127.0.0.1:{port}")`.

### Socket handshake, mTLS & attestation (`socket.rs`)

- **Framing:** the handshake is **line-oriented** — the first line read off the
  stream is the attestation token (`read_and_validate_token`, `:87-114`). The bulk
  data transfer itself is delegated to QEMU's migration stream.
- **mTLS:** `build_mtls_config` (`:44-83`) builds a `rustls::ServerConfig` with a
  `WebPkiClientVerifier` (client cert **required**) from the CA PEM.
  `listen_for_incoming_tls` wraps a `TcpListener` in a `TlsAcceptor`; the token is
  validated *after* the TLS handshake.
- **Attestation:** delegated to `aether_auth::token::TokenManager` — **HMAC-SHA256**,
  token `node_id:timestamp:nonce:signature`, **60 s** expiry, single-use replay
  protection. See [Security & Protocol Reference](./impl_security_protocol.md#2-attestation-tokens).
- The NBD destination is pinned to `127.0.0.1` (not `0.0.0.0`); shutdown via a
  `oneshot` channel.

### Memory pre-copy (`memory.rs`, 59 lines)

Thin QMP wrapper: `migrate(uri)` then poll `query-migrate` every **500 ms** until
`completed` / `failed` / `cancelled`. **No max timeout and no rollback.** Zero tests.

### NBD block mirror (`block.rs`, 49 lines)

Thin wrapper: destination runs `nbd-server-start` + `nbd-server-add` (writable);
source runs `drive-mirror` (`sync:"full"`, `mode:"existing"`). Default NBD port
**10809**. Zero tests.

### Auto-converge (`converge.rs`, 30 lines)

The **entire** logic toggles one QEMU capability:

```rust
qmp.set_migration_capability("auto-converge", true).await
```

> **Accuracy correction.** There is **no custom throttle logic** — no
> `cpu-throttle-initial` / `-increment`, no penalty-percentage gradient, no
> convergence-threshold constants. Aether relies entirely on QEMU's native
> auto-converge. `migrate-set-parameters` sets **only** `max-bandwidth`. Any
> "gradual 10 %→99 % throttle" language in older notes is aspirational, not
> implemented.

### Migration proto (`aether.proto`)

```proto
PrepareMigrationRequest { token, workload_uuid, port: uint32, use_tls: bool }
StartMigrationRequest   { token, workload_uuid, destination_ip, port: uint32,
                          use_tls: bool, max_bandwidth: uint64 }
```

Both return `{ success: bool, error_message }` and are served on `AetherNode`.

---

## 6. Constants worth knowing

| Constant | Value | Source |
| :--- | :--- | :--- |
| Attestation token expiry | 60 s | `token.rs:70` |
| Memory-migration poll interval | 500 ms | `memory.rs:56` |
| iSCSI login poll | 10 × 200 ms = 2000 ms | `iscsi.rs:167-168` |
| NBD default port | 10809 | `qemu.rs:252` |
| CSI default volume size | 10 GiB | `csi.rs:157,160` |
| CSI reported capacity | 1 TB | `csi.rs:334` |
| CSI max volumes/node | 100 | `csi.rs:649` |
| ZFS ARC cap | 15 % of MemTotal | `zfs.rs:151` |
| Migration block device id | `"drive-root"` (hardcoded) | `mod.rs:165,229` |
| CSI publish device path | `/dev/zvol/tank/{id}` (hardcoded pool) | `csi.rs:239` |

---

## 7. Maturity summary

| Component | State |
| :--- | :--- |
| Migration socket + mTLS + HMAC attestation | **Most complete** — 13 unit + `tests/migration_tests.rs` (22 tests, mock QMP). |
| CSI mock driver | **Tested** — 39 tests; in-memory only. |
| iSCSI initiator (sysfs walk) | **Tested** — 11 tests. |
| ZFS `RealZvolManager` | Real bindings, **0 tests**, no compression/thin flags. |
| `memory.rs` / `block.rs` / `converge.rs` | Thin QMP wrappers, **0 tests**, no timeouts/rollback. |
| iSCSI/LIO **target export** | **Not implemented**. |
| CSI snapshots / expand / group services | **Not implemented** (`unimplemented`). |
| Custom auto-converge throttle | **Not implemented** (native QEMU flag only). |
