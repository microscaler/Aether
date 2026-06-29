# Story: MidplaneNetworkManager Dell SmartFabric Services (SFS) Driver

*   **Status:** Draft
*   **Story ID:** `STORY-08.2`
*   **Parent Epic:** [EPIC-08: Multi-Vendor Hardware Abstraction](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/epic_08_multi_vendor_hal.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator
I want tag midplane switch networks via Dell SmartFabric Services (SFS) APIs
So that Dell blades can be partitioned on tenant networks automatically at the hardware switch level
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-8.2.1: Implement the MidplaneNetworkManager trait for Dell switch modules.
*   FR-8.2.2: Program SFS APIs to tag/untag VLAN IDs on Dell MX7000 midplane interfaces.

### B. Non-Functional Requirements
*   NFR-8.2.1: API connections must use TLS validation and securely store credentials.
*   NFR-8.2.2: Midplane configuration calls must execute asynchronously.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/network/dell_sfs.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/network/dell_sfs.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a VM placed on a Dell blade slot requiring VLAN 200
    *   **When** When provisioning the VLAN interface
    *   **Then** Then the SFS driver tags VLAN 200 on the midplane switch fabric successfully.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator dell_sfs_tests`
*   **Integration Tests:** Run `cargo test --test dell_smartfabric_vlan_tagging`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
