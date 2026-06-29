# Epic: Out-of-Band Fencing & HA

*   **Status:** Planned
*   **Epic ID:** `EPIC-06`
*   **Target Roadmap Stage:** Stage 6: Out-of-Band Fencing & HA
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic implements high availability and safety measures for Project Aether. In the event of a blade crash or network partition, the Aggregator must ensure that the failing node is completely cut off from resources (fenced) before its VMs are re-auctioned. This epic develops the `aether-fence` controller using HPE iLO 5 Redfish REST interfaces to execute Shoot The Other Node In The Head (STONITH) power resets. It also integrates `zrepl` for 5-minute RPO storage replication.

## 2. User Stories

- [ ] `[STORY-06.1]` [iLO 5 Redfish API Client & Power Command Driver](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/story_06_1_redfish_ilo.md) - **Status:** Draft
- [ ] `[STORY-06.2]` [Reconciler Deadman Switch Heartbeat & Failover Monitoring](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/story_06_2_deadman_switch.md) - **Status:** Draft
- [ ] `[STORY-06.3]` [STONITH Fencing Execution Workflow](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/story_06_3_stonith_workflow.md) - **Status:** Draft
- [ ] `[STORY-06.4]` [Asynchronous ZFS Volume Replication (zrepl integration)](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/story_06_4_zfs_replication.md) - **Status:** Draft
- [ ] `[STORY-06.5]` [Kubernetes Management Plane Recovery & Auto-Discovery Sync](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/story_06_5_k8s_recovery_discovery.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aether-fence/` (Redfish client library, iLO auth, power control execution)
    *   `crates/aether-aggregator/` (Heartbeat listener, failover coordinator, fencing dispatcher)
*   **CRD / API Boundaries:**
    *   `AetherVirtualDeployment` CRD finalizers (`finalizers.compute.aether.infra`) ensuring clean teardown.
*   **Core Traits:**
    *   `ChassisManager` (Abstracts OOB power reset commands).

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-04` (Storage Slicing & Net Tagging)
*   **Hardware/Environment Requirements:**
    *   Physical access to HPE iLO 5 controllers over a dedicated management network (VLAN 999).
*   **Third-Party Libraries:**
    *   `reqwest` (asynchronous REST client).
    *   `serde_json` (JSON serialization).

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Heartbeats must be token-signed to prevent spoofing from partitioned nodes.
*   **Secrets Handling:** iLO management credentials must be securely encrypted and injected at runtime.
*   **Hardware Safety:** Strictly enforces "fencing before VM re-auctioning." Fencing commands must report a confirmed shutdown before any secondary node mounts the ZVOL to prevent split-brain filesystem corruption.

## 6. Epic Acceptance Criteria

1. `aether-fence` executes a hard power-off command on an HPE iLO 5 BMC slot.
2. A network partition triggers the Aggregator's deadman monitor (15-second heartbeat timeout).
3. The Aggregator triggers the fencing command and receives verification of slot shutdown.
4. The Aggregator re-auctions and restarts the fenced VM on a new blade host using the latest replicated ZVOL snapshot.
5. The Aggregator recovers from a control plane crash by performing a discovery sync and reconciling K8s CRD statuses without disrupting running guest VM processes.
