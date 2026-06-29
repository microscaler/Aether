# Story: MidplaneNetworkManager Lenovo Flex System CLI Driver

*   **Status:** Draft
*   **Story ID:** `STORY-08.4`
*   **Parent Epic:** [EPIC-08: Multi-Vendor Hardware Abstraction](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/epic_08_multi_vendor_hal.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator
I want tag midplane networks on Lenovo Flex System switches using CLI wrappers over SSH
So that we can provision VLAN interfaces on older Lenovo chassis switch modules
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-8.4.1: Implement the MidplaneNetworkManager trait for Lenovo Flex switches.
*   FR-8.4.2: Connect via SSH and run CLI commands to tag and untag tenant VLAN IDs on server ports.

### B. Non-Functional Requirements
*   NFR-8.4.1: Enforce private key SSH authentication without hardcoded passwords.
*   NFR-8.4.2: Connection and command timeouts must be strictly enforced (e.g. `< 10s`).

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/network/lenovo_flex.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/network/lenovo_flex.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a Lenovo Flex blade slot requiring VLAN 150
    *   **When** When running the network provisioning task
    *   **Then** Then the Lenovo driver establishes SSH, executes the CLI tags, and verifies port VLAN states.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator lenovo_flex_tests`
*   **Integration Tests:** Run `cargo test --test lenovo_switch_ssh_integration`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
