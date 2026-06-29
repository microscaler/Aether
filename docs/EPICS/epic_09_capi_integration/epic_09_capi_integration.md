# Epic: Cluster API (CAPI) Integration

*   **Status:** Planned
*   **Epic ID:** `EPIC-09`
*   **Target Roadmap Stage:** Stage 9: Cluster API (CAPI) Integration
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic future-proofs Project Aether for Cluster API integration. It defines the structural boundaries, API specs, and status reporting required to implement `cluster-api-provider-aether`. By utilizing a Kubernetes-native model where the CAPI provider manages Aether's `AetherVirtualDeployment` CRDs, CAPI can leverage Aether's decentralized Reverse-Bidding scheduler, Virtual Connect networking, ZFS storage, and HPE iLO fencing.

## 2. User Stories

- [ ] `[STORY-09.1]` [Cloud-Init Injection & Base64 Metadata Handshake](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/story_09_1_cloud_init_injection.md) - **Status:** Draft
- [ ] `[STORY-09.2]` [Firecracker MMDS Bootstrap Configuration Delivery](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/story_09_2_firecracker_mmds.md) - **Status:** Draft
- [ ] `[STORY-09.3]` [Guest IP Address Discovery (DHCP Snooping & Guest Agent Queries)](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/story_09_3_ip_discovery.md) - **Status:** Draft
- [ ] `[STORY-09.4]` [Expose Blade Slot Failure Domains in Cluster Status](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_09_capi_integration/story_09_4_failure_domains.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aether-aggregator/` (Exposes Failure Domains, registers VM IPs to CRD status)
    *   `crates/aetherd/` (Sets up Firecracker MMDS, compiles KVM NoCloud ISO, queries QEMU Guest Agent)
*   **CRD / API Boundaries:**
    *   `AetherVirtualDeployment` CRD fields: `spec.userDataSecretRef`, `status.addresses`, `status.providerID`.
*   **Core Traits:**
    *   None.

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-03` (Dual Hypervisor Engine)
    *   `EPIC-04` (Storage Slicing & Net Tagging)
*   **Hardware/Environment Requirements:**
    *   Target guest images compiled with Cloud-Init and `qemu-guest-agent` packages.
*   **Third-Party Libraries:**
    *   QMP communication client libraries in `aetherd`.

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Decrypted user-data secrets containing control plane tokens must be protected during transit over gRPC mTLS.
*   **Secrets Handling:** Dynamic Cloud-Init ISO generation must execute in memory (`tmpfs`) to prevent residual files from staying on host NVMe.
*   **Hardware Safety:** CAPI topology constraints (failure domains) must be respected during bidding to prevent single-blade control plane outages.

## 6. Epic Acceptance Criteria

1. `AetherVirtualDeployment` CRD accepts base64-encoded Cloud-Init user-data and injects it into VMs.
2. Guest VMs boot and configure themselves using the injected Cloud-Init bootstrap data.
3. Node IP address allocations are automatically discovered by `aetherd` and reported to the Aggregator within `60 seconds` of VM boot.
4. Chassis slot topology map is exposed as Failure Domains in `AetherCluster` status.
