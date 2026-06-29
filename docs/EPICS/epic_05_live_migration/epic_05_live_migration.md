# Epic: Live Migration & Auto-Convergence

*   **Status:** Planned
*   **Epic ID:** `EPIC-05`
*   **Target Roadmap Stage:** Stage 5: Live Migration & Auto-Convergence
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic delivers a key enterprise capability: live migration of running virtual machines between blades in the chassis without guest downtime. It achieves this for QEMU-KVM virtual machines using block-level storage replication (QEMU `drive-mirror` + NBD) and concurrent memory pre-copy over TCP migration sockets. It also implements guest vCPU auto-convergence to guarantee migration completion under write-heavy loads.

## 2. User Stories

- [ ] `[STORY-05.1]` [QEMU Migration Socket Server & Client Handshake](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_05_live_migration/story_05_1_migration_socket.md) - **Status:** Draft
- [ ] `[STORY-05.2]` [Block Level Replication (Drive Mirroring over NBD)](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_05_live_migration/story_05_2_block_replication.md) - **Status:** Draft
- [ ] `[STORY-05.3]` [Asynchronous Memory Pre-copy Transfer](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_05_live_migration/story_05_3_memory_transfer.md) - **Status:** Draft
- [ ] `[STORY-05.4]` [Auto-Converge vCPU Throttling for Write-Heavy VMs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_05_live_migration/story_05_4_auto_converge.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aetherd/` (Compiles QMP migration commands, opens NBD ports, manages TCP block streams)
    *   `crates/aether-aggregator/` (Coordinates migration target nodes and verifies telemetry)
*   **CRD / API Boundaries:**
    *   `AetherVirtualDeployment` Status field updates tracking active migration progress (e.g. `MigrationState: Mirroring`).
*   **Core Traits:**
    *   None directly, builds upon hypervisor execution controls.

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-04` (Storage Slicing & Net Tagging)
*   **Hardware/Environment Requirements:**
    *   10Gb Virtual Connect midplane interconnect active to guarantee migration bandwidth.
    *   Target host must match CPU instructions or utilize compatibility flags.
*   **Third-Party Libraries:**
    *   QMP (QEMU Machine Protocol) client integrations.

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Migration traffic must be encrypted. Aether will route memory and block transfer over TLS-secured TCP sockets.
*   **Secrets Handling:** Ephemeral single-use authentication tokens must be exchanged before opening migration ports on the destination host.
*   **Hardware Safety:** If migration fails, the source VM must remain active, ensuring zero guest downtime or data corruption.

## 6. Epic Acceptance Criteria

1. Active memory and block states migrate from a source blade to a destination blade.
2. Guest packet drop or VM freeze during the final switchover phase is `< 1 second`.
3. Auto-convergence successfully throttles guest CPU writes to ensure migration convergence when guest writes exceed network capacity.
4. Storage backing block maps on ZFS are correctly updated post-migration.
