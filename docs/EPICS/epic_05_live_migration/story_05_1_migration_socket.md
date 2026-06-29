# Story: QEMU Migration Socket Server & Client Handshake

*   **Status:** Draft
*   **Story ID:** `STORY-05.1`
*   **Parent Epic:** [EPIC-05: Live Migration & Auto-Convergence](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_05_live_migration/epic_05_live_migration.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want manage live migration TCP sockets on source and destination hosts
So that destination hosts can verify attestation tokens and prepare to receive VM migration streams
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-5.1.1: Open listening sockets on target nodes to receive incoming memory and block migration streams.
*   FR-5.1.2: Validate source node attestation certificates before accepting connection streams.

### B. Non-Functional Requirements
*   NFR-5.1.1: Enforce TLS encryption on all migration socket connections.
*   NFR-5.1.2: Close and cleanup socket servers if the migration times out or fails.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/migration/socket.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/migration/socket.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a migration command dispatched by the Aggregator
    *   **When** When source and destination nodes establish a migration socket
    *   **Then** Then the handshake succeeds, attestation is verified, and the channel is marked ready for data transfer.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd migration_socket_tests`
*   **Integration Tests:** Run `cargo test --test migration_handshake_security`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
