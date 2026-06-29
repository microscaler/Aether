# Project Aether: Autonomous, Decentralized Multi-Tenant Compute Plane

[![License: Apache 2.0](https://shields.io)](https://opensource.org)
[![Language: Rust](https://shields.io)](https://rust-lang.org)
[![Infrastructure: Pure Linux / Firecracker](https://shields.io)](https://github.io)

**Project Aether** is a zero-dependency, open-source bare-metal hypervisor management platform designed to turn enterprise blade chassis deployments into highly dense, multi-tenant private clouds. Inspired by Kubernetes Cluster API (CAPI) philosophies, Aether completely replaces heavy, centralized, licensing-restrictive management frameworks (like VMware vCenter and vSphere DRS) with a fast, type-safe, and decentralized **Reverse-Bidding Architecture** written in Rust.

Standardized entirely on **Bare-Metal Linux, Firecracker MicroVMs, and Linux ZFS Volumes (ZVOLs)**, Aether slices a physical cluster—such as an HPE BladeSystem c7000 with 640 CPU cores and 4TB of RAM—into secure, isolated compute environments with sub-100ms boot performance and near-zero hypervisor overhead.

---

## The Strategic Pivot for CTOs & Platform Leaders

### Why Aether?
Enterprise infrastructure is facing unprecedented vendor licensing shifts and forced architectural migrations. For Small and Medium Enterprises (SMEs) operating dense on-premise hardware footprints, traditional hypervisors consume a massive percentage of system resources just to run their own management engines, while adding millions in licensing friction.

Aether cuts out the middleman. Most engineering environments utilize less than 20% of the features built into corporate virtualization suites. Aether implements **"just enough orchestration"** to replace the most widely used enterprise features:

*   **vCenter Server Replacement ──► Stateless K8s Operator & GitOps Framework:** We replace monolithic database controllers with a lightweight Rust controller pod running inside a management Kubernetes cluster, synchronized directly via **FluxCD**.
*   **vSphere DRS Replacement ──► Autonomous Reverse-Bidding Daemon:** There is no centralized scheduler calculations. When a workload is declared, bare-metal nodes calculate their local resource availability and submit a competitive bid to claim it.
*   **vSphere HA Replacement ──► Decentralized gRPC Deadman Switch:** Real-time health monitoring automatically triggers an automated hardware power truncation via out-of-band management if a blade fails, safely re-auctioning orphan workloads instantly.

---

## Architectural Blueprint

[ GitOps Manifest Directory / Private Repository ]
│
▼ (FluxCD Automated Sync Loop)
[ Aether Aggregator Core K8S Operator Pod ]
│
┌───────────────────────┴───────────────────────┐
│ gRPC Control Bus (HPE Virtual Connect VLAN 10)│
▼ ▼
[ Blade Server Node 01 ] [ Blade Server Node 16 ]
(Bare-Metal Linux / aetherd) (Bare-Metal Linux / aetherd)
├── Firecracker Engine ├── Firecracker Engine
└── ZFS on Linux (ZVOLs) └── ZFS on Linux (ZVOLs)

### The Core Technology Stack
*   **The Substrate (Bare-Metal Linux):** Eliminates hypervisor nesting penalties. Firecracker runs directly on top of the physical `/dev/kvm` hardware extensions.
*   **The Virtual Machine Monitor (Firecracker):** Provides secure, multi-tenant kernel isolation with minimal memory footprints (~5MB per microVM) and sub-100ms initialization phases.
*   **The Storage Subsystem (Linux ZVOLs):** Leverages copy-on-write ZFS block-level datasets on native local SSD pools. This allows the cluster to execute instant, 0ms base-image snapshots and thin-provisioned `PersistentVolumeClaims` using **`democratic-csi`** integration.

---

## How It Works: The Reverse-Bidding Core

Traditional scheduling pushes a workload onto a node based on a centralized, potentially stale cluster database state. Aether introduces a pull-based marketplace model:

1.  **Workload Intent Broadcast:** The Aether Aggregator receives a declarative workload request via GitOps and broadcasts the specification payload (defining CPU quotas, memory bytes, storage boundaries, and tenant mappings) to all registered blade daemons (`aetherd`) over a secure HTTP/2 gRPC channel.
2.  **Autonomous Telemetry Evaluation:** Each blade node parses the intent string and queries its own local kernel parameters. It checks CPU task congestion, memory channel bandwidth limits, and disk health metrics.
3.  **The Reverse-Bid Response:** Nodes compute an algorithmic score from 1 to 1000. If a node lacks resources to safely host the instance without degrading current SLAs, it returns `-1`. Healthy nodes return their score within a strict **250ms convergence window**.
4.  **Deterministic Convergence & Execution:** The Aggregator accepts the highest-value bid (using cryptographic and physical layout metrics to resolve ties deterministically). The winning node instantly clones its local ZFS template, dynamically compiles a custom NoCloud Cloud-Init `seed.iso` using native memory utilities, mounts the hardware volumes, and executes the hypervisor.

---

## System Components

Aether is packaged as a type-safe, compiled Rust workspace splitting clear operational responsibilities:

```text
aether-cluster-core/
├── Cargo.toml
├── proto/
│   └── aether.proto       # High-performance gRPC protobuf definitions
└── src/
    ├── main.rs            # Central Aggregator runtime / K8S Operator controller
    ├── bhyve_kvm.rs       # Local hypervisor virtualization API connectors
    ├── tiebreaker.rs      # Pure calculation engine for sorting identical bids
    ├── fence.rs           # STONITH HPE iLO 5 / Redfish power fencing automation
    └── gitops_storage.rs  # Serde structures parsing manifests to ZVOL disk queues
```

---

## Declarative Operations (Examples)

### 1. Multi-Tenant Resource Allocation Ceilings
Tenants are bounded cleanly at the declarative level. The Aether Operator validates these resource envelopes before a workload can enter the reverse-bidding marketplace.

```yaml
apiVersion: core.aether.infra/v1alpha1
kind: AetherTenant
metadata:
  name: engineering-team
  namespace: aether-system
spec:
  networkIsolation:
    vlanTag: 20                      # Hard-wired HPE Virtual Connect VLAN mapping
    subnetCIDR: "10.20.20.0/24"
  quotas:
    maxVcpus: 128
    maxMemoryBytes: 549755813888    # 512 GB RAM pool ceiling
    maxStorageBytes: 10995116277760 # 10 TB ZFS volume pool ceiling
```

### 2. MicroVM / Pod Instance Configurations
When an application instance is committed to Git, the system reconciles its lifecycle automatically. Changing the `baseImage` version string triggers an automated rolling update.

```yaml
apiVersion: compute.aether.infra/v1alpha1
kind: AetherVirtualDeployment
metadata:
  name: k8s-worker-p04
  namespace: tenant-engineering
spec:
  tenantRef: engineering-team
  runtimeRequirement: firecracker
  updateStrategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
  compute:
    vcpus: 4
    memoryBytes: 17179869184       # 16 GB RAM
  storage:
    storageClassName: aether-zfs-nvme
    rootVolumeSizeGB: 100
    baseImage: ubuntu-24.04-minimal-v2
  networking:
    requestedIP: "10.20.20.44"
```

---

## High Availability & Hard Fencing (STONITH)

To protect your system from data corruption caused by network partitions (where an isolated node continues writing to shared block domains), Aether implements out-of-band **STONITH (Shoot The Other Node In The Head)** enforcement:

1.  **Heartbeat Timeout:** If a node daemon falls off the gRPC communication bus for more than 15 consecutive seconds, it is immediately marked as `Suspect` and pulled from the active auction loop registry.
2.  **Out-of-Band Power Disruption:** Before any workloads can be reassigned, the `aether-fence` module opens a secure connection directly to the parent chassis' **HPE iLO 5 RESTful API (Redfish compliant)**. It transmits a forced hardware power interruption payload:
    ```json
    POST /redfish/v1/Systems/1/Actions/ComputerSystem.Reset
    { "ResetType": "ForceOff" }
    ```
3.  **State Verification & Re-Auction:** The Aggregator queries the Redfish power domain states. Once the hardware state returns a verified `PowerState: Off`, the Aggregator safely extracts the orphan workload specifications from memory and re-injects them into the reverse-bidding engine to self-heal the cluster onto healthy blades.

---

## License

Project Aether is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.
To advance the project setup, please specify if you would like me to draft the concrete rust compilation script (build.rs) for code-generation of the gRPC protobuf files on your Linux build host, or outline the automated server bootstrap scripts to provision the aetherd binary onto a raw blade.

