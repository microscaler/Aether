# Story: Developer CLI Tool (aether) Command Parser

*   **Status:** Draft
*   **Story ID:** `STORY-07.1`
*   **Parent Epic:** [EPIC-07: Developer CLI & Guest Operations](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/epic_07_dev_cli_guest_ops.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a developer
I want interact with Project Aether APIs using a compiled CLI tool binary (aether)
So that I can deploy, delete, query, and troubleshoot virtual machines easily from my local shell
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-7.1.1: Parse commands for vm launch, destroy, list, status, shell, and config.
*   FR-7.1.2: Connect and communicate with the Aggregator gRPC endpoint over authenticated TLS channels.

### B. Non-Functional Requirements
*   NFR-7.1.1: CLI command parser and initial startup latency must be `< 50ms`.
*   NFR-7.1.2: Compile the CLI tool as a single statically linked binary with no dynamic external library dependencies.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether`
*   **Target Files:**
*   [crates/aether/src/main.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether/src/main.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a developer running 'aether list' from their local terminal
    *   **When** When the CLI tool connects to the Aggregator
    *   **Then** Then a formatted table listing active VM names, IPs, and states is returned.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether cli_parser_tests`
*   **Integration Tests:** Run `cargo test --test cli_api_integration`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
