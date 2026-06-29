# Story: Guest IP Address Discovery

*   **Status:** Draft
*   **Story ID:** `STORY-09.3`
*   **Parent Epic:** [EPIC-09: Cluster API (CAPI) Integration](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/epic_09_capi_integration.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want discover guest VM IP address allocations using DHCP lease snooping and QEMU Guest Agent queries
So that the Aggregator and CAPI can discover node IP addresses to verify node health and connectivity
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-9.3.1: Query DHCP lease files and monitor ARP bridge packets to snoop guest IP addresses.
*   FR-9.3.2: Query the QEMU Guest Agent serial socket using guest-network-get-interfaces commands on persistent VMs.

### B. Non-Functional Requirements
*   NFR-9.3.1: Discover and report guest IPs to the Aggregator within 60 seconds of guest OS boot.
*   NFR-9.3.2: Guest agent communication socket timeouts must be capped at 2 seconds.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/network/discovery.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/network/discovery.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a persistent VM booting with DHCP configured
    *   **When** When the guest agent starts up inside the VM
    *   **Then** Then aetherd queries the agent socket, retrieves the IP, and updates the AetherVirtualDeployment status.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd guest_ip_discovery_tests`
*   **Integration Tests:** Run `cargo test --test agent_network_discovery_integration`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
