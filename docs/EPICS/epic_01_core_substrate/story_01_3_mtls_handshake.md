# Story: Secure mTLS Server/Client Handshakes

*   **Status:** Draft
*   **Story ID:** `STORY-01.3`
*   **Parent Epic:** [EPIC-01: Core API & Rust Substrate](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_01_core_substrate/epic_01_core_substrate.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a security engineer
I want enforce Mutual TLS (mTLS) with client certificate verification on all gRPC connections
So that only authenticated physical blade nodes can communicate with the Aggregator control loop
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-1.3.1: Enforce TLS configuration requiring client certificate verification (RequireAndVerifyClientCert).
*   FR-1.3.2: Load certificate authority (CA) certificates, server keys, and client certs from designated directory paths.

### B. Non-Functional Requirements
*   NFR-1.3.1: All handshake attempts using self-signed or expired certificates must be blocked and log security warnings.
*   NFR-1.3.2: TLS verification checks must add less than 10ms of latency to initial socket establishment.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-auth`
*   **Target Files:**
*   [crates/aether-auth/src/mtls.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-auth/src/mtls.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a gRPC client attempting connection without a valid CA-signed certificate
    *   **When** When performing the TLS handshake with the Aggregator
    *   **Then** Then the connection must be immediately terminated with a TLS handshake error.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-auth mtls_handshake_tests`
*   **Integration Tests:** Run `cargo test --test mtls_client_server_integration`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
