# Epic: Stateless Reverse-Bidding & Scheduling

*   **Status:** Active
*   **Epic ID:** `EPIC-02`
*   **Target Roadmap Stage:** Stage 2: Stateless Reverse-Bidding & Scheduling
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic implements the core scheduling mechanism of Project Aether: the pull-based, decentralized **Reverse-Bidding Marketplace**. Instead of a central master orchestrating host allocations based on a stale database, the Aggregator broadcasts workload specifications to all blades, and individual blades run local telemetry evaluations to submit algorithmic bids.

## 2. User Stories

- [ ] `[STORY-02.1]` [In-Memory Node Registry & Telemetry Tables](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/story_02_1_node_registry.md) - **Status:** Draft
- [ ] `[STORY-02.2]` [Async 250ms Bid Broadcast and Convergence Loop](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/story_02_2_bid_broadcast.md) - **Status:** Draft
- [ ] `[STORY-02.3]` [Local Telemetry Collector (CPU Loadavg, Meminfo, Channel Pressure)](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/story_02_3_telemetry_collector.md) - **Status:** Draft
- [ ] `[STORY-02.4]` [Bidding Calculator & Algorithm](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/story_02_4_bidding_algorithm.md) - **Status:** Draft
- [ ] `[STORY-02.5]` [Deterministic Tie-Breaker Resolution Engine](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/story_02_5_tie_breaker_resolver.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aether-aggregator/` (Broadcast loop, `tokio::sync::RwLock` registry, tie-breaker resolver)
    *   `crates/aetherd/` (Telemetry collection, raw bidding algorithm execution)
*   **CRD / API Boundaries:**
    *   `AetherVirtualDeployment` CRD fields for CPU/Memory requests and tenant allocations.
    *   `aether.proto` updates to support bidirectional `BidRequest` and `BidResponse` structures.
*   **Core Traits:**
    *   `TelemetryCollector`: Abstracting OS queries to `/proc` or `/sys` files.

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-01` (Core API & Substrate)
*   **Hardware/Environment Requirements:**
    *   Multi-blade network (or simulated mock network) to demonstrate bidding convergence.
*   **Third-Party Libraries:**
    *   `sysinfo` or direct `/proc` parser libraries.
    *   `futures` (for concurrent broadcast handles).

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Only attestation-verified blades can participate in the auction. Bids received from unauthorized hosts are immediately discarded.
*   **Secrets Handling:** Node telemetry details are kept private to the chassis control bus and must not contain tenant secret details.
*   **Hardware Safety:** If a node's local safety check detects thermal warnings, it must return a `-1` bid, automatically removing itself from placement.

## 6. Epic Acceptance Criteria

1. The Aggregator manages node registrations dynamically.
2. Broadcasting a workload request triggers parallel evaluation across registered nodes.
3. The convergence loop successfully aggregates bids and chooses the winner within a `250ms` window.
4. If multiple nodes return identical scores, the tie-breaker resolver selects a node deterministically using chassis parameters.
