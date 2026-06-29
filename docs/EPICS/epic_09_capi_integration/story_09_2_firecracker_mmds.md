# Story: Firecracker MMDS Bootstrap Configuration Delivery

*   **Status:** Draft
*   **Story ID:** `STORY-09.2`
*   **Parent Epic:** [EPIC-09: Cluster API (CAPI) Integration](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/epic_09_capi_integration.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want populate local MicroVM Metadata Service (MMDS) endpoints with guest user-data configurations
So that ephemeral Firecracker VMs can query link-local ports to pull bootstrap configurations
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-9.2.1: Write user-data JSON structures to local MMDS socket files during Firecracker boot loops.
*   FR-9.2.2: Ensure the local HTTP MMDS service responds to guest requests on the link-local IP (`169.254.169.254`).

### B. Non-Functional Requirements
*   NFR-9.2.1: MMDS response latencies must be `< 5ms`.
*   NFR-9.2.2: Block access to MMDS endpoints from outside the guest VM's local interface.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/hypervisor/mmds.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/hypervisor/mmds.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a running guest microVM trying to access metadata on 169.254.169.254
    *   **When** When the guest queries http://169.254.169.254/latest/user-data
    *   **Then** Then the local MMDS responds with the complete, base64-decoded user-data script.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd firecracker_mmds_tests`
*   **Integration Tests:** Run `cargo test --test guest_metadata_http_query`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
