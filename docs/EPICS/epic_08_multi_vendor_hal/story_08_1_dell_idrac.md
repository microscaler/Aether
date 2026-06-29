# Story: ChassisManager Dell iDRAC Redfish API Driver

*   **Status:** Draft
*   **Story ID:** `STORY-08.1`
*   **Parent Epic:** [EPIC-08: Multi-Vendor Hardware Abstraction](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/epic_08_multi_vendor_hal.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a fencing controller
I want fence and reset Dell PowerEdge MX7000 blades using Dell iDRAC Redfish APIs
So that we can expand Project Aether to support Dell hardware environments without changing scheduling loops
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-8.1.1: Implement the ChassisManager trait for Dell iDRAC controllers.
*   FR-8.1.2: Send Dell-specific Redfish REST commands to execute hard power shutdowns and reset slots.

### B. Non-Functional Requirements
*   NFR-8.1.1: Power control operations must complete in `< 5 seconds`.
*   NFR-8.1.2: Ensure uniform return values and error types matching the HPE driver.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-fence`
*   **Target Files:**
*   [crates/aether-fence/src/redfish/idrac.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-fence/src/redfish/idrac.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a Dell PowerEdge blade in Slot 2
    *   **When** When the fencing coordinator invokes ChassisManager::power_off(2)
    *   **Then** Then the Dell iDRAC driver executes force-off commands and confirms the slot shutdown.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-fence dell_idrac_tests`
*   **Integration Tests:** Run `cargo test --test dell_chassis_fencing_integration`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
