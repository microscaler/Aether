# Story: Integration with democratic-csi

*   **Status:** Draft
*   **Story ID:** `STORY-04.3`
*   **Parent Epic:** [EPIC-04: Storage Slicing & Net Tagging](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/epic_04_storage_net_tagging.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a storage administrator
I want integrate ZFS volume provisioning with democratic-csi interfaces
So that Kubernetes persistent volumes can map directly and dynamically to Aether ZVOL blocks
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-4.3.1: Respond to standard CSI volume creation, deletion, and mounting requests.
*   FR-4.3.2: Map K8s volume requests to backend Aether ZVOL structures on storage blades.

### B. Non-Functional Requirements
*   NFR-4.3.1: CSI volume provisioning requests must be resolved in less than 5 seconds.
*   NFR-4.3.2: Secure volume mounts using tenant namespace isolation rules.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/storage/csi.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/storage/csi.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a K8s PVC requesting ZFS storage
    *   **When** When democratic-csi triggers the volume allocation
    *   **Then** Then a thin-provisioned ZVOL is created on Aether storage blades and attached to the target pod VM.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator csi_driver_tests`
*   **Integration Tests:** Run `cargo test --test csi_zvol_mount_lifecycle`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
