# Story: ChassisManager Generic Redfish Compliance Driver

*   **Status:** Draft
*   **Story ID:** `STORY-08.3`
*   **Parent Epic:** [EPIC-08: Multi-Vendor Hardware Abstraction](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/epic_08_multi_vendor_hal.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a fencing controller
I want power cycle chassis slots using baseline DMTF Redfish compliance models
So that Aether can fence worker blades on any standard Redfish-compliant hardware platform
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-8.3.1: Implement a fallback driver under the ChassisManager trait using standard Redfish schemas.
*   FR-8.3.2: Perform power actions and query slot status using standard DMTF REST endpoints.

### B. Non-Functional Requirements
*   NFR-8.3.1: The driver must run successfully with zero custom vendor logic.
*   NFR-8.3.2: Reject connections if target endpoints violate basic TLS parameters.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-fence`
*   **Target Files:**
*   [crates/aether-fence/src/redfish/generic.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-fence/src/redfish/generic.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a Redfish-compliant server chassis from a generic vendor
    *   **When** When executing power_off calls
    *   **Then** Then the generic Redfish driver shuts down the slot and returns status success.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-fence generic_redfish_tests`
*   **Integration Tests:** Run `cargo test --test generic_redfish_chassis_control`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
