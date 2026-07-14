# Reverse-Bidding & Scheduling — Implementation Reference

> **Scope.** This document describes the *as-built* implementation of Aether's
> decentralized scheduler: the local bid calculation on each `aetherd` blade, the
> aggregator's broadcast/convergence loop, the deterministic tie-breaker, and the
> node registry / heartbeat state machine. It is grounded in the source tree and
> cites `file:line` throughout so it can be re-verified. For the conceptual
> framing, see [ARCHITECTURE.md](../../ARCHITECTURE.md) §2.

**Crates involved**

| Concern | Location |
| :--- | :--- |
| Local telemetry collection | `crates/aetherd/src/telemetry.rs` |
| Local bid scoring | `crates/aetherd/src/bidder.rs` |
| Broadcast / convergence | `crates/aether-aggregator/src/scheduler.rs` |
| Node registry & heartbeat | `crates/aether-aggregator/src/registry.rs` |
| Tie-breaker | `crates/aether-aggregator/src/tie_breaker.rs` |
| Wire types | `proto/aether.proto` |

---

## 1. Topology recap

Coordination is a **star**, not a mesh. Each `aetherd` holds a single mTLS gRPC
channel to the aggregator; there is no Raft, gossip, or quorum between blades.
The aggregator fans a `RequestReverseBid` out to every registered node, each node
scores *its own* local telemetry, and the aggregator selects the winner. The only
blade-to-blade traffic in the whole system is live migration (a distinct path —
see [Storage & Live Migration](./impl_storage_migration.md)).

```
              RegisterNode / SendHeartbeat  (aetherd → aggregator)
   ┌────────────────────────────────────────────────────────────────┐
   │                          Aether Aggregator                       │
   │            NodeRegistry (Arc<RwLock<…>>) · Scheduler              │
   └───────┬───────────────┬───────────────┬───────────────┬──────────┘
    Bid ▲  │ Execute     ▲  │            ▲  │            ▲  │   (per-node
        │  ▼             │  ▼            │  ▼            │  ▼    gRPC channel)
     ┌──────┐         ┌──────┐        ┌──────┐        ┌──────┐
     │aetherd│        │aetherd│       │aetherd│       │aetherd│
     │ bid() │        │ bid() │       │ bid() │       │ bid() │
     └──────┘         └──────┘        └──────┘        └──────┘
```

---

## 2. Telemetry inputs

`crates/aetherd/src/telemetry.rs`. The collector reads four host sources; each has
a defined fallback so a read failure degrades to a conservative mock value rather
than crashing the daemon.

| Signal | Source (default) | Parsed into | Fallback |
| :--- | :--- | :--- | :--- |
| CPU load | `/proc/loadavg` (`:22`) | `load_one/five/fifteen: f64` | `0.5` each |
| Memory | `/proc/meminfo` (`:23`) | `mem_total`, `mem_available: u64` (kB×1024 → bytes) | 16 GB / 8 GB |
| Disk | `statvfs` on `/` (`:25,147`) | `disk_total`, `disk_available: u64` | 100 GB / 50 GB |
| NVMe temp | `/sys/class/nvme/.../temp1_input` (`:24`) | `nvme_temp: f64` (m°C ÷ 1000) | `nvme smart-log`, then `35.0 °C` |
| CPU cores | `available_parallelism()` (`:235`) | `cpu_cores: u32` | `4` |

`MemAvailable` falls back to `MemFree` when absent (`:137`). The consolidated
struct passed to the bidder:

```rust
// telemetry.rs:31-53
pub struct SystemMetrics {
    pub load_one: f64, pub load_five: f64, pub load_fifteen: f64,
    pub mem_total: u64, pub mem_available: u64,
    pub disk_total: u64, pub disk_available: u64,
    pub nvme_temp: f64, pub cpu_cores: u32, pub active_migrations: u32,
}
```

`active_migrations` is **not** measured here — it is supplied by the caller. At
request time the node handler pulls the live count from the migration manager and
threads it in:

```rust
// crates/aetherd/src/lib.rs:94-102 (paraphrased)
let migration_count = migration_manager.get_active_migration_count();
let metrics = telemetry_collector.collect(migration_count);
let score = bidder.calculate_bid(&metrics, cpu_request, mem_bytes, disk_bytes);
```

> **Note.** The `TelemetryReport` message in `aether.proto:91-102` mirrors these
> fields but omits `active_migrations` and is **not** the type used for bids — it
> is presently defined without an RPC. Bids travel as `BidResponse`.

---

## 3. Bid scoring algorithm

`crates/aetherd/src/bidder.rs`, `Bidder::calculate_bid` (`:40-109`).

```rust
pub fn calculate_bid(&self, metrics: &SystemMetrics, cpu_request: i32,
    memory_request_bytes: i64, disk_request_bytes: i64) -> i32
```

Config defaults (`BidderConfig`, `:9-24`): `max_cpu_load = 2.0`, `critical_temp = 80.0 °C`.

