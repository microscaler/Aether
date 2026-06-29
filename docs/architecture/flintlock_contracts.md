# Flintlock Functional Contracts & API Specification

This document extracts the high-level functional contracts and API specifications delivered by **Flintlock** (Liquid Metal's microVM manager). Since **Project Aether** was created as a result of these distributed proposals being rejected—combined with the commercial need to dispose of VMware licensing—Aether adapts these contracts to run on a decentralized, bare-metal Linux/ZFS infrastructure.

---

## 1. Core Service APIs

Flintlock exposes a gRPC service contract (`MicroVM`) to manage microVM lifecycles:

| API Operation | Input Payload | Output Payload | Description |
| :--- | :--- | :--- | :--- |
| `CreateMicroVM` | `CreateMicroVMRequest` | `CreateMicroVMResponse` | Spawns a new microVM instance based on the provided hardware and image specification. |
| `DeleteMicroVM` | `DeleteMicroVMRequest` | `google.protobuf.Empty` | Gracefully shuts down, terminates the hypervisor process, and cleans up host storage mounts. |
| `GetMicroVM` | `GetMicroVMRequest` | `GetMicroVMResponse` | Fetches the complete runtime status, including active host network interface names and mount points. |
| `ListMicroVMs` | `ListMicroVMsRequest` | `ListMicroVMsResponse` | Lists all microVMs assigned to a specific namespace, with optional name filtering. |
| `ListMicroVMsStream` | `ListMicroVMsRequest` | `stream ListMessage` | Open a server-side gRPC stream to receive real-time updates of VM state transitions. |

---

## 2. Specification Contracts (`MicroVMSpec`)

The functional layout of a VM deployment is structured around resource requests, OS boots, networking, and storage.

### A. Compute & Identity Metadata
*   **UID / ID / Namespace:** Uniquely identifies the VM across the cluster namespace.
*   **Labels:** Key-value pairs for organizational categorizations.
*   **vCPUs:** Integer specifying the core allocation.
*   **MemoryInMb:** Integer specifying the RAM allocation in Megabytes.
*   **Metadata:** A base64-encoded payload map (e.g., custom configuration scripts, SSH keys) passed to the guest metadata service.

### B. OS Kernel & Ramdisk Bootstrap
*   **Kernel Image:** Specifies an OCI container image containing the target kernel, along with:
    *   `filename`: The path to the kernel file inside the container image.
    *   `cmdline`: A map of key-value kernel command-line arguments (e.g., console routing, root device mappings).
    *   `add_network_config`: A boolean indicating whether to auto-generate network configurations.
*   **Initrd Image (Optional):** Specifies an OCI container image containing the initial ramdisk and its target filename.

### C. Storage Volume Layout
VM volumes (root and additional attachments) specify how filesystems are constructed:
*   **Volume ID:** Unique string identifier.
*   **ReadOnly:** Boolean indicating mount security.
*   **SizeInMb:** Optional size parameter to resize block storage.
*   **Volume Source:** Specifies where the files are sourced:
    *   `container_source`: An OCI container image to pull down and extract.
    *   `virtiofs_source`: A direct host folder path passed through via VirtioFS.
    *   *Note:* The spec includes hooks for Container Storage Interface (CSI) drivers.

### D. Networking Interfaces
Defines how guest interfaces attach to host networking backplanes:
*   **Device ID:** Unique ID representing the card.
*   **Interface Type:**
    *   `TAP`: Standard virtual Ethernet tap interface (useful for bridging).
    *   `MACVTAP`: High-performance bridge routing traffic directly from physical host interfaces.
*   **Guest MAC Address:** Custom static MAC or fallback to autogeneration.
*   **IP Configuration:**
    *   *Static IP:* CIDR-format IP address, default gateway, and nameserver IPs.
    *   *Dynamic IP:* Default fallback to DHCP.
*   **Overrides:** Custom Linux bridge name (`bridge_name`) to attach the guest interface.

---

## 3. Runtime Status Contracts (`MicroVMStatus`)

The status reporting structure tracks active resource usage on the host:

*   **Lifecycle State Machine:** Transitions between `PENDING`, `CREATED`, `FAILED`, and `DELETING`.
*   **Mount Status:** Details the physical location on the host filesystem (`HOSTPATH` or `/dev` path) where the kernel, initrd, and storage volumes are mounted.
*   **Network Interface Status:**
    *   `host_device_name`: The physical interface name assigned on the host OS.
    *   `index`: Host OS interface index.
    *   `mac_address`: The MAC address of the host-side endpoint.
*   **Reconciliation State:** Tracks retry metrics for self-healing loops.

---

## 4. How Aether Adapts and Unifies These Contracts

Project Aether does not simply discard the Flintlock model; rather, it **unifies and expands** it into a single decentralized compute plane. Aether delivers dual capabilities across the HPE blade chassis, supporting both high-density ephemeral workflows and persistent, heavy-virtualization workloads:

### A. The Dual Execution Planes

1.  **Ephemeral microVM Plane (Firecracker):** Standardized on slots 1–8 (Compute Pool). It maps directly to Flintlock's model, using OCI container extractions and OverlayFS caching to spawn lightweight, hardware-isolated microVMs in sub-100ms for developer workspaces and ephemeral container pods.
2.  **Persistent VMware-Replacement Plane (QEMU-KVM):** Standardized on slots 9–16 (Infrastructure Pool). It replaces legacy VMware virtualization, utilizing **ZFS on Linux (ZVOLs)** to provide instant block-level cloning, thin provisioning, inline compression, and persistent storage for heavy virtual machines (such as databases and Kubernetes control planes).

### B. Functional Adaptation Matrix

| Flintlock Specification Profile | Aether Ephemeral Plane (Firecracker) | Aether Persistent Plane (QEMU-KVM) |
| :--- | :--- | :--- |
| **Storage Sourcing** | Read-heavy OCI image extractions & local OverlayFS mounts. | Thin-provisioned **ZFS on Linux (ZVOL) clones** with 0ms snapshots. |
| **Networking Integration** | Linux bridges mapped to hardware Virtual Connect VLANs. | Direct LACP-trunked bridge routing, mapping static IPs to VLAN interfaces. |
| **Boot Mechanism** | Minimal guest kernel + initrd execution loop (<5MB RAM overhead). | Standard OS boot via full QEMU virtualization, mounting custom autogenerated Cloud-Init ISO metadata drives. |
| **Cluster Orchestration** | Isolated single-host REST endpoints. | Decentralized **Reverse-Bidding Marketplace** via gRPC and token-attested node daemons (`aetherd`). |
| **Failover / HA** | Ephemeral tear-down and clean rebuild. | Out-of-band **Redfish STONITH fencing** via HPE iLO 5, followed by instant VM re-auctioning. |

---

> [!NOTE]
> While Aether's initial driver implementations target **HPE iLO 5** and **HPE Virtual Connect** modules, these vendor-specific control protocols are decoupled from the core scheduler. They are implemented as pluggable drivers behind the `ChassisManager` and `MidplaneNetworkManager` traits in the **Hardware Abstraction Layer (HAL)** (see [ARCHITECTURE.md](file:///Users/casibbald/Workspace/remote/microscaler/Aether/ARCHITECTURE.md#4-multi-vendor-hardware-abstraction-layer-hal) for details), allowing future expansion to Dell, IBM/Lenovo, and other blade platforms.
