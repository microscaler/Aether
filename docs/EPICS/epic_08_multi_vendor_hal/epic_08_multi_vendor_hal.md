# Epic: Multi-Vendor Hardware Abstraction

*   **Status:** Planned
*   **Epic ID:** `EPIC-08`
*   **Target Roadmap Stage:** Stage 8: Multi-Vendor Hardware Abstraction
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic decouples Aether's core scheduling and fencing controller from vendor-specific blade chassis protocols. It abstracts power resets and VLAN switch tagging operations behind pluggable Rust traits (`ChassisManager` and `MidplaneNetworkManager`). While initial drivers target HPE iLO 5 and HPE Virtual Connect, this Epic delivers Dell iDRAC, Dell SmartFabric, and Lenovo Flex switch driver implementations.

## 2. User Stories

- [ ] `[STORY-08.1]` [ChassisManager Dell iDRAC Redfish API Driver](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/story_08_1_dell_idrac.md) - **Status:** Draft
- [ ] `[STORY-08.2]` [MidplaneNetworkManager Dell SmartFabric Services (SFS) Driver](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/story_08_2_dell_smartfabric.md) - **Status:** Draft
- [ ] `[STORY-08.3]` [ChassisManager Generic Redfish Compliance Driver](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/story_08_3_generic_redfish.md) - **Status:** Draft
- [ ] `[STORY-08.4]` [MidplaneNetworkManager Lenovo Flex System CLI Driver](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_08_multi_vendor_hal/story_08_4_lenovo_flex.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aether-fence/` (Implements new Dell/Generic ChassisManager drivers)
    *   `crates/aether-aggregator/` (Loads and utilizes new MidplaneNetworkManager drivers)
*   **CRD / API Boundaries:**
    *   `AetherCluster` Spec fields defining chassis vendor types (e.g. `vendor: Dell`).
*   **Core Traits:**
    *   `ChassisManager` (Fencing power controls)
    *   `MidplaneNetworkManager` (Midplane switch VLAN configuration)

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-06` (OOB Fencing & HA)
*   **Hardware/Environment Requirements:**
    *   Access to Dell PowerEdge MX7000 or FX2 chassis controllers for driver validation.
*   **Third-Party Libraries:**
    *   DMTF Redfish baseline API client libraries.

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Drivers must enforce secure TLS validation when querying vendor BMC endpoints.
*   **Secrets Handling:** Dell iDRAC/Lenovo SSH credentials must be stored and accessed via Kubernetes Secrets.
*   **Hardware Safety:** Strictly verifies power-off command completion. Redfish APIs must return HTTP 200/204 and state changes must be polled to guarantee physical shutdown.

## 6. Epic Acceptance Criteria

1. Dell iDRAC driver executes power reset cycles, conforming to the `ChassisManager` trait contracts.
2. Generic Redfish driver successfully manages power status on non-HPE/non-Dell BMC targets.
3. Dell SmartFabric driver provisions VLAN tagging on MX chassis interfaces.
4. ChassisManager and MidplaneNetworkManager traits are loaded dynamically based on cluster configuration file parameters.
