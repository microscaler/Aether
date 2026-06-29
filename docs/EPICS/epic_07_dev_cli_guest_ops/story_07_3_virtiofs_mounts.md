# Story: Local Host Path Passthrough Mounts via VirtioFS

*   **Status:** Draft
*   **Story ID:** `STORY-07.3`
*   **Parent Epic:** [EPIC-07: Developer CLI & Guest Operations](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/epic_07_dev_cli_guest_ops.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a tenant
I want pass host directory paths directly through into guest VMs using VirtioFS mounts
So that files are shared between the host and the guest with high performance and no network protocol overhead
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-7.3.1: Spawn and manage VirtioFS daemon processes (`virtiofsd`) targeting specific host directories.
*   FR-7.3.2: Configure QEMU parameters to map VirtioFS devices and mount them inside guest VMs.

### B. Non-Functional Requirements
*   NFR-7.3.1: Folder read/write speeds over VirtioFS must be within 90% of native host disk speeds.
*   NFR-7.3.2: Prevent guest VMs from escaping directory structures using Linux user namespaces and cgroup paths.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/hypervisor/virtiofs.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/hypervisor/virtiofs.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a host folder at /mnt/data
    *   **When** When attaching the path to a VM using VirtioFS
    *   **Then** Then the guest VM mounts the share and successfully performs read/write operations on the files.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd virtiofs_config_tests`
*   **Integration Tests:** Run `cargo test --test virtiofs_mount_performance`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
