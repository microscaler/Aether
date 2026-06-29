# Story: Single-Use Ephemeral Attestation Tokens

*   **Status:** Draft
*   **Story ID:** `STORY-01.4`
*   **Parent Epic:** [EPIC-01: Core API & Rust Substrate](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_01_core_substrate/epic_01_core_substrate.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a security engineer
I want generate and validate signed, single-use attestation tokens for RPC requests
So that unauthorized or duplicate control commands cannot be executed on worker blades
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-1.4.1: Generate cryptographically signed (HMAC-SHA256) tokens containing node ID and timestamp.
*   FR-1.4.2: Enforce a strict 60-second expiration window and keep a sliding history window of used tokens to prevent replay attacks.

### B. Non-Functional Requirements
*   NFR-1.4.1: Token validation must complete in `< 1ms` to avoid adding overhead to critical VM control loops.
*   NFR-1.4.2: HMAC key rotation must be handled securely in memory without writing tokens to host disks.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-auth`
*   **Target Files:**
*   [crates/aether-auth/src/token.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-auth/src/token.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a client sending a control command with an expired or replayed token
    *   **When** When validating the request header on the host daemon
    *   **Then** Then the validation must fail and return an Unauthenticated gRPC status code.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-auth token_validation_tests`
*   **Integration Tests:** Run `cargo test --test token_replay_prevention`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
