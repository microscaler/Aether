# Story: Auto-Converge vCPU Throttling

*   **Status:** Draft
*   **Story ID:** `STORY-05.4`
*   **Parent Epic:** [EPIC-05: Live Migration & Auto-Convergence](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_05_live_migration/epic_05_live_migration.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want throttle guest vCPU write cycles dynamically if guest memory dirty rate exceeds network transfer speeds
So that live migrations can converge successfully even under high write workloads
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-5.4.1: Monitor guest memory dirty rates vs network bandwidth.
*   FR-5.4.2: Enable QEMU auto-converge features to gradually throttle guest vCPU execution times during migration.

### B. Non-Functional Requirements
*   NFR-5.4.1: Slowly increase throttling from 10% to 99% based on convergence rates.
*   NFR-5.4.2: Lift all vCPU throttling immediately once the migration is finalized or aborted.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/migration/converge.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/migration/converge.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a write-heavy database VM failing to converge during live migration
    *   **When** When auto-converge is activated
    *   **Then** Then guest vCPUs are throttled dynamically, memory dirty rates drop, and the migration converges.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd auto_converge_tests`
*   **Integration Tests:** Run `cargo test --test migration_convergence_under_heavy_write`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
