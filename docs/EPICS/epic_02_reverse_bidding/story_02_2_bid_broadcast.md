# Story: Async 250ms Bid Broadcast and Convergence Loop

*   **Status:** Draft
*   **Story ID:** `STORY-02.2`
*   **Parent Epic:** [EPIC-02: Stateless Reverse-Bidding & Scheduling](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/epic_02_reverse_bidding.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator scheduler
I want broadcast bid requests to all registered blades and converge on responses within a strict 250ms window
So that Aether can select a hosting node and place workloads dynamically in sub-second timelines
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-2.2.1: Concurrently broadcast RequestReverseBid messages to all registered node daemons via async gRPC client connections.
*   FR-2.2.2: Implement a strict 250ms timeout window to collect all BidResponses.

### B. Non-Functional Requirements
*   NFR-2.2.1: The broadcast window must close exactly at 250ms (+/- 5ms), discarding any slow or late bids.
*   NFR-2.2.2: Ensure non-blocking execution so that parallel deployments can trigger independent bidding pools simultaneously.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/scheduler.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/scheduler.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a deployment request broadcasted to 3 nodes where one node responds in 300ms
    *   **When** When compiling the auction bids after 250ms
    *   **Then** Then the late node's bid must be discarded, and only the 2 on-time bids are evaluated.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator scheduler_broadcast_tests`
*   **Integration Tests:** Run `cargo test --test auction_convergence_timing`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
