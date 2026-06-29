# Epic: Dual Hypervisor Engine

*   **Status:** In Progress
*   **Epic ID:** `EPIC-03`
*   **Target Roadmap Stage:** Stage 3: Dual Hypervisor Engine
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic implements the hypervisor execution controls in the `aetherd` daemon. Aether supports a dual execution plane: launching lightweight, ephemeral Firecracker microVMs on the Compute Pool (Slots 1-8), and launching full QEMU-KVM virtual machines on the Infrastructure Pool (Slots 9-16). This epic also handles dynamically compiling Cloud-Init metadata into a custom NoCloud ISO.

## 2. User Stories

- [ ] `[STORY-03.1]` [Firecracker Process Orchestration & Console Routing](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/story_03_1_firecracker_process.md) - **Status:** Draft
- [ ] `[STORY-03.2]` [Firecracker VSOCK Integration](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/story_03_2_firecracker_vsock.md) - **Status:** Draft
- [ ] `[STORY-03.3]` [QEMU-KVM Command Builder & Execution Loop](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/story_03_3_qemu_kvm.md) - **Status:** Draft
- [ ] `[STORY-03.4]` [Dynamic NoCloud Cloud-Init ISO Builder in memory (tmpfs)](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_03_dual_hypervisor/story_03_4_cloud_init_iso.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aetherd/` (Orchestrates child processes, configures network taps, builds commands, generates ISOs)
*   **CRD / API Boundaries:**
    *   `AetherVirtualDeployment` CRD fields for hypervisor profiles (Compute/Firecracker vs. Infrastructure/QEMU).
*   **Core Traits:**
    *   `Hypervisor`: Exposes uniform methods to `spawn`, `stop`, and `query_status` across both Firecracker and QEMU-KVM.

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-02` (Stateless Reverse-Bidding)
*   **Hardware/Environment Requirements:**
    *   Worker blades must support KVM (`/dev/kvm` accessible).
    *   Firecracker binary and QEMU system utilities installed on nodes.
    *   `xorriso` or `mkisofs` CLI tools for compiling ISO images on the fly.
*   **Third-Party Libraries:**
    *   `tokio::process` (for asynchronous subprocess lifecycle control).
    *   `tempfile` (for temporary `tmpfs` mounts).

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Decrypted user-data secrets are passed to `aetherd` securely over mTLS.
*   **Secrets Handling:** Cloud-Init configuration is built directly inside a `tmpfs` memory path to avoid secrets touching permanent disks.
*   **Hardware Safety:** Strictly prevents nested virtualization configurations by executing directly on the bare-metal Linux host.

## 6. Epic Acceptance Criteria

1. `aetherd` spawns a Firecracker microVM running a guest OS.
2. `aetherd` spawns a QEMU-KVM VM.
3. The guest VM successfully mounts the compiled NoCloud ISO and executes its Cloud-Init script.
4. Guest VM memory and CPU allocation parameters are correctly applied via hypervisor flags.
