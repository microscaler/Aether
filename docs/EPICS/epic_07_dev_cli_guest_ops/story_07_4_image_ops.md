# Story: Local VM Image Operations

*   **Status:** Draft
*   **Story ID:** `STORY-07.4`
*   **Parent Epic:** [EPIC-07: Developer CLI & Guest Operations](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/epic_07_dev_cli_guest_ops.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a developer
I want pull, build, and extract OCI container images containing VM kernel and root file systems
So that VM templates can be built using standard Dockerfiles and cached locally for instant startup times
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-7.4.1: Pull OCI images from registries and extract their kernel/filesystem layers.
*   FR-7.4.2: Interface with containerd on nodes to cache image layers locally.

### B. Non-Functional Requirements
*   NFR-7.4.1: Image parsing and extraction to ZFS volumes must complete in under 2 seconds for cached layers.
*   NFR-7.4.2: Image downloads must use mirror registries over 10Gb Virtual Connect midplanes to avoid WAN bottlenecks.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether`
*   **Target Files:**
*   [crates/aether/src/image.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether/src/image.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a custom OS OCI image in the local registry mirror
    *   **When** When pulling and extracting the image
    *   **Then** Then the kernel file and root filesystems are correctly written to ZVOL targets.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether image_layer_tests`
*   **Integration Tests:** Run `cargo test --test oci_image_extraction_to_zfs`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
