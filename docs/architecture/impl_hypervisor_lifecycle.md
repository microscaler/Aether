# Hypervisor & VM Boot Lifecycle — Implementation Reference

> **Scope.** The dual-hypervisor engine inside `aetherd`: the common `Hypervisor`
> trait, the Firecracker and QEMU drivers, the QMP client, NoCloud Cloud-Init ISO
> generation, the VSOCK channel, and the end-to-end VM lifecycle behind
> `ExecuteVM` / `TeardownVM` / `ListVMs`. Grounded in source with `file:line`
> references. Conceptual overview in [ARCHITECTURE.md](../../ARCHITECTURE.md) §1.

**Files:** `crates/aetherd/src/hypervisor/{mod,firecracker,qemu}.rs`,
`crates/aetherd/src/cloud_init.rs`, `crates/aetherd/src/vsock.rs`,
`crates/aetherd/src/lib.rs`.

---

## 1. The `Hypervisor` trait

`crates/aetherd/src/hypervisor/mod.rs:12-25`. A single async trait abstracts both
backends; the module forbids `unwrap`/`expect`/`panic` via JSF lints.

```rust
#[async_trait]
pub trait Hypervisor: Send + Sync {
    async fn spawn(&self) -> Result<(), String>;
    async fn stop(&self) -> Result<(), String>;
    async fn query_status(&self) -> Result<String, String>;   // "RUNNING" | "STOPPED" | "PAUSED"
    fn get_qmp_socket_path(&self) -> Option<String>;
}
```

Drivers are stored type-erased as `Box<dyn Hypervisor>` inside `ActiveVm`
(`lib.rs:37`). `get_qmp_socket_path` is the migration hook: Firecracker returns
`None` (no QMP), QEMU returns `Some(qmp_socket_path)`.

### Backend selection

Selection is **not** driven by a CRD "profile" field today — it is a simple
threshold on the requested vCPU count (`lib.rs:132`):

```rust
if req.cpu_limit < 4 { /* Firecracker microVM */ } else { /* QEMU-KVM full VM */ }
```

This approximates the Compute-pool / Infra-pool split but is decided per request,
not per blade role.

---

## 2. Firecracker driver

`crates/aetherd/src/hypervisor/firecracker.rs`.

### Configuration model

Config is expressed as serde structs that serialize to Firecracker's JSON schema
(field names renamed to match, e.g. `boot_source` → `"boot-source"`,
`machine_config` → `"machine-config"`):

```rust
struct BootSource       { kernel_image_path, boot_args }                 // :14-20
struct Drive            { drive_id, path_on_host, is_root_device, is_read_only } // :23-33
struct MachineConfig    { vcpu_count: u32, mem_size_mib: u32, smt: Option<bool> }// :36-45
struct NetworkInterface { iface_id, host_dev_name }                      // :48-54  (TAP name)
struct FirecrackerConfig{ boot_source, machine_config, drives, network_interfaces } // :57-70
struct JailerConfig     { uid, gid, chroot_base_dir, node_index: Option<u32> }  // :73-84
```

> **Implementation reality.** The driver configures Firecracker via a **JSON
> config file** (`--config-file`), *not* the HTTP API socket. There is currently
> **no MMDS and no vsock** wiring inside this driver — those config fields are not
> present. (VSOCK exists as a separate module; see §5.)

### Process launch

`spawn` (`:131-214`) serializes the config to `config_path`, opens the log file,
redirects stdout+stderr to it, spawns via `tokio::process::Command`, and writes a
**PID file** (`config_path` with `.json`→`.pid`). Two launch modes:

- **Direct** (default): `firecracker --config-file <path>`.
- **Jailer** (when `jailer_config` is set): runs the `jailer` binary with
  `--id --exec-file --uid --gid --chroot-base-dir -- --config-file <path>`
  (`:160-174`). `--node` is appended only if `node_index` is set (it was removed
  from newer jailer releases). A test comment notes jailer **v1.10.0** requires
  the exec-file basename to be `firecracker` (`:386-387`) — the only version
  reference in the tree.

