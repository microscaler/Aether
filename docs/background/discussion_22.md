# Project Aether: Autonomous, Decentralized Multi-Tenant Compute Plane

[![License: Apache 2.0](https://shields.io)](https://opensource.org)
[![Language: Rust](https://shields.io)](https://rust-lang.org)
[![Infrastructure: Pure Linux / Firecracker](https://shields.io)](https://github.io)

**Project Aether** is a zero-dependency, open-source hypervisor orchestration plane designed to turn enterprise blade chassis deployments into highly dense, multi-tenant private clouds. Written entirely in Rust, Aether eliminates heavy, licensing-restrictive management frameworks (such as VMware vCenter and vSphere DRS) and replaces them with an elegant, decentralized **Reverse-Bidding Architecture**.

Standardized on **Bare-Metal Linux, Firecracker MicroVMs, and Linux ZFS Volumes (ZVOLs)**, Aether slices dense hardware footprints—such as an HPE BladeSystem c7000 with 640 CPU cores and 4TB of RAM—into secure, isolated compute environments with sub-100ms boot performance and near-zero hypervisor overhead.

---

## Executive Pitch: Eliminating the VMware Tax

Most Small and Medium Enterprises (SMEs) operating on-premise hardware barely scratch the surface of the features provided by a full VMware vSphere suite. Yet, they pay massive licensing penalties while sacrificing significant hardware capacity to run heavy, centralized management virtual appliances.

Aether delivers **"just enough orchestration"** to replace the most widely used enterprise features with cloud-native, compiled alternatives:

*   **vCenter Server ──► Stateless K8s Operator & GitOps Framework:** We replace heavy, stateful lifecycle databases with a lightweight Rust controller pod running inside a utility Kubernetes cluster, synchronized directly via **FluxCD**.
*   **vSphere DRS ──► Autonomous Reverse-Bidding Daemon:** Centralized scheduler calculations are eliminated. Each blade node evaluates its own real-time telemetry and "bids" down to the millisecond to claim incoming workloads.
*   **vSphere HA ──► Out-of-Band Redfish Fencing:** Real-time health monitoring automatically triggers automated hardware power truncation via the HPE iLO Redfish API if a blade fails, safely re-auctioning orphan workloads instantly without risk of data split-brain.

---

## Technical Architecture & Cluster Carve-Out

Aether avoids the performance penalties of nested virtualization. Every blade runs a minimal, bare-metal Linux installation, but the cluster is partitioned into two distinct, specialized hardware profiles at the logical level. This approach balances bursty, ephemeral developer tasks with persistent infrastructure components across the enclosure midplane.

┌───────────────────────────────────────────────────────────────────────────────────────┐
│ PURE LINUX HPE c7000 BLADE CHASSIS │
├───────────────────────────────────────────────────────────────────────────────────────┤
│ [ Compute Pool: Ephemeral & Pods ] │ [ Storage Pool: Persistent Infra & K8s ] │
│ Slots 1–8 (8 Blades) │ Slots 9–16 (8 Blades) │
├────────────────────────────────────────────┼──────────────────────────────────────────┤
│ - Engine: Firecracker / MicroVM Runtimes │ - Engine: Heavy QEMU-KVM Hypervisor │
│ - Storage: OverlayFS Container Images │ - Storage: Thin-LVM / ZFS on Linux │
│ - Workloads: Cloud Functions, MicroVM Dev │ - Workloads: K8s Control Plane, DB VMs │
└────────────────────────────────────────────┴──────────────────────────────────────────┘

### Profile 1: The Compute Blades (Slots 1–8)
*   **Target Workloads:** Ephemeral developer micro-environments, serverless cloud functions, and high-density multi-tenant container pods.
*   **Mechanics:** Optimized for rapid, lightweight execution. The Rust local node daemon (`aetherd`) intercepts requests and leverages Firecracker to spin up secure, hardware-isolated microVMs directly onto the bare-metal kernel in under 100 milliseconds, utilizing less than 5MB of memory overhead per instance.

### Profile 2: The Storage & Infrastructure Blades (Slots 9–16)
*   **Target Workloads:** Long-lived "full-fat" Linux virtual machines, production database replicas, and persistent Kubernetes control plane/worker nodes.
*   **Mechanics:** Optimized for heavy, stateful processing. These blades run full QEMU-KVM configurations and leverage **ZFS on Linux (ZVOLs)** to handle block-level storage. By matching KVM with ZFS datasets, the cluster handles database deployments with inline data compression, thin provisioning, and 0ms atomic snapshot cloning.

---

## How It Works: The Reverse-Bidding Marketplace

Instead of a centralized scheduler pushing workloads onto nodes based on a potentially stale cluster database state, Aether implements a decentralized, pull-based marketplace model:

1.  **Workload Intent Broadcast:** The Aether Aggregator receives a declarative workload request via GitOps and broadcasts the specification payload (defining CPU quotas, memory bytes, storage boundaries, and tenant mappings) to all registered blade daemons over a secure gRPC channel.
2.  **Autonomous Telemetry Evaluation:** Each blade node parses the intent string and queries its own local kernel parameters. It evaluates CPU task congestion, memory channel bandwidth availability, and drive array wear leveling.
3.  **The Reverse-Bid Response:** Nodes compute an algorithmic score from 1 to 1000. If a node lacks resources to safely host the instance without degrading current SLAs, it returns `-1`. Healthy nodes return their score within a strict **250ms convergence window**.
4.  **Deterministic Convergence & Execution:** The Aggregator accepts the highest-value bid. If multiple nodes return identical scores, the engine drops into a multi-tier tie-breaker matrix (evaluating chassis thermal layout, adjacent slot density, and SSD smart write wear) to pick a winner deterministically. The winning node instantly clones its local volume, dynamically compiles a custom NoCloud Cloud-Init configuration drive using native memory utilities, and boots the hypervisor.

---

## Declarative Manifest Interface

The cluster is managed declaratively by aligning your infrastructure files with standard GitOps tools like FluxCD.

### Multi-Tenant Resource Isolation Ceiling
Tenants are bounded cleanly at the API level. The Aether Operator validates these resource envelopes before a workload can enter the reverse-bidding marketplace.

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

### MicroVM / Infrastructure Workspace Spec
Modifying parameters—such as bumping the `baseImage` version or altering the compute allocations—triggers an automated rolling update across the blades.

```yaml
apiVersion: compute.aether.infra/v1alpha1
kind: AetherVirtualDeployment
metadata:
  name: k8s-master-node-01
  namespace: tenant-engineering
spec:
  tenantRef: engineering-team
  runtimeRequirement: kvm-heavy     # Directs the workload to the Infrastructure Pool
  updateStrategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
  compute:
    vcpus: 8
    memoryBytes: 34359738368        # 32 GB RAM
  storage:
    storageClassName: aether-zfs-nvme
    rootVolumeSizeGB: 200
    baseImage: rhel-9-minimal-v2
  networking:
    requestedIP: "10.20.20.10"
```

---

## Reliability and Out-of-Band Fencing (STONITH)

To protect your persistent data from corruption caused by network partitions (where an isolated node continues writing to shared block domains), Aether implements out-of-band **STONITH (Shoot The Other Node In The Head)** enforcement:

1.  **Heartbeat Timeout:** If a node daemon falls off the gRPC communication bus for more than 15 consecutive seconds, it is immediately marked as `Suspect` and pulled from the active auction loop registry.
2.  **Out-of-Band Power Disruption:** Before any workloads can be reassigned, the `aether-fence` module opens a secure connection directly to the parent chassis' **HPE iLO 5 RESTful API (Redfish compliant)**. It transmits a forced hardware power interruption payload:
    ```json
    POST /redfish/v1/Systems/1/Actions/ComputerSystem.Reset
    { "ResetType": "ForceOff" }
    ```
3.  **State Verification & Re-Auction:** The Aggregator queries the Redfish power domain states. Once the hardware state returns a verified `PowerState: Off`, the Aggregator safely extracts the orphan workload specifications from memory and re-injects them into the reverse-bidding engine to self-heal the cluster onto healthy blades.

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

## License

Project Aether is licensed under the Apache License, Version 2.0. See the [LICENSE](LICENSE) file for details.
To advance the orchestration architecture, let me know if you would like to:
Propose the build configuration script (build.rs) for code-generation of the gRPC protobuf files on your Linux build host.
Request the automated shell-bootstrap configurations to deploy the aetherd binary onto a raw blade.