### 3.1 Rejection — return `-1`

A node removes itself from the auction (returns `-1`) if **any** hold:

1. Malformed request: `cpu_request <= 0`, or negative memory/disk request (`:48`).
2. `memory_request_bytes > mem_available` (`:53`).
3. `disk_request_bytes > disk_available` (`:56`).
4. `cpu_cores == 0` (`:61`).
5. `cpu_load_ratio > max_cpu_load`, where `cpu_load_ratio = load_one / cpu_cores` (`:64-66`).
6. `nvme_temp >= critical_temp` (i.e. ≥ 80 °C) (`:70`).

Condition 6 is the SLA/thermal self-defense the epic calls for: an overheating
node bows out automatically.

### 3.2 Component scores (each clamped to `[0.0, 1.0]`)

```rust
mem_score  = (mem_left  / mem_total ).clamp(0,1)   // mem_left  = mem_available  − mem_request  (saturating)
disk_score = (disk_left / disk_total).clamp(0,1)   // disk_left = disk_available − disk_request (saturating)
cpu_score  = (1.0 − (cpu_load_ratio / max_cpu_load)).clamp(0,1)
```

### 3.3 Multiplicative penalties

**Temperature penalty** (`:91-96`) — inactive below 60 °C, then linear falloff to 0 at `critical_temp`:

```rust
temp_penalty = if nvme_temp <= 60.0 { 1.0 }
               else { ((critical_temp − nvme_temp) / (critical_temp − 60.0)).clamp(0,1) };
```

**Migration penalty** (`:100`) — each in-flight migration shaves 15 %; floors at 0 by ~7 concurrent migrations:

```rust
migration_penalty = (1.0 − (active_migrations as f64 * 0.15)).clamp(0.0, 1.0);
```

### 3.4 Final score

Weighted **Memory 40 % / CPU 40 % / Disk 20 %**, scaled by both penalties, mapped to 1–1000:

```rust
// bidder.rs:102-108
let raw_score = (mem_score * 0.4 + cpu_score * 0.4 + disk_score * 0.2)
              * temp_penalty * migration_penalty;
(1.0 + raw_score * 999.0) as i32
```

**Valid range: 1–1000; rejection: −1.** This matches the proto contract
(`aether.proto:48`).

> **Maturity.** The rejection boundaries and range are unit- and integration-tested
> (`bidder.rs:112-181`, `tests/bidding_resource_thresholds.rs`). The exact numeric
> score of a *healthy* bid, the 60–80 °C temperature gradient, and the migration
> penalty path are **not** currently asserted by tests.

---

## 4. Broadcast & convergence loop

`crates/aether-aggregator/src/scheduler.rs`.

```rust
pub struct Scheduler {
    registry: Arc<RwLock<NodeRegistry>>,
    client_tls_config: ClientTlsConfig,
}
```

`broadcast_bid(cpu, memory_bytes, disk_bytes, workload_uuid) -> Vec<BidResponse>` (`:33-102`):

1. Snapshot the active nodes under a **read** lock (`:40-43`).
2. Spawn one task **per node** into a `tokio::task::JoinSet` (fan-out).
3. Each task builds a tonic `Channel` with mTLS and a **250 ms** `connect_timeout`
   **and** `timeout` (`:59-60`), then calls `request_reverse_bid`.
4. The per-node future is *additionally* wrapped in
   `tokio::time::timeout(Duration::from_millis(250), …)` (`:80`). **The 250 ms
   auction window appears three times** — connect, request, and outer bound.
5. Results are drained via `join_next().await`; only `Ok(Some(bid))` survive. A
   node that errors, fails to connect, or misses the window is logged
   (`log::warn!`) and simply **dropped from this auction** — there is no retry.

The convergence window is thus enforced end-to-end at ~250 ms. The integration
test `tests/auction_convergence_timing.rs` proves a node responding at 350 ms is
excluded while 50 ms / 120 ms nodes are included.

### Winner selection

`select_winner(bids, ssd_wears, chassis_active_vms) -> Result<Option<BidResponse>, String>` (`:106-161`):

1. Keep bids with `score > 0` (drops both `−1` rejections and any `0`). Empty → `Ok(None)`.
2. Find `max_score`; collect all bids equal to it.
3. Exactly one top bid → return it.
4. Tie → build `TieBreakerCandidate`s and defer to `tie_breaker::resolve_tie`, then map the winning `node_id` back to its `BidResponse`.

> **Design gap to note.** `ssd_wears` and `chassis_active_vms` are **caller-supplied**.
> No code in these crates yet populates them from real hardware — today they are
> provided only by tests. Real SSD-wear / chassis-density feeds are a pending
> integration.

---

## 5. Deterministic tie-breaker

`crates/aether-aggregator/src/tie_breaker.rs`.

```rust
pub struct TieBreakerCandidate { pub node_id: String, pub ssd_wear: f64 }
pub fn resolve_tie(candidates: &[TieBreakerCandidate],
                   chassis_active_vms: &HashMap<u32, u32>) -> Result<TieBreakerCandidate, String>
```

