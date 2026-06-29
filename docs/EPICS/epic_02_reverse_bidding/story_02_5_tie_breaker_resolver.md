# Story: Deterministic Tie-Breaker Resolution Engine

*   **Status:** Draft
*   **Story ID:** `STORY-02.5`
*   **Parent Epic:** [EPIC-02: Stateless Reverse-Bidding & Scheduling](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/epic_02_reverse_bidding.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator scheduler
I want resolve identical bid scores using chassis physical layouts and node parameters
So that workloads are placed consistently without scheduling deadlocks
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-2.5.1: Implement tie-breaking rules: compare adjacent slot densities, disk wear parameters, and physical chassis slot numbers.
*   FR-2.5.2: Select a single winner node deterministically.

### B. Non-Functional Requirements
*   NFR-2.5.1: Tie-breaker calculations must execute in under 1ms.
*   NFR-2.5.2: Avoid random number generation to ensure scheduler debugging and replication consistency.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/tie_breaker.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/tie_breaker.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given 2 nodes submitting an identical bid score of 850
    *   **When** When resolving the auction winner
    *   **Then** Then the node in the slot adjacent to fewer active VMs or with less SSD wear must be selected.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator tie_breaker_tests`
*   **Integration Tests:** Run `cargo test --test deterministic_scheduling_selection`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
