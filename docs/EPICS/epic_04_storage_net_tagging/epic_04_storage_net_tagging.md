# Epic: Storage Slicing & Net Tagging

*   **Status:** Planned
*   **Epic ID:** `EPIC-04`
*   **Target Roadmap Stage:** Stage 4: Storage Slicing & Net Tagging
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic integrates Project Aether's storage and network fabric layers. On the storage plane, it coordinates ZFS on Linux (ZoL) commands to perform instantaneous snapshot cloning of VM disk templates as thin-provisioned ZVOLs. On the network plane, it handles midplane network switch tagging (e.g. Virtual Connect Flex-10) to tag VLANs for isolated tenant bridges.

## 2. User Stories

- [ ] `[STORY-04.1]` [ZFS ZVOL Provisioning, Snapshots, & Thin-Clone Automation](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/story_04_1_zfs_zvol.md) - **Status:** Draft
- [ ] `[STORY-04.2]` [Dynamic Linux Bridge and VLAN Tag Interface Setup](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/story_04_2_linux_bridge.md) - **Status:** Draft
- [ ] `[STORY-04.3]` [Integration with democratic-csi for Kubernetes persistent volumes](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/story_04_3_democratic_csi.md) - **Status:** Draft
- [ ] `[STORY-04.4]` [Virtual Connect REST Driver for dynamic FlexNIC configuration](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_04_storage_net_tagging/story_04_4_virtual_connect.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aetherd/` (Executes host ZFS and IP/Bridge commands)
    *   `crates/aether-aggregator/` (VLAN allocation coordination, CSI controller interactions)
*   **CRD / API Boundaries:**
    *   `AetherVirtualDeployment` CRD fields for ZFS storage pools, thin provisioning limits, and VLAN network interfaces.
*   **Core Traits:**
    *   `MidplaneNetworkManager` (Abstracts Virtual Connect or MX7000 VLAN tagging commands).

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-03` (Dual Hypervisor Engine)
*   **Hardware/Environment Requirements:**
    *   Storage blades (Slots 9-16) configured with ZFS zpools.
    *   HPE Virtual Connect switch modules for actual midplane network tests.
*   **Third-Party Libraries:**
    *   `libzfs-core` or safe process wrappers to ZFS CLI.
    *   `rtnetlink` (for programmatically creating bridges and TAP interfaces in Rust).

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Only attestation-signed daemons can execute storage deletions or modifications.
*   **Secrets Handling:** Switch admin credentials (Virtual Connect REST API tokens) must be stored in Kubernetes Secrets and decrypted via SOPS/Sealed Secrets.
*   **Hardware Safety:** Strictly configures ZVOL user quotas and ZFS ARC cache ceilings to prevent memory starvation and host disk OOMs.

## 6. Epic Acceptance Criteria

1. `aetherd` creates thin-cloned ZVOLs from a base ZFS snapshot in under `500ms`.
2. Host network interfaces are successfully tagged and attached to guest VM bridges (`br-tenant`).
3. Midplane network switch configurations are tagged with appropriate tenant VLANs via Virtual Connect REST drivers.
4. Kubernetes persistent volume claims map dynamically to ZFS storage backend volumes.
