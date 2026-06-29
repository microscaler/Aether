# Story: iLO 5 Redfish API Client & Power Command Driver

*   **Status:** Draft
*   **Story ID:** `STORY-06.1`
*   **Parent Epic:** [EPIC-06: Out-of-Band Fencing & HA](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/epic_06_oob_fencing_ha.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a fencing controller
I want control physical blade slot power states using HPE iLO 5 Redfish APIs
So that I can execute out-of-band hard power fences on unresponsive physical blades
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-6.1.1: Connect and authenticate with HPE iLO 5 controllers over VLAN 999.
*   FR-6.1.2: Send Redfish REST commands to query slot status, execute hard resets, and force power shutdowns.

### B. Non-Functional Requirements
*   NFR-6.1.1: BMC query calls and power actions must complete in less than 5 seconds.
*   NFR-6.1.2: Enforce secure HTTPS certificate validation on all iLO connections.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-fence`
*   **Target Files:**
*   [crates/aether-fence/src/redfish/ilo.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-fence/src/redfish/ilo.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given an active HPE iLO 5 endpoint for Slot 9
    *   **When** When sending a force off power request
    *   **Then** Then the Redfish client returns a success status and Blade 9 immediately powers down.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-fence redfish_ilo_client_tests`
*   **Integration Tests:** Run `cargo test --test ilo_power_cycle_verification`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