Slot number is parsed from the `node_id` (`parse_slot_number`, `:9-21`; requires a
`blade-<n>` form, rejects slot 0). Candidates are pre-parsed so the sort closure is
infallible, then sorted ascending ("lower is better"); the winner is the first
element. **Ordered tiers:**

1. **Adjacent-slot density** (`:69-73`) — sum of active VMs in `slot−1` and `slot+1`
   (from `chassis_active_vms`, missing = 0). Prefers physically isolated slots to
   spread thermal/failure load.
2. **SSD write wear** (`:75-83`) — lower wear wins; `NaN` treated as equal.
3. **Physical slot number** (`:85-87`) — final, always-decisive fallback.

**Determinism** is guaranteed because slot number is unique per node and is the
terminal criterion, so a total order always exists regardless of input order,
sort stability, or `NaN` wear values.

> **Maturity.** All three tiers plus slot parsing are unit-tested
> (`tie_breaker.rs:95-179`) and exercised end-to-end via
> `tests/deterministic_scheduling_selection.rs`.

---

## 6. Node registry & heartbeat state machine

`crates/aether-aggregator/src/registry.rs`.

```rust
pub struct NodeInfo {
    pub node_id: String, pub grpc_endpoint: String,
    pub pool: String,                 // "COMPUTE" | "INFRA"
    pub token: String,                // ephemeral attestation token
    pub last_seen_heartbeat: tokio::time::Instant,
}
pub struct NodeRegistry { nodes: HashMap<String, NodeInfo> }   // #[derive(Default)]
```

The registry is a plain `HashMap`; thread-safety comes from wrapping it in
`Arc<RwLock<NodeRegistry>>` (`tokio::sync::RwLock`) at every call site. Reads take
`read().await`; mutations take `write().await`.

| Operation | Behavior |
| :--- | :--- |
| `register(id, endpoint, pool, token)` (`:39`) | Upsert `NodeInfo`, stamp `last_seen = now`. |
| `renew_heartbeat(id, token)` (`:57`) | Refresh timestamp **iff** token matches, else `Err`. Unknown node → `Err`. |
| `deregister(id)` (`:71`) | Remove and return the entry. |
| `prune_inactive_nodes(threshold)` (`:77`) | `retain` nodes seen within `threshold`; return pruned ids. |
| `get_active_nodes()` (`:92`) | Clone all current entries ("active" = present in map). |

### Lifecycle wiring

- **Registration** (`aggregator/src/lib.rs:42-59`): `register_node` mints an
  ephemeral token via the `TokenManager` and returns it in `RegisterNodeResponse`.
- **Heartbeat** (`lib.rs:61-78`): `send_heartbeat` validates the token, then calls
  `renew_heartbeat`; failures map to `Status::unauthenticated` / `invalid_argument`.
- **Pruning** (`aggregator/src/main.rs:34-46`): a background task ticks every
  **3 s** and prunes nodes unseen for **15 s**. These are the effective heartbeat
  TTL values — the registry itself hardcodes no interval.

State per node, in effect:

```
        register_node                heartbeat within 15s
  (absent) ─────────────► ACTIVE ──────────────────────────► ACTIVE
                            │  ▲                                 │
              deregister    │  └── renew_heartbeat (token ok) ───┘
                            ▼
                        (removed)  ◄──── prune (no heartbeat 15s)
```

> **Maturity.** Registry operations are unit-tested (`registry.rs:97-172`) and the
> 3 s-tick / 15 s-TTL pruner is verified end-to-end against the real gRPC handlers
> using `tokio::time::pause`/`advance` in `tests/heartbeat_timeout_tests.rs`.

---

## 7. Wire types (`proto/aether.proto`)

| Message | Fields |
| :--- | :--- |
| `RegisterNodeRequest` | `node_id`, `grpc_endpoint`, `pool` ("COMPUTE"/"INFRA") |
| `RegisterNodeResponse` | `success: bool`, `token` |
| `HeartbeatRequest` | `node_id`, `token` |
| `HeartbeatResponse` | `success: bool` |
| `BidRequest` | `workload_uuid`, `cpu_request: i32`, `memory_request_bytes: i64`, `disk_request_bytes: i64` |
| `BidResponse` | `node_id`, `score: i32` (−1 rejected, else 1–1000) |

`RegisterNode` / `SendHeartbeat` live on the `AetherAggregator` service;
`RequestReverseBid` lives on the `AetherNode` service. Full protocol table in the
[Security & Protocol Reference](./impl_security_protocol.md).

---

## 8. Known gaps & non-goals (current build)

- No real SSD-wear or chassis-VM-density feed into `select_winner` (test-supplied only).
- No bidding retry/hysteresis — a missed 250 ms window means exclusion from that auction.
- The aggregator's convergence is tested against mock scorers; the full
  telemetry → bidder pipeline is exercised only on the `aetherd` side.
