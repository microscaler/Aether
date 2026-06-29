# Proxmox Architecture Analysis & v1 Integration Plan

This document analyzes the core functionality of **Proxmox Virtual Environment (VE)** and **Proxmox Datacenter Manager (PDM)**. It extracts "just enough functionality" from these platforms to complete a production-ready **v1 version of Project Aether**, substituting complex centralized cluster databases with our decentralized, reverse-bidding GitOps engine.

---

## 1. Architectural Mapping: Proxmox vs. Aether v1

Proxmox delivers an enterprise virtualization platform built around clustering, dual virtualization styles, and centralized storage. Aether v1 adapts these patterns to a minimalist, stateless architecture:

| Architectural Vector | Proxmox VE / PDM Solution | Project Aether v1 Adaptation |
| :--- | :--- | :--- |
| **Virtualization Style** | Dual execution engine: **KVM** (full virtual machines) + **LXC** (Linux Containers). | Dual execution engine: **QEMU-KVM** (persistent, full virtual machines) + **Firecracker** (ephemeral, sub-100ms microVMs). |
| **Cluster Database** | Centralized, replicated cluster filesystem (`pmxcfs`) synchronized across hosts via Corosync. | **Stateless Control Plane:** No replicated database. In-memory state table synced from Kubernetes Custom Resources via **FluxCD (GitOps)**. |
| **High Availability (HA)** | Replicated `ha-manager` service executing fencing and cluster-wide lock consensus. | **Deadman Switch Heartbeat Loop:** 15s timeout loop in the Aggregator operator. Triggers hard power fencing (STONITH) via Redfish/iLO API. |
| **Storage Management** | Storage abstraction layer managing local ZFS pools, thin LVM, Ceph, and NFS. | Standardized **ZFS on Linux (ZVOL)** snapshot cloning for persistent storage, and **OverlayFS container mounts** for ephemeral microVMs. |
| **Networking Control** | Software-Defined Networking (SDN) managing VLANs, VxLAN overlays, and virtual fabrics. | **HAL midplane routing:** Pluggable hardware bridge API (HPE Virtual Connect or Dell SmartFabric) bridging tagged VLANs directly to host bridges. |
| **Multi-Cluster Manager** | Datacenter Manager (PDM) providing single-pane-of-glass API querying over disjointed clusters. | **Aether Aggregator:** Kubernetes-native Operator that aggregates cluster resource telemetry dynamically during the 250ms auction window. |

---

## 2. "Just Enough Proxmox" for Aether v1

To deliver a robust v1 without introducing high software overhead, Aether extracts five specific operational contracts from Proxmox:

### A. Dual-Engine Node Agent (`aetherd`)
Instead of choosing between a container manager and a VM hypervisor, Aether's local Rust daemon (`aetherd`) acts as a unified executor on the host:
*   For ephemeral, high-density serverless and micro-environments: Spawns **Firecracker** processes using a minimal kernel/initrd and lightweight OverlayFS roots.
*   For long-lived, heavy persistent servers: Spawns **QEMU-KVM** processes booting complete operating systems via raw block-storage mounts.

### B. Unified Storage API (ZFS Focus)
Aether maps storage requests to local host disks similar to Proxmox's ZFS integration:
*   A base snapshot (e.g., `zroot/templates/ubuntu-22.04@base`) serves as the read-only backing block.
*   When a persistent VM bid wins, `aetherd` clones the snapshot instantly into a thin-provisioned **ZVOL** (`zroot/vms/vm-101-disk-0`) with 0ms allocation overhead and inline compression enabled.
*   Ephemeral VMs bypass ZFS cloning, instead using container image layers extracted onto local `OverlayFS` mounts.

### C. Stateless HA & Out-of-Band Fencing
Aether replicates Proxmox’s VM protection during node crashes without requiring Corosync quorum setups:
*   The Aggregator operator runs a lightweight background loop, checking node health every 3 seconds.
*   If a blade fails to respond for 15 seconds (5 checks), the Aggregator calls `aether-fence` to execute a STONITH command via the Redfish API.
*   Once power interruption is verified, the Aggregator re-injects the VMs into the reverse-bidding marketplace.

### D. VLAN-Aware Bridging
Aether handles midplane networks using native Linux network features:
*   Aether configures host-level Linux bridges (`br-tenant`).
*   The guest VM virtual network interfaces (TAP/MACVTAP) are attached directly to these bridges.
*   The physical blade interface is configured as a trunk port on the Virtual Connect backplane, filtering tags at the hardware layer.

---

## 3. Implementation Blueprint for Aether v1

```
                         [ GitOps Custom Resource (YAML) ]
                                         │
                                         ▼
                        [ Central Aggregator Operator ]
                                         │
                 ┌───────────────────────┴───────────────────────┐
                 │ (Auction: Broadcast Workload Spec)            │ (Fencing: 15s Heartbeat Timeout)
                 ▼                                               ▼
     [ aetherd (Worker Node) ]                        [ aether-fence (Redfish HAL) ]
                 │                                               │
     ┌───────────┴───────────┐                                   │ (Hard Power Reset)
     ▼                       ▼                                   ▼
 [ Firecracker ]        [ QEMU-KVM ]                    [ HPE iLO / Dell iDRAC ]
 (Ephemeral vCPU)      (Persistent vCPU)
     │                       │
     ▼                       ▼
 [ OverlayFS ]          [ ZFS ZVOL ]
 (Temp Disk)           (Snapshot Clone)
```

By focusing on these core capabilities, Aether v1 provides the essential cloud virtualization features of Proxmox VE and PDM within a lightweight, GitOps-driven footprint, eliminating the need for complex database synchronizations or heavy management consoles.
