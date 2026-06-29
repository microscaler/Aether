# Epic: Developer CLI & Guest Operations

*   **Status:** Planned
*   **Epic ID:** `EPIC-07`
*   **Target Roadmap Stage:** Stage 7: Developer CLI & Guest Operations
*   **Owner:** [@username]

---

## 1. Description & Context

This Epic implements developer-focused tools and guest VM inspection commands, matching the Multipass developer experience. It compiles the `aether` CLI tool for shell commands and provides direct guest access tunnels (`aether shell` and `aether exec`) that bypass network interfaces by routing commands over Firecracker VSOCK or QEMU-KVM guest agent serial connections. It also configures directory sharing via VirtioFS.

## 2. User Stories

- [ ] `[STORY-07.1]` [Developer CLI tool (aether) Command Parser](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/story_07_1_cli_parser.md) - **Status:** Draft
- [ ] `[STORY-07.2]` [Zero-Network Guest Command Tunnels (exec & shell via VSOCK/serial)](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/story_07_2_guest_tunnels.md) - **Status:** Draft
- [ ] `[STORY-07.3]` [Local Host Path Passthrough Mounts via VirtioFS](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/story_07_3_virtiofs_mounts.md) - **Status:** Draft
- [ ] `[STORY-07.4]` [Local VM Image Operations (Pull, Build, Push)](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_07_dev_cli_guest_ops/story_07_4_image_ops.md) - **Status:** Draft

## 3. Technical Design & Architecture Constraints

*   **Crate Targets:**
    *   `crates/aether/` (Command CLI binary client)
    *   `crates/aetherd/` (Exposes local socket endpoints and routes VSOCK streams)
*   **CRD / API Boundaries:**
    *   `AetherVirtualDeployment` CRD fields for VirtioFS directory sharing specs.
*   **Core Traits:**
    *   None.

## 4. Dependencies

*   **Upstream Epics/Stories:**
    *   `EPIC-03` (Dual Hypervisor Engine)
*   **Hardware/Environment Requirements:**
    *   Target guest images must have QEMU guest agent installed.
    *   VirtioFS support enabled in the host Linux kernel.
*   **Third-Party Libraries:**
    *   `clap` (command-line argument parser).
    *   `tokio-vsock` (VSOCK socket connection helper).

## 5. Security, Attestation & Safety

*   **mTLS and Attestation:** Exec and shell requests must authenticate using client-side identity tokens before accessing VM guest sockets.
*   **Secrets Handling:** Shared directory permissions (VirtioFS) must be strictly mapped to tenant UIDs to prevent guest VMs from accessing host system files.
*   **Hardware Safety:** Directory mounts must default to Read-Only unless explicitly configured as Read-Write by tenant policies.

## 6. Epic Acceptance Criteria

1. The `aether` CLI tool compiled and working on developer workstations.
2. `aether shell` establishes an interactive TTY session into a guest VM over VSOCK/serial, bypassing network ports.
3. `aether exec` runs a non-interactive shell command and returns output logs.
4. Host-side folders are mounted inside guest VMs via VirtioFS and respect read/write security boundaries.
