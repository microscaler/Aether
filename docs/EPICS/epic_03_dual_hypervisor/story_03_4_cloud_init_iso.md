# Story: Dynamic NoCloud Cloud-Init ISO Builder

*   **Status:** Draft
*   **Story ID:** `STORY-03.4`
*   **Parent Epic:** [EPIC-03: Dual Hypervisor Engine](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/epic_03_dual_hypervisor.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want compile Cloud-Init configuration files into a custom NoCloud ISO in memory
So that guest virtual machines can boot and auto-configure safely without writing secrets to permanent disk
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-3.4.1: Write dynamic `user-data` and `meta-data` files to a temporary in-memory `tmpfs` path.
*   FR-3.4.2: Compile these files into a standard ISO 9660 volume image (`seed.iso`) and mount it to the VM CD-ROM drive.

### B. Non-Functional Requirements
*   NFR-3.4.1: ISO compilation and mounting must execute in less than 100ms.
*   NFR-3.4.2: No plaintext secrets (e.g. root passwords) must touch host SSD filesystems during generation.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/cloud_init.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/cloud_init.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given decrypted bootstrap credentials in memory
    *   **When** When executing the ISO compilation process
    *   **Then** Then a valid ISO image is created in a RAM drive and attached to the hypervisor configurations.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd cloud_init_compiler_tests`
*   **Integration Tests:** Run `cargo test --test cloud_init_iso_guest_boot`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
