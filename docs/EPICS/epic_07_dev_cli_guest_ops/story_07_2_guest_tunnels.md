# Story: Zero-Network Guest Command Tunnels

*   **Status:** Draft
*   **Story ID:** `STORY-07.2`
*   **Parent Epic:** [EPIC-07: Developer CLI & Guest Operations](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/epic_07_dev_cli_guest_ops.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a developer
I want execute shell commands inside guest VMs over local VSOCK or serial agent sockets
So that I can log in and troubleshoot instances even if the guest network interfaces are down or misconfigured
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-7.2.1: Pipe stdin, stdout, and stderr streams directly to Firecracker VSOCK or QEMU guest agent sockets.
*   FR-7.2.2: Establish interactive terminal sessions (pseudo-TTY) into guest VMs.

### B. Non-Functional Requirements
*   NFR-7.2.1: Session connections must utilize secure attestation handshakes before granting guest console access.
*   NFR-7.2.2: Tunnel sessions must run in isolation without exposing TCP/UDP ports on the host system.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/guest/tunnel.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/guest/tunnel.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a guest VM with broken ethernet configurations
    *   **When** When running 'aether shell [vm-name]'
    *   **Then** Then an interactive terminal session opens, and commands run successfully within the guest.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd guest_tunnel_tests`
*   **Integration Tests:** Run `cargo test --test interactive_tty_vsock_tunnel`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
