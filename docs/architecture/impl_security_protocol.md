# Security, Networking & Protocol Reference

> **Scope.** The security posture (mTLS + attestation), the Linux/HPE networking
> drivers, the full gRPC protocol surface, the concurrency/error model, and the
> hardware-mock server. Grounded in source with `file:line` references. Conceptual
> overview: [ARCHITECTURE.md](../../ARCHITECTURE.md) §1, §3, §4.

---

## 1. Transport security (mTLS)

There are **two independent TLS stacks**:

### 1a. Control plane — tonic transport (`aether-auth/src/mtls.rs`)

Used for all aggregator ↔ node gRPC. Backed by tonic (rustls under the hood).

```rust
pub fn create_server_tls_config(ca, server_cert, server_key) -> ServerTlsConfig      // :5
    // ServerTlsConfig::new().identity(id).client_ca_root(ca)  → client cert REQUIRED
pub fn create_client_tls_config(ca, client_cert, client_key, domain) -> ClientTlsConfig // :19
pub fn load_pem<P: AsRef<Path>>(path) -> io::Result<Vec<u8>>                          // :34
```

Mutual authentication is enforced server-side via `.client_ca_root(...)` — a peer
without a CA-signed client certificate is rejected.

### 1b. Data plane — hand-rolled rustls (`aetherd`)

Migration (`migration/socket.rs:44-83`) and VSOCK (`vsock.rs:35-77`) do **not** use
tonic; they build `rustls` `ServerConfig`/`ClientConfig` directly with
`WebPkiClientVerifier`, using `tokio-rustls` 0.26 + the `ring` provider (`tls12`
enabled). Cert/key are parsed with `rustls_pemfile`; empty path ⇒ plain TCP,
non-empty ⇒ mTLS.

### 1c. Certificate model — important correction

Cert generation (`mtls.rs:39-104`, `test_pki`) uses the **`rcgen`** crate with
`KeyPair::generate()` **and no algorithm argument**, so keys are rcgen's default —
**ECDSA P-256 (`PKCS_ECDSA_P256_SHA256`)**, *not* RSA-4096 and *not* ed25519 as
some older notes state. Details:

- CA is `IsCa::Ca(Unconstrained)`, CN "Aether Test CA".
- Server cert CN "localhost", SANs `DNS:localhost` + `IP:127.0.0.1`.
- Client cert CN "aetherd-client".
- **No `not_after` / expiry** is set on any certificate.

> **Deployment reality.** The aggregator `main.rs:25-52` **generates a fresh
> in-memory dev PKI at startup** (`test_pki::generate_test_creds()`) and wires it
> into `Server::builder().tls_config(...)`. There is **no production cert-file
> loading path** wired into `main` yet — so out of the box the cluster runs on
> ephemeral dev certs. Production PKI provisioning is an open task.

---

## 2. Attestation tokens (`aether-auth/src/token.rs`)

Ephemeral, single-use tokens gate every privileged RPC. Crypto is **HMAC-SHA256**
(symmetric), not asymmetric signatures.

```rust
pub struct TokenManager {
    secret: Vec<u8>,
    seen_signatures: parking_lot::Mutex<HashMap<String, u64>>,   // replay set
}
```

- **Generate** (`:27-38`): payload `"{node_id}:{now_secs}:{now_nanos}"`; token =
  `"{node_id}:{secs}:{nanos}:{hex(hmac)}"`. The nonce is `subsec_nanos()`.
- **Validate** (`:41-98`): split into exactly 4 fields; check `node_id` matches;
  **expiry 60 s** (`now > timestamp + 60` → expired, `:70`); recompute + compare
  HMAC; then **replay protection** — reject if the signature was already seen, else
  record it; finally GC entries older than 60 s.

Errors surfaced: `"Malformed token format"`, `"Token node_id mismatch"`,
`"Token expired or invalid timestamp"`, `"Replayed token detected"`.

### Flow

