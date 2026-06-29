# Story: Dynamic Linux Bridge and VLAN Tag Interface Setup

*   **Status:** Draft
*   **Story ID:** `STORY-04.2`
*   **Parent Epic:** [EPIC-04: Storage Slicing & Net Tagging](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/epic_04_storage_net_tagging.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want provision virtual TAP interfaces and host bridge networks tagged with tenant VLAN IDs
So that tenant network packets are partitioned securely at the physical midplane level
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-4.2.1: Programmatically create Linux bridge devices and TAP interfaces for virtual machines.
*   FR-4.2.2: Tag guest network interfaces with tenant-specific VLAN IDs using ip link configurations.

### B. Non-Functional Requirements
*   NFR-4.2.1: Dynamic interface creation must not cause any packet loss or latency spikes on existing node networks.
*   NFR-4.2.2: Reject unauthorized MAC spoofing attempts using bridge firewall rules (ebtables/nftables).

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/network/bridge.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/network/bridge.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a request to attach a VM to tenant VLAN 100
    *   **When** When running the network setup loop
    *   **Then** Then a TAP device is created, tagged with VLAN 100, and bridged to the physical interface.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd network_bridge_tests`
*   **Integration Tests:** Run `cargo test --test vlan_isolation_verification`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
