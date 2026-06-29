# Epic Template: [Epic Name]

*   **Status:** [Draft | In Progress | Complete]
*   **Epic ID:** `EPIC-XX`
*   **Target Roadmap Stage:** [e.g., Stage 3: Dual Hypervisor Engine]
*   **Owner:** [@username]

---

## 1. Description & Context

Provide a high-level explanation of the problem this Epic solves, the value it brings, and how it aligns with the overall vision of Project Aether (e.g., replacing VMware, enabling decentralized reverse-bidding).

## 2. User Stories

The following checklist tracks individual stories within this Epic:

- [ ] `[STORY-01]` [Story Title: As a... I want to...](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/story_template.md) - **Status:** [Draft]
- [ ] `[STORY-02]` [Story Title: As a... I want to...](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/story_template.md) - **Status:** [Draft]

## 3. Technical Design & Architecture Constraints

Explain the technical design strategy for this Epic. Reference relevant system components, traits, and files:
*   **Crate Targets:** [e.g., `crates/aether-aggregator/`, `crates/aetherd/`]
*   **CRD / API Boundaries:** [e.g., changes to `AetherVirtualDeployment` CRD or `aether.proto`]
*   **Core Traits:** [e.g., `ChassisManager`, `MidplaneNetworkManager`]

## 4. Dependencies

Detail any dependencies required before starting or completing this Epic:
*   **Upstream Epics/Stories:** [e.g., `EPIC-01`]
*   **Hardware/Environment Requirements:** [e.g., HPE iLO 5 REST access, ZFS on host NVMe]
*   **Third-Party Libraries:** [e.g., `tokio`, `tonic`, `zfs-core`]

## 5. Security, Attestation & Safety

Specify any security or safety considerations that must be met:
*   **mTLS and Attestation:** Do we need to sign/attest any messages?
*   **Secrets Handling:** How are private keys or credentials managed (e.g., tmpfs ISO)?
*   **Hardware Safety:** Are there fencing/STONITH implications?

## 6. Epic Acceptance Criteria

Global rules that must be satisfied to consider this Epic complete:
1. All child stories are completed and verified.
2. Code conforms to Aether Rust style standards (`cargo clippy --workspace --all-targets -- -D warnings`).
3. Resident memory footprint limits are verified (e.g., `aetherd` RSS `< 15MB`).
4. End-to-end integration tests pass on target blade environments.