```
 aetherd                         aggregator
   │   RegisterNode ─────────────►│  generate_token(node_id)
   │◄──────── RegisterNodeResponse{ token }
   │   SendHeartbeat{ token } ───►│  validate_token → renew_heartbeat
   │
   │   (privileged RPCs carry the token; aetherd validates locally)
   │   ExecuteVM / TeardownVM / Prepare|StartMigration{ token }
```

The token is validated at every privileged handler
(`aetherd/src/lib.rs:115-117,284,335,357`) and re-validated as the first line of
the migration socket handshake (`socket.rs:87-114`). Dev shared secret in `main`
is a literal (`b"supersecretkeyfor…"`). There is no key-rotation API yet.

---

## 3. gRPC protocol reference

### `proto/aether.proto` (package `aether`, proto3)

| Service | RPC | Request → Response |
| :--- | :--- | :--- |
| `AetherAggregator` | `RegisterNode` | `RegisterNodeRequest` → `RegisterNodeResponse` |
| `AetherAggregator` | `SendHeartbeat` | `HeartbeatRequest` → `HeartbeatResponse` |
| `AetherNode` | `RequestReverseBid` | `BidRequest` → `BidResponse` |
| `AetherNode` | `ExecuteVM` | `ExecuteVMRequest` → `ExecuteVMResponse` |
| `AetherNode` | `TeardownVM` | `TeardownVMRequest` → `TeardownVMResponse` |
| `AetherNode` | `ListVMs` | `ListVMsRequest` → `ListVMsResponse` |
| `AetherNode` | `PrepareMigration` | `PrepareMigrationRequest` → `PrepareMigrationResponse` |
| `AetherNode` | `StartMigration` | `StartMigrationRequest` → `StartMigrationResponse` |

**Messages** (field : type = tag):

| Message | Fields |
| :--- | :--- |
| `RegisterNodeRequest` | `node_id`=1, `grpc_endpoint`=2, `pool`=3 ("COMPUTE"/"INFRA") |
| `RegisterNodeResponse` | `success bool`=1, `token`=2 |
| `HeartbeatRequest` | `node_id`=1, `token`=2 |
| `HeartbeatResponse` | `success bool`=1 |
| `BidRequest` | `workload_uuid`=1, `cpu_request i32`=2, `memory_request_bytes i64`=3, `disk_request_bytes i64`=4 |
| `BidResponse` | `node_id`=1, `score i32`=2 (−1 rejected, else 1–1000) |
| `ExecuteVMRequest` | `token`=1, `workload_uuid`=2, `name`=3, `cpu_limit i32`=4, `memory_limit_bytes i64`=5, `image_uri`=6 |
| `ExecuteVMResponse` | `success bool`=1, `ip_address`=2, `mac_address`=3, `error_message`=4 |
| `TeardownVMRequest` | `token`=1, `workload_uuid`=2 |
| `TeardownVMResponse` | `success bool`=1, `error_message`=2 |
| `ListVMsRequest` | *(empty)* |
| `VMDetails` | `uuid`=1, `name`=2, `state`=3, `ip_address`=4, `mac_address`=5 |
| `ListVMsResponse` | `repeated VMDetails vms`=1 |
| `PrepareMigrationRequest` | `token`=1, `workload_uuid`=2, `port u32`=3, `use_tls bool`=4 |
| `PrepareMigrationResponse` | `success bool`=1, `error_message`=2 |
| `StartMigrationRequest` | `token`=1, `workload_uuid`=2, `destination_ip`=3, `port u32`=4, `use_tls bool`=5, `max_bandwidth u64`=6 |
| `StartMigrationResponse` | `success bool`=1, `error_message`=2 |
| `TelemetryReport` | `node_id`, `load_one/five/fifteen double`, `mem_total/available u64`, `disk_total/available u64`, `nvme_temp double`, `cpu_cores u32` — **defined, no RPC** |
| `ControlRequest` | `command`, `target` — **defined, no handler** |

Rust bindings via `tonic::include_proto!("aether")` (`aether-auth/src/lib.rs:17-19`).

