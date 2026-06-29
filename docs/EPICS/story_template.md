# Story Template: [Story Name]

*   **Status:** [Draft | Ready | In Progress | Done]
*   **Story ID:** `STORY-XX`
*   **Parent Epic:** [e.g., EPIC-01: Core API & Rust Substrate]
*   **Estimation:** [e.g., 3 Story Points / T-Shirt Size]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a [User Role / System Component]
I want [Capability / Action]
So that [Business Value / Result]
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   **[FR-XX]:** [Describe the functional requirement - what the system MUST do under specific inputs/conditions.]
*   **[FR-XX]:** [Another functional requirement.]

### B. Non-Functional Requirements
*   **[NFR-XX]:** [Describe the non-functional requirement - performance constraints, memory footprint limits, security/mTLS controls, safety protocols.]
*   **[NFR-XX]:** [Another non-functional requirement.]

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/[crate_name]`
*   **Target Files:**
    *   [filename.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/...) (L10-L40): Detail specific function/struct changes.
    *   [aether.proto](file:///Users/casibbald/Workspace/remote/microscaler/Aether/proto/aether.proto): Proto changes if applicable.

### B. Detailed Design
*   **Structs / Enums:** Define new data structures.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** [preconditions]
    *   **When** [action occurs]
    *   **Then** [expected outcome]
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Commands to run unit tests (e.g. `cargo test --lib [test_name]`).
*   **Integration Tests:** Commands to run integration tests (e.g. `cargo test --test [test_file]`).

### B. Manual Verification
*   **Step 1:** Run [command or script].
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm state (e.g. query database or ZFS pool).
