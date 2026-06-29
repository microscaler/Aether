# Story: Virtual Connect REST Driver

*   **Status:** Draft
*   **Story ID:** `STORY-04.4`
*   **Parent Epic:** [EPIC-04: Storage Slicing & Net Tagging](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/epic_04_storage_net_tagging.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator
I want tag and configure physical VLANs on midplane switches via Virtual Connect REST drivers
So that tenant networks are dynamically isolated at the hardware switch level
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-4.4.1: Authenticate and interface with HPE Virtual Connect switch modules.
*   FR-4.4.2: Programmatically tag, untag, and verify tenant VLAN IDs on target blade server profiles.

### B. Non-Functional Requirements
*   NFR-4.4.1: Switch configuration calls must execute asynchronously to prevent blocking scheduling loops.
*   NFR-4.4.2: Handle switch API rate limits and retry queries safely on network failures.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/network/hpe_vc.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/network/hpe_vc.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a VM placed on Blade 9 requiring tenant VLAN 105
    *   **When** When the Aggregator initiates network provisioning
    *   **Then** Then the HPE Virtual Connect switch tags VLAN 105 on Slot 9 and verifies interface connectivity.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator virtual_connect_client_tests`
*   **Integration Tests:** Run `cargo test --test switch_vlan_tagging_integration`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