`stop` (`:216-252`): `SIGTERM`, poll up to 500 ms for `STOPPED`, then `SIGKILL`,
remove PID file. `query_status` (`:254-283`) probes liveness with `kill(pid, None)`.

---

## 3. QEMU driver

`crates/aetherd/src/hypervisor/qemu.rs`.

```rust
struct QemuConfig { vcpu_count, mem_size_mib, disk_image_path, qmp_socket_path,
                    host_tap_device: Option<String> }   // :17-29
```

### Command-line assembly (`spawn`, `:340-408`)

The driver compiles this argument vector:

```
-enable-kvm  -cpu host  -m <mem_mib>  -smp <vcpus>
-drive file=<disk_image_path>,format=raw,media=disk
-qmp unix:<qmp_socket_path>,server,nowait
# if host_tap_device is Some(tap):
-netdev tap,id=net0,ifname=<tap>,script=no,downscript=no
-device virtio-net-pci,netdev=net0
-nographic
```

The VM boots from the raw disk image; no kernel/boot-args are passed. PID is
written to `log_path` with `.log`→`.pid`.

### QMP client

A full line-oriented `QmpClient` (`:78-336`) speaks the QEMU Monitor Protocol over
the Unix socket. `connect_and_negotiate` reads the greeting and issues
`qmp_capabilities`. Implemented commands include `query-status`, `migrate`,
`query-migrate`, `migrate-set-capabilities`, `migrate-set-parameters`
(`max-bandwidth`), `nbd-server-start` / `nbd-server-add` (default NBD port
**10809**), `drive-mirror` (`sync:"full"`, `mode:"existing"`),
`block-job-complete`, `query-block-jobs`, and `migrate-cancel`. These back the
[live-migration subsystem](./impl_storage_migration.md).

`query_status` (`:448-477`) checks PID liveness first, then queries QMP, falling
back to `"RUNNING"` if the QMP query fails but the process is alive.

> **Stub.** `cleanup_host_resources` (`:70-74`) is currently a `println!`
> placeholder — real bridge/ZVOL teardown is a TODO.

---

## 4. Cloud-Init (NoCloud ISO)

`crates/aetherd/src/cloud_init.rs`.

```rust
struct CloudInitConfig { instance_id, hostname, user_data }   // YAML cloud-config
struct CloudInitIso    { _temp_dir: TempDir, iso_path: PathBuf }
```

`build_iso` (`:58-172`):

- **RAM-backed staging** — if `/dev/shm` exists (Linux), the temp dir is created
  there (`aether-cloudinit-` prefix) so nothing hits disk; otherwise a normal temp
  dir (macOS dev). The `TempDir` is dropped with the handle, cleaning RAM.
- Writes exactly two files into `input/`: `user-data` (verbatim) and `meta-data`
  (`instance-id: …\nlocal-hostname: …\n`). **No `network-config` file is written.**
- Builds the ISO with **`xorriso`** (`-as mkisofs -R -V config-2 -o seed.iso input/`)
  or **`mkisofs`** as fallback; volume label is `config-2` in both. (`genisoimage`
  is not used.)
- **Mock path**: if neither tool is present, it writes the bytes
  `mock_iso_content` — a dev stub, not a real ISO.

> **Implementation reality / gap.** The ISO is built and its handle is retained in
> `ActiveVm._iso`, but in `execute_vm` the ISO is **not attached as a drive** to
> either hypervisor — `_iso_path` is computed and then unused (`lib.rs:129`). So
> Cloud-Init generation works and is tested, but the seed drive is not yet wired
> into the boot. This is the single most important lifecycle gap to close.

---

## 5. VSOCK channel

`crates/aetherd/src/vsock.rs`. Despite the name, this is a **Unix-domain-socket
multiplexer** (Firecracker-style vsock-over-UDS), not raw `AF_VSOCK`; there is no
CID handling. `VsockConnector::connect_to_guest(port)` writes `CONNECT <port>\n`
and expects an `OK` line; `connect_to_guest_secure` upgrades that raw channel to
mTLS via `tokio_rustls`. Helper functions build rustls client/server configs from
CA/cert/key PEM using the `ring` provider and WebPKI client-cert verification.
There is no guest-agent protocol beyond the CONNECT/OK handshake — callers get a
raw byte stream.

