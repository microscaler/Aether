# Story: Firecracker VSOCK Integration

*   **Status:** Draft
*   **Story ID:** `STORY-03.2`
*   **Parent Epic:** [EPIC-03: Dual Hypervisor Engine](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/epic_03_dual_hypervisor.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want establish VSOCK communications with Firecracker guest microVMs
So that I can configure and monitor guest VMs securely without assigning public IP addresses
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-3.2.1: Configure host-side vsock devices and bind local sockets to guest ports.
*   FR-3.2.2: Establish bidirectional stream channels for guest command execution and telemetry reporting.

### B. Non-Functional Requirements
*   NFR-3.2.1: VSOCK communication channels must enforce TLS encryption for all data packets.
*   NFR-3.2.2: Ensure VSOCK data throughput supports at least 100MB/s.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/vsock.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/vsock.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a running Firecracker guest configured with VSOCK
    *   **When** When sending command payloads to guest port 1024 from the host daemon
    *   **Then** Then the guest receives the payload and executes it, returning logs back over the VSOCK stream.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd vsock_connection_tests`
*   **Integration Tests:** Run `cargo test --test vsock_stream_performance`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
