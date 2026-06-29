# Story: Local Telemetry Collector

*   **Status:** Draft
*   **Story ID:** `STORY-02.3`
*   **Parent Epic:** [EPIC-02: Stateless Reverse-Bidding & Scheduling](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_02_reverse_bidding/epic_02_reverse_bidding.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a blade daemon
I want query local kernel metrics (CPU loadavg, memory capacity, disk I/O pressure)
So that I can calculate an accurate, real-time bid representing the node's resource availability
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-2.3.1: Parse system parameters from `/proc/loadavg` and `/proc/meminfo`.
*   FR-2.3.2: Gather disk space availability and temperature stats using NVMe S.M.A.R.T. commands.

### B. Non-Functional Requirements
*   NFR-2.3.1: Telemetry parsing must consume less than 1% CPU core time and `< 1MB` of RSS memory.
*   NFR-2.3.2: Metrics queries must complete in under 5ms to avoid holding up the bidding response loop.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aetherd`
*   **Target Files:**
*   [crates/aetherd/src/telemetry.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/telemetry.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a telemetry request from the daemon loop
    *   **When** When querying system files
    *   **Then** Then the current CPU load averages and memory availability bytes must be returned accurately.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aetherd telemetry_parsing_tests`
*   **Integration Tests:** Run `cargo test --test host_system_metrics_query`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