### `proto/csi.proto` (package `csi.v1`)

The verbatim upstream Container Storage Interface v1 spec. Aether implements
`Identity`, `Controller`, and `Node`; `GroupController` and `SnapshotMetadata` are
not implemented. Server lives in
[`aether-aggregator/src/storage/csi.rs`](./impl_storage_migration.md#4-csi-driver-aether-aggregatorsrcstoragecsirs).

---

## 4. Networking

### 4a. Tenant bridges on `aetherd` (`network/bridge.rs`)

```rust
#[async_trait]
pub trait BridgeManager: Send + Sync {                        // network/mod.rs:7-20
    async fn create_tenant_bridge(&self, vlan_id: u16) -> io::Result<String>;
    async fn create_tap_device(&self, tap: &str, bridge: &str) -> io::Result<()>;
    async fn apply_mac_anti_spoofing(&self, tap: &str, allowed_mac: &str) -> io::Result<()>;
    async fn teardown_tenant_network(&self, vlan_id: u16, tap: &str) -> io::Result<()>;
}
```

> **Correction.** `RealBridgeManager` does **not** shell out to `ip`, `brctl`,
> `ebtables`, or `iptables`. It uses **Rust netlink/nftables libraries directly**:
>
> - **`rtnetlink`** to create the bridge (`br-tenant-{vlan_id}`), create/attach the
>   TAP, and delete links.
> - **`tun-rs`** to create a persistent **L2 TAP** (`Layer::L2`, `.persist()`).
> - **`rustables`** (nftables) for MAC anti-spoofing: a bridge-family table
>   `aether-filter` with a per-TAP chain that drops any frame whose source MAC ≠
>   the allowed guest MAC.
>
> `RealBridgeManager` is `cfg(all(target_os = "linux", not(tarpaulin)))`, so it is
> excluded from coverage builds; tests exercise `MockBridgeManager`.

### 4b. HPE Virtual Connect client (`aether-aggregator/src/network/hpe_vc.rs`)

Protocol is **REST + JSON over HTTPS** (HPE OneView API) via `reqwest` — not SOAP.

```rust
#[async_trait]
pub trait MidplaneNetworkManager: Send + Sync {               // network/mod.rs:21-28
    async fn provision_vlan_interface(&self, slot: u8, vlan_id: u16) -> Result<(), NetworkError>;
    async fn teardown_vlan_interface(&self, slot: u8, vlan_id: u16) -> Result<(), NetworkError>;
}
```

`VirtualConnectClient` (`:49-60`) holds a `reqwest::Client`, endpoint/credentials,
`api_version = "600"`, a cached `session_token` (behind a `tokio::Mutex`), and
poll settings (default 5 s interval × 60 attempts). Key behaviors:

- `get_token` — POST `/rest/login-sessions` (`X-API-Version: 600`), cache `sessionID`.
- `send_request` — attach `auth` + version headers; on **401** clear the token and
  retry once (the token-refresh loop).
- `poll_task` — async `tokio::time::interval` poll of a task URI until
  `taskState == "Completed"` (or `Error/Failed/Terminated` → failure).
- `get_or_create_network(vlan_id)` — GET by filter, else POST a `Tagged` ethernet
  network.
- `provision_vlan_interface` — resolve/create network → GET server profile
  `profile-slot-{slot}` → set the FlexNIC-1a connection's `networkUri` → PUT; if
  `202 Accepted`, deserialize the `Task` and `poll_task`. `teardown` clears the
  `networkUri` symmetrically.

> **Correction.** Older notes call this `HpeVirtualConnectClient` with "OAuth2" and
> `configure_vlan()/verify_vlan()`. The real type is `VirtualConnectClient` with
> **session-token** auth and `provision_vlan_interface` / `teardown_vlan_interface`.

### 4c. VLANs (design intent)

`VLAN 10` control bus · `VLAN 11` storage fabric (iSCSI, jumbo frames MTU 9000) ·
`VLAN 999` OOB management · `VLAN 20+` tenant bridges. See
[ARCHITECTURE.md](../../ARCHITECTURE.md#storage--network-integration).

---

## 5. Hardware mock (`pact-mock-server`)

A standalone **axum** server emulating the **HPE OneView REST API**. Modules:
`lib.rs` (health, logging, auth middleware), `hpe_oneview.rs` (the OneView mock),
`bin/oneview.rs` (runnable binary, seeds slots 1–8, listens on `$PORT`/8080).

State (`AppState = Arc<tokio::RwLock<InnerState>>`): `sessions`, `networks`,
`server_profiles`, `tasks`. Login accepts only `admin`/`password` → mints
`session-token-{uuid}`; duplicate VLAN → 409; profile update → 202 + synthesized
`Completed` task.

> **Test nuance.** The aggregator's VC contract tests
> (`tests/switch_vlan_tagging_integration.rs`) do **not** use this crate — they use
> the **`pact_consumer`** crate's ephemeral `PactBuilder` mock, driving a real
> `VirtualConnectClient` through five scenarios (provision, teardown, auth failure,
> token-refresh loop, profile-not-found, task-failure). The standalone
> `pact-mock-server` is a separate dev/integration server (0 unit tests).

---

## 6. Concurrency & error model

- **Runtime:** `tokio` throughout; gRPC via `tonic` (`#[tonic::async_trait]`).
  Trait objects use `async_trait`.
- **Locks:** predominantly `tokio::sync::Mutex`/`RwLock` for async-held state —
  `active_vms: Arc<Mutex<…>>` (`aetherd/lib.rs:57`), `Arc<RwLock<NodeRegistry>>`
  (aggregator), migration `qmp_sockets`/`active_migrations` (`RwLock`), plus
  `oneshot` for migration shutdown. **Exception:** the token replay set uses
  `parking_lot::Mutex` (sync, short critical section).
- **Errors:** there is **no** `FencingError` and no crate-wide `Result` alias
  (the `aether-fence` crate is effectively empty). The one custom error enum is
  `NetworkError` (aggregator, `thiserror`): `Authentication`, `Http(#[from]
  reqwest::Error)`, `Api { code, message }`, `NotFound`, `Json(#[from]
  serde_json::Error)`, `Other`. Elsewhere errors are `io::Result<T>` (aetherd
  networking) or plain `Result<_, String>` (auth, migration helpers).
- **gRPC boundary:** internal errors map to `tonic::Status` at the handler —
  `Status::unauthenticated` (token failures), `Status::internal` (spawn/build),
  `Status::invalid_argument`, `Status::not_found`, `Status::unimplemented` (CSI).
  `NetworkError` is not converted to `Status` (VC path is internal-only today).

---

## 7. Maturity

| Area | State |
| :--- | :--- |
| mTLS (tonic) + attestation tokens | **Well tested** — token suite (lifecycle, replay, expiry, tamper) + `tests/mtls_integration_tests.rs`. |
| Migration data-plane mTLS + HMAC | **Tested** — most complete data-plane story. |
| Proto contracts | Stable; `TelemetryReport` / `ControlRequest` defined but unwired. |
| Tenant bridge (`RealBridgeManager`) | Real netlink/nftables code, **0 unit tests** (coverage-excluded cfg); only the mock is exercised. |
| HPE VC client | Covered by `pact_consumer` contract tests (HTTP/JSON layer), not real hardware. |
| Production PKI provisioning | **Not implemented** — dev PKI generated at startup. |
| Key rotation | **Not implemented**. |

> **Audit caveat.** `docs/EPICS/audit_dev_1to5.md` is **stale** on two current-code
> points: it claims "no nftables rules" for MAC anti-spoofing (contradicted by the
> `rustables` implementation) and describes the VC client as
> `HpeVirtualConnectClient`/OAuth2 (the code is `VirtualConnectClient`/session
> token). Trust the source over the audit on these.
