# Story: STONITH Fencing Execution Workflow

*   **Status:** Draft
*   **Story ID:** `STORY-06.3`
*   **Parent Epic:** [EPIC-06: Out-of-Band Fencing & HA](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/epic_06_oob_fencing_ha.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator
I want execute STONITH fencing workflows before re-auctioning orphaned virtual machines
So that duplicate volume mounts and database split-brain corruption are prevented
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-6.3.1: Intercept node failures and dispatch force-off commands to the target blade's iLO BMC.
*   FR-6.3.2: Confirm power-off status via BMC queries before releasing ZVOL locks and re-auctioning VMs.

### B. Non-Functional Requirements
*   NFR-6.3.1: Fencing workflows must prioritize deterministic shutdown verification over timing limits.
*   NFR-6.3.2: Abort re-auctioning and raise critical alarms if the fencing command fails to verify shutdown.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/ha/fencing.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/ha/fencing.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a failed node hosting a database VM
    *   **When** When the recovery loop starts
    *   **Then** Then iLO shuts down the node, confirms the off state, and only then does the Aggregator start the recovery auction.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator fencing_workflow_tests`
*   **Integration Tests:** Run `cargo test --test split_brain_prevention_on_failover`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
