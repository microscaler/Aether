# Story: Firecracker Process Orchestration & Console Routing

*   **Status:** Draft
*   **Story ID:** `STORY-03.1`
*   **Parent Epic:** [EPIC-03: Dual Hypervisor Engine](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/epic_03_dual_hypervisor.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want spawn and manage Firecracker microVM processes with dynamic CPU/RAM parameters and console routing
So that ephemeral developer workspaces can boot in sub-100ms with hardware-level isolation
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-3.1.1: Build Firecracker configuration JSON specs dynamically from CRD resources.
*   FR-3.1.2: Spawn Firecracker processes, configure jailer settings, and pipe standard serial/console outputs to local logs.

### B. Non-Functional Requirements
*   NFR-3.1.1: Firecracker microVM boot time (from process spawn to kernel execute) must be `< 100ms`.
*   NFR-3.1.2: Restrict running memory overhead of the Firecracker wrapper to `< 5MB` per instance.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/hypervisor/firecracker.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/hypervisor/firecracker.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a request to launch an ephemeral microVM with 1 vCPU and 512MB RAM
    *   **When** When spawning the Firecracker process
    *   **Then** Then the microVM boots and runs guest kernel execution in under 100ms.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd firecracker_config_tests`
*   **Integration Tests:** Run `cargo test --test firecracker_vm_boot_lifecycle`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
