# Contributing to Project Aether

Thank you for your interest in contributing to Project Aether! This document outlines our development workflow, repository structure, and guidelines for adding code.

---

## 1. Repository Layout

Aether is structured as a single Rust Cargo workspace containing all the necessary components. Before submitting pull requests, please familiarize yourself with the layout:

```directory
.
├── Cargo.toml                      # Root Cargo workspace configuration
├── ARCHITECTURE.md                 # Deep-dive on structural design patterns
├── README.md                       # High-level overview & get-started guide
├── docs/                           # Architectural discussions & PRDs
│   └── background/                 # Historical specs and design evaluations
├── proto/                          # Protocol Buffer gRPC contracts
│   └── aether.proto                # Bid, control, and telemetry schema
├── crates/                         # Compiled Rust workspaces
│   ├── aether-aggregator/          # Kubernetes Operator & Bidding Convergence loop
│   ├── aetherd/                    # Bare-metal local agent daemon
│   ├── aether-auth/                # Mutual-TLS & Ephemeral token handshakes
│   ├── aether-fence/               # STONITH HPE iLO 5 / Redfish integration
│   └── pact-mock-server/           # Standalone mock server for physical chassis APIs
```

### Crate Roles
*   **`aether-aggregator`:** The central Kubernetes controller. If you are working on the reverse-bidding coordinator, tie-breaker metrics, or FluxCD CRD synchronizations, this is where the logic resides.
*   **`aetherd`:** The bare-metal node agent daemon. Work here if you are editing Firecracker execution loops, QEMU-KVM parameters, local Linux telemetry checks, or Cloud-Init ISO generation.
*   **`aether-auth`:** Cryptographic identity library. Updates to mutual-TLS (mTLS) bootstrapping or ephemeral token generations should be made in this crate.
*   **`aether-fence`:** Out-of-band fencing integration. Focus here if you are expanding Redfish REST client coverage for hardware vendors (e.g., HPE iLO 5).
*   **`pact-mock-server`:** Standalone mock server simulating physical chassis REST APIs (such as HPE OneView). Work here if you are adding new vendor API contract mocks (e.g., Dell SmartFabric REST specs) or expanding chassis endpoint mocks.

---

## 2. Development Workflow

1.  **Fork the Repository:** Create your own branch from the main branch.
2.  **Define Protocol Buffer Changes First:** If your change modifies the communications between the Aggregator and node daemons, update [aether.proto](file:///Users/casibbald/Workspace/remote/microscaler/Aether/proto/aether.proto) first and ensure it compiles successfully.
3.  **Run Quality Checks:** Ensure code conforms to Rust styling standards:
    ```bash
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    ```
4.  **Validate Chassis Integration Changes via Pact Contracts:** If you modify `MidplaneNetworkManager` clients (such as the HPE Virtual Connect client), you must run and update the Pact contract integration tests in `crates/aether-aggregator/tests/switch_vlan_tagging_integration.rs` to ensure contract validity.
5.  **Submit a Pull Request (PR):** Make sure your PR contains a detailed description matching the architectural guidelines in [ARCHITECTURE.md](file:///Users/casibbald/Workspace/remote/microscaler/Aether/ARCHITECTURE.md).
