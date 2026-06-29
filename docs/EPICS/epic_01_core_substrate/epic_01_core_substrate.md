# Epic: Core API & Rust Substrate

*   **Status:** Active
*   **Epic ID:** `EPIC-01`
*   **Target Roadmap Stage:** Stage 1: Core API & Rust Substrate
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic establishes the foundational Rust Cargo workspace structure and defines the core communication interfaces between the central Aether Aggregator and the worker blade daemons (`aetherd`). This includes establishing type-safe, compiled protobuf gRPC contracts and securing all communications over Mutual TLS (mTLS).

## 2. User Stories

- [ ] `[STORY-01.1]` [Workspace Setup & Crate Layout](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_01_core_substrate/story_01_1_workspace_setup.md) - **Status:** Draft
- [ ] `[STORY-01.2]` [Define & Compile aether.proto Schemas](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_01_core_substrate/story_01_2_proto_schema.md) - **Status:** Draft
- [ ] `[STORY-01.3]` [Secure mTLS Server/Client Handshakes](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_01_core_substrate/story_01_3_mtls_handshake.md) - **Status:** Draft
- [ ] `[STORY-01.4]` [Single-Use Ephemeral Attestation Tokens](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_01_core_substrate/story_01_4_attestation.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aether-aggregator/` (K8s Operator logic & tokio gRPC server)
    *   `crates/aetherd/` (Bare-metal agent node & gRPC client)
    *   `crates/aether-auth/` (Shared library for mTLS configs & attestation helper)
*   **CRD / API Boundaries:**
    *   Initial `aether.proto` contract specifying basic bid requests, telemetry status reports, and control commands.
*   **Core Traits:**
    *   `Attestor`: Decouples attestation signing and verification strategies.

## 4. Dependencies

*   **Upstream Epics/Stories:** None (Foundational Epic).
*   **Hardware/Environment Requirements:**
    *   Utility Kubernetes cluster for running the Aggregator.
    *   A test blade machine to host the daemon.
    *   Valid certificate authority (CA) certificates or mock certs for local validation.
*   **Third-Party Libraries:**
    *   `tonic` & `prost` (gRPC and Protocol Buffers compiler).
    *   `rcgen` (runtime certificate generation for testing).
    *   `tokio` (asynchronous runtime).

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** All gRPC sockets must enforce client and server certificate validation. The `aether-auth` crate will compile credentials statically or read them from a mount path.
*   **Secrets Handling:** Ephemeral tokens must be cryptographically signed (e.g. HMAC-SHA256) and expire within a strict 60-second window.
*   **Hardware Safety:** No OOB power commands are executed in this phase.

## 6. Epic Acceptance Criteria

1. Rust workspace builds clean without compile or clippy warnings.
2. The `aether-aggregator` starts up as a Kubernetes Operator, exposing the gRPC control port.
3. `aetherd` establishes a verified mTLS handshake connection with the Aggregator.
4. Telemetry and control messages successfully flow over the mTLS gRPC channel, validated by unit tests.
