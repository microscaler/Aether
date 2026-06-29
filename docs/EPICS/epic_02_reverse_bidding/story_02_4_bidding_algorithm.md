# Story: Bidding Calculator & Algorithm

*   **Status:** Draft
*   **Story ID:** `STORY-02.4`
*   **Parent Epic:** [EPIC-02: Stateless Reverse-Bidding & Scheduling](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/epic_02_reverse_bidding.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want evaluate a workload request spec and calculate an efficiency score between 1 and 1000
So that the Aggregator can determine which node has the best resource fit for the virtual machine
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-2.4.1: Compare requested CPU and RAM spec against local telemetry availability.
*   FR-2.4.2: Implement mathematical score calculation: higher scores indicate lower resource utilization and better alignment.
*   FR-2.4.3: Return a score of `-1` if resources are exhausted or overcommit thresholds are violated.

### B. Non-Functional Requirements
*   NFR-2.4.1: Bidding algorithm math must be deterministic (same telemetry + same spec = same score).
*   NFR-2.4.2: Execute bidding calculations in `< 1ms`.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/bidder.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/bidder.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a request for 8GB RAM on a node with only 4GB free memory
    *   **When** When calculating the bidding score
    *   **Then** Then the node must return a score of -1 to disqualify itself.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd bidding_calculator_tests`
*   **Integration Tests:** Run `cargo test --test bidding_resource_thresholds`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
