# Story: QEMU-KVM Command Builder & Execution Loop

*   **Status:** Draft
*   **Story ID:** `STORY-03.3`
*   **Parent Epic:** [EPIC-03: Dual Hypervisor Engine](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/epic_03_dual_hypervisor.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want build QEMU commands dynamically and manage KVM execution lifecycles on infrastructure nodes
So that long-lived databases and Kubernetes nodes can run with native virtual machine performance
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-3.3.1: Compile standard QEMU execution commands specifying vCPU counts, RAM size, disk mappings, and network cards.
*   FR-3.3.2: Manage the QEMU process execution loop, capturing state transitions and errors via QMP sockets.

### B. Non-Functional Requirements
*   NFR-3.3.1: Guest virtualization overhead must be restricted to `< 3%` of bare-metal capabilities.
*   NFR-3.3.2: Clean up all host network bridges and ZVOL mappings if the QEMU process crashes.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/hypervisor/qemu.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/hypervisor/qemu.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a request to launch a persistent database VM on slot 9
    *   **When** When spawning the QEMU process with KVM enabled
    *   **Then** Then the VM runs successfully, exposes QMP interface sockets, and registers with the local daemon.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd qemu_command_builder_tests`
*   **Integration Tests:** Run `cargo test --test qemu_kvm_vm_lifecycle`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
