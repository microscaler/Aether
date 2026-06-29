# Story: Asynchronous ZFS Volume Replication

*   **Status:** Draft
*   **Story ID:** `STORY-06.4`
*   **Parent Epic:** [EPIC-06: Out-of-Band Fencing & HA](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/epic_06_oob_fencing_ha.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want replicate ZVOL snapshot deltas asynchronously to a secondary chassis host every 5 minutes
So that fenced VMs can resume operations on a new node with a maximum of 5 minutes of data loss (RPO 5m)
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-6.4.1: Automate cron-triggered ZVOL snapshot actions every 5 minutes.
*   FR-6.4.2: Transmit incremental snapshot differences using zfs send/recv over secure control networks.

### B. Non-Functional Requirements
*   NFR-6.4.1: Limit replication time to under 3 minutes to prevent overlapping replication tasks.
*   NFR-6.4.2: Max RPO data loss must be capped at 5 minutes.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/storage/replication.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/storage/replication.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given an active ZVOL on Blade 9
    *   **When** When the replication trigger fires
    *   **Then** Then an incremental ZFS snapshot is sent and successfully received by a standby host (e.g. Blade 10).
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd zfs_replication_tests`
*   **Integration Tests:** Run `cargo test --test incremental_rpo_validation`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
