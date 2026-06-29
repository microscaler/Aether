# Developer CLI Experience: Multipass-Inspired Aether CLI

This document outlines the design and functional contracts for the **Aether Command-Line Interface (`aether` CLI)**. While Project Aether is an enterprise-focused multi-tenant compute plane, it adopts the developer-centric workflow and rapid loop patterns of **Canonical Multipass** to allow operators and platform engineers to quickly spin up, access, and manage virtual machines.

---

## 1. The Developer-to-GitOps Gateway

Aether's core engine is driven declaratively by GitOps (CRDs synced via FluxCD). To prevent this from slowing down developers, the `aether` CLI acts as a real-time wrapper that generates, applies, and monitors these declarative resources under the hood:

```
                            [ aether launch ]
                                    │
                                    ▼ (CLI Helper)
                  [ Generates AetherVirtualDeployment YAML ]
                                    │
                       ┌────────────┴────────────┐
                       ▼ (Developer Loop)        ▼ (Production Loop)
                [ Apply to K8s API ]       [ Commit to GitOps Repo ]
```

---

## 2. Core CLI Commands & Functional Design

Aether adopts the most successful commands from the Multipass CLI, adapting them to run across our decentralized cluster:

### A. Workload Lifecycle Commands

*   **`aether launch`**
    *   *Syntax:* `aether launch <image> --name <name> --cpus <n> --mem <size> --cloud-init <file>`
    *   *Contract:* Validates local parameters, generates an `AetherVirtualDeployment` CRD, and applies it to the active namespace. The Aggregator receives the CRD, triggers the 250ms reverse-bidding auction, and provisions the VM.
*   **`aether list` / `aether info`**
    *   *Contract:* Queries the central Aggregator's in-memory registry. `list` provides a tabular status overview of all VMs in the tenant namespace. `info` outputs detailed JSON/YAML metadata (IP addresses, active host blade slot, storage volume configurations, and boot times).
*   **`aether stop` / `aether start` / `aether restart`**
    *   *Contract:* Updates the `desiredState` field of the matching CRD (`Active` / `Stopped`). The change is reconciled via gRPC down to the hosting blade daemon (`aetherd`), which sends ACPI shutdown or execution start commands to QEMU/Firecracker.

### B. In-Guest Execution (Zero-Network Access)

Multipass allows users to access guest environments instantly. Aether implements this securely without exposing public SSH ports on VM interfaces:

*   **`aether shell <vm-name>`**
    *   *Contract:* Establishes a terminal tunnel into the VM.
    *   *Mechanism:*
        *   **For QEMU-KVM VMs:** Opens a stream over the host-guest **QEMU Guest Agent** socket channel.
        *   **For Firecracker microVMs:** Establishes a raw serial terminal connection over the **VSOCK (Virtual Socket)** backplane interface.
*   **`aether exec <vm-name> -- <command>`**
    *   *Contract:* Executes a non-interactive command inside the guest OS and streams `stdout`/`stderr` back to the developer's local shell.

### C. Directory Mounts & File Transfers

*   **`aether mount <local-path> <vm-name>:<guest-path>`**
    *   *Contract:* Enables directory sharing. For Firecracker microVMs, this modifies the VM spec to include a `virtiofs_source` path. The host daemon mounts the folder directly via **VirtioFS** with line-rate host filesystem performance.
*   **`aether transfer <local-file> <vm-name>:<guest-path>`**
    *   *Contract:* Streams file binaries over the gRPC control connection down to the host daemon `aetherd`, which writes them to the guest filesystem via the Guest Agent or VSOCK channel.

### D. Storage Management & Soft Deletion

*   **`aether snapshot <vm-name> --name <snap-name>` / `aether restore`**
    *   *Contract:* Leverages Aether's ZFS-on-Linux backend. Invoking `snapshot` triggers `aetherd` on the winning blade node to execute an instant **ZFS ZVOL snapshot** (`zroot/vms/vm-name@snap-name`) with zero allocation overhead.
*   **`aether delete <vm-name>`**
    *   *Contract:* Marks the VM as `Deleted` in the CRD spec but preserves the underlying ZVOL data, moving it into a "recycle-bin" state.
*   **`aether recover <vm-name>`**
    *   *Contract:* Restores a soft-deleted VM, re-enabling it in the auction queue.
*   **`aether purge`**
    *   *Contract:* Permanently destroys all soft-deleted VM ZVOLs, reclaiming raw NVMe blocks on the storage blades.

---

## 3. CLI Command Schema Reference

```
Usage: aether [COMMAND] [ARGS]...

Commands:
  launch      Create and start a virtual machine instance
  list        List all running/stopped instances in the current tenant space
  info        Show detailed resource and IP allocation for an instance
  start       Boot a stopped instance
  stop        Send a graceful ACPI shutdown signal to an instance
  shell       Open an interactive terminal session inside the guest (via VSOCK/Agent)
  exec        Execute a command inside the guest and stream output
  mount       Mount a local folder into the VM via VirtioFS
  transfer    Copy files into/out of the VM guest environment
  snapshot    Create a ZFS block-level snapshot of the VM volume
  restore     Revert the VM storage to a previous snapshot
  delete      Move an instance to the soft-deleted trash bin
  recover     Recover a soft-deleted instance
  purge       Permanently erase deleted instances and reclaim ZFS blocks
```
