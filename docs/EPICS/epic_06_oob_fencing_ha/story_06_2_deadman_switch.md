# Story: Reconciler Deadman Switch Heartbeat & Failover Monitoring

*   **Status:** Draft
*   **Story ID:** `STORY-06.2`
*   **Parent Epic:** [EPIC-06: Out-of-Band Fencing & HA](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/epic_06_oob_fencing_ha.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator
I want monitor worker blade heartbeats and detect physical node failures
So that we can initiate failover recovery loops within 15 seconds of a node crash
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-6.2.1: Listen for periodic heartbeat messages sent by node daemons.
*   FR-6.2.2: Trigger the node failure recovery loop if heartbeats from a node are missed for 15 seconds.

### B. Non-Functional Requirements
*   NFR-6.2.1: Heartbeat verification must execute in `< 1ms` to prevent operator thread blockages.
*   NFR-6.2.2: Heartbeat messages must carry cryptographic signatures to prevent spoofing.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/ha/heartbeat.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/ha/heartbeat.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a blade node that suddenly loses power
    *   **When** When the Aggregator misses its heartbeats for 15 seconds
    *   **Then** Then the node state is marked Offline and the recovery loop is triggered.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator heartbeat_monitor_tests`
*   **Integration Tests:** Run `cargo test --test deadman_switch_timeout`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
