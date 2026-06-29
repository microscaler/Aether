# Story: Block Level Replication (Drive Mirroring over NBD)

*   **Status:** Draft
*   **Story ID:** `STORY-05.2`
*   **Parent Epic:** [EPIC-05: Live Migration & Auto-Convergence](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_05_live_migration/epic_05_live_migration.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want mirror VM block storage live over NBD to a destination ZVOL
So that active writes are synced continuously without halting guest VM execution
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-5.2.1: Set up NBD server endpoints on the destination host mapping to ZVOL targets.
*   FR-5.2.2: Execute QMP `drive-mirror` commands on the source QEMU VM to sync blocks over NBD.

### B. Non-Functional Requirements
*   NFR-5.2.1: Block synchronization must run concurrently with active guest writes.
*   NFR-5.2.2: Limit block mirror network usage to 80% of interface capacity to protect control plane traffic.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/migration/block.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/migration/block.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a running VM with continuous block writes
    *   **When** When drive mirroring is initiated
    *   **Then** Then ZVOL blocks are synchronized to the destination node, and write changes mirror instantly.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd block_mirror_tests`
*   **Integration Tests:** Run `cargo test --test nbd_stream_performance`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
