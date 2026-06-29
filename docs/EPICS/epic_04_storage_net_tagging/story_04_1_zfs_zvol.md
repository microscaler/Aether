# Story: ZFS ZVOL Provisioning, Snapshots, & Thin-Clone Automation

*   **Status:** Draft
*   **Story ID:** `STORY-04.1`
*   **Parent Epic:** [EPIC-04: Storage Slicing & Net Tagging](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/epic_04_storage_net_tagging.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want clone ZVOL base template snapshots dynamically as thin-provisioned block devices for new VMs
So that VM local disks can be provisioned in milliseconds with zero initial disk space consumption
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-4.1.1: Automate ZVOL block creation, snapshotted clones, and volume resize commands.
*   FR-4.1.2: Implement thin-provisioning configurations and ZFS user quotas to limit maximum disk footprint.

### B. Non-Functional Requirements
*   NFR-4.1.1: Clone creation times must complete in `< 100ms` (0ms block copy overhead on ZFS).
*   NFR-4.1.2: Restrict ZFS ARC caches to 15% of host memory on storage blades to protect VM RAM.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/storage/zfs.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/storage/zfs.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given an active ZFS zpool containing a base OS snapshot
    *   **When** When creating a thin-provisioned clone for a new VM
    *   **Then** Then the ZVOL block device is instantly available under /dev/zvol/ and ready for mounting.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd zfs_zvol_tests`
*   **Integration Tests:** Run `cargo test --test zfs_thin_provisioning_limits`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
