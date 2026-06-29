# Story: In-Memory Node Registry & Telemetry Tables

*   **Status:** Draft
*   **Story ID:** `STORY-02.1`
*   **Parent Epic:** [EPIC-02: Stateless Reverse-Bidding & Scheduling](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/epic_02_reverse_bidding.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator scheduler
I want maintain a thread-safe in-memory registry of worker blades and their current telemetry
So that I can dispatch bid requests only to active, responsive blades without relying on a database
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-2.1.1: Provide registration, deregistration, and heartbeat lease renewal APIs for blade nodes.
*   FR-2.1.2: Maintain a thread-safe node list containing active IP addresses, pools (Compute/Infra), and telemetry snapshots.

### B. Non-Functional Requirements
*   NFR-2.1.1: Registry access must utilize non-blocking locking structures (tokio RwLock) to support parallel auction handling.
*   NFR-2.1.2: Prune inactive nodes if no heartbeat is received within 15 seconds.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/registry.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/registry.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given an active node daemon sending a register RPC request
    *   **When** When processing in the Aggregator
    *   **Then** Then the node must be added to the registry and return a success lease token.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator registry_tests`
*   **Integration Tests:** Run `cargo test --test node_deregistration_on_heartbeat_timeout`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
