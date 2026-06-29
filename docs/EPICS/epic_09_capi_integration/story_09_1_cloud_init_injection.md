# Story: Cloud-Init Injection & Base64 Metadata Handshake

*   **Status:** Draft
*   **Story ID:** `STORY-09.1`
*   **Parent Epic:** [EPIC-09: Cluster API (CAPI) Integration](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/epic_09_capi_integration.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator
I want accept base64-encoded Cloud-Init user-data in AetherVirtualDeployment specifications
So that Kubernetes bootstrap provider secrets (CABPK) can flow dynamically into the Aether deployment loop
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-9.1.1: Parse and validate `userDataSecretRef` configurations in `AetherVirtualDeployment` specs.
*   FR-9.1.2: Decrypt K8s secret payloads, encode them to base64, and pack them into the node's auction dispatch payload.

### B. Non-Functional Requirements
*   NFR-9.1.1: Base64 payload decoding must support data sizes up to 64KB.
*   NFR-9.1.2: Reject deployment actions immediately if the referenced bootstrap secret is missing or unreadable.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/controller/deployment.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/controller/deployment.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given an AetherVirtualDeployment CR specifying a valid bootstrap secret name
    *   **When** When the Aggregator reconciles the deployment
    *   **Then** Then the bootstrap script is read, decrypted, and sent as part of the execution spec to the worker node.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator deployment_controller_tests`
*   **Integration Tests:** Run `cargo test --test secret_decryption_and_dispatch`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