---

## 6. VM lifecycle

VMs are tracked in-memory:
`AetherNodeImpl.active_vms: Arc<Mutex<HashMap<String, ActiveVm>>>` keyed by
`workload_uuid` (`lib.rs:57`), where
`ActiveVm { details: VmDetails, hypervisor: Box<dyn Hypervisor>, iscsi_iqn: Option<String>, _iso: CloudInitIso }`.

### `execute_vm` (`lib.rs:110-275`)

1. Validate the attestation token → `Status::unauthenticated` on failure.
2. Build the Cloud-Init ISO in `/dev/shm` (fixed `user_data = "#cloud-config\n"`).
3. Select backend by `cpu_limit < 4`.
4. **Firecracker branch** — hardcoded kernel `/var/lib/aether/vmlinux`, boot args
   `console=ttyS0 reboot=k panic=1 pci=off`, single root drive from `image_uri`,
   `mem_size_mib = memory_limit_bytes / 1MiB`, `smt=false`, no NIC. Binary is
   `/usr/bin/firecracker` if present, else `sleep 1000` (dev fallback).
5. **QEMU branch** — if `image_uri` starts with `iscsi://`, parse `portal/iqn` and
   `login_target` first; QMP socket at `$TMP/qmp-<uuid>.sock`; binary
   `/usr/bin/qemu-system-x86_64` or `sleep` fallback.
6. `hypervisor.spawn()` → `Status::internal` on error.
7. If `get_qmp_socket_path()` is `Some`, register the VM with the migration manager.
8. Build `VmDetails` (currently **hardcoded** `state="RUNNING"`,
   `ip="192.168.1.100"`, `mac="52:54:00:12:34:56"`), insert into `active_vms`,
   return success.

### `teardown_vm` (`:277-311`)

Validate token → remove from map → `hypervisor.stop()` → unregister migration →
iSCSI logout if applicable. Unknown UUID → `Status::not_found`.

### `list_v_ms` (`:313-326`)

Iterate the map, call `query_status()` on each hypervisor, **update
`details.state` live** from the result, and return the collected `VmDetails`.

```
 ExecuteVM ──► build ISO (/dev/shm) ──► select backend (cpu<4?) ──► spawn ──►
              register w/ migration mgr (if QMP) ──► ACTIVE
 ListVMs   ──► query_status() per VM ──► refresh state
 TeardownVM ─► stop() ──► iSCSI logout ──► unregister ──► removed
```

---

## 7. Maturity

| Area | State |
| :--- | :--- |
| Firecracker arg/PID lifecycle | **Tested** — 6 unit tests + `tests/firecracker_vm_boot_lifecycle.rs` (mock via `sleep`, asserts spawn < 100 ms against the mock). |
| QEMU driver | Config test only; the assembled arg vector and the QMP command methods are **not** unit-asserted. Integration `tests/qemu_kvm_vm_lifecycle.rs` runs against a **mock QMP listener**, not real QEMU. |
| Cloud-Init ISO | **Tested** — ~11 tests incl. tmpfs behavior and `config-2` volume label; but the ISO is not mounted into the VM (see §4). |
| VSOCK | **Tested** — handshake success/failure + mTLS round-trip, plus `tests/vsock_stream_performance.rs` (≥100 MB/s over 5 MB). |
| gRPC handlers | **Tested** — `tests/node_tests.rs` runs execute → list → teardown with token-tamper rejection paths. |
| Real boot / memory-overhead / networking | **Not** verified — process mocks use `sleep`; IP/MAC/kernel path are hardcoded; no IPAM. |

> **Audit caveat.** `docs/EPICS/audit_dev_1to5.md` rates Epic 3 at ~40 % and is
> partly **stale**: it references symbols that no longer exist
> (`build_firecracker_config()`, `VsockServer`, `build_qemu_command()`) and claims
> "zero VSOCK tests" / "no tmpfs" — both contradicted by current code. Treat the
> audit as directional on *structure*, not authoritative on symbol names or test
> presence.
