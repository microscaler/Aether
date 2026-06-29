# Story: Expose Blade Slot Failure Domains in Cluster Status

*   **Status:** Draft
*   **Story ID:** `STORY-09.4`
*   **Parent Epic:** [EPIC-09: Cluster API (CAPI) Integration](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/epic_09_capi_integration.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a Aggregator
I want expose physical chassis slot topology structures as CAPI Failure Domains in AetherCluster resources
So that CAPI schedulers can spread control planes and worker nodes across separate physical blades for HA
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   FR-9.4.1: Parse registered blade details and export them as FailureDomains in `AetherCluster.Status`.
*   FR-9.4.2: Enforce bidding restrictions so that reverse-bid broadcasts are sent only to blades matching CAPI placement request hints.

### B. Non-Functional Requirements
*   NFR-9.4.1: Ensure CAPI control plane anti-affinity constraints are honored during auctions.
*   NFR-9.4.2: Dynamically mark slot failure domains as unavailable if a node daemon fails heartbeats.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
*   [crates/aether-aggregator/src/ha/failure_domains.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/ha/failure_domains.rs)

### B. Detailed Design
*   **Structs / Enums:** Define new data structures relevant to this story.
*   **Traits to Implement:** List any standard or custom traits.
*   **Concurrency / Async:** State any specific Tokio task execution rules or lock considerations.

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** Given a CAPI scheduling constraint targeting 'slot-09'
    *   **When** When initiating the workload auction
    *   **Then** Then the Aggregator only accepts bids submitted by Blade 9 and ignores bids from other hosts.
*   **Criteria 2:** [Specific validation constraint]

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator failure_domain_tests`
*   **Integration Tests:** Run `cargo test --test failure_domain_scheduling_isolation`

### B. Manual Verification
*   **Step 1:** Run node or aggregator mock services.
*   **Step 2:** Inspect logs (e.g. `journalctl -u aetherd -n 100`).
*   **Step 3:** Confirm physical/logical resource state changes.
