To enforce strict multi-tenancy without a heavy virtualization layer, Project Aether handles isolation using a Multi-Tenant Policy Mapping Engine.
When a small business or platform team defines a tenant boundary in GitOps, the Aether Aggregator intercepts that Custom Resource Definition (CRD). It maps those logical boundaries directly onto the low-level, bare-metal isolation primitives of the winning blade's operating system: Linux Namespaces/Cgroups (for KVM and Firecracker hosts) or FreeBSD Jails/ZFS Datasets (for bhyve hosts).
1. Architectural Topology: Logical to Bare-Metal Isolation

       [ GitOps Manifest: `AetherTenant` Resource ]
                           │
                           ▼ (FluxCD Syncs to K8S API)
       [ Aether Aggregator Core K8S Operator ]
                           │
                           ▼ (gRPC Control Bus via VLAN 10)
         ┌─────────────────┴─────────────────┐
         ▼ (If Allocated to Linux Host)      ▼ (If Allocated to FreeBSD Host)
┌───────────────────────────────┐   ┌───────────────────────────────┐
│   Linux Bare-Metal Node       │   │   FreeBSD Bare-Metal Node     │
├───────────────────────────────┤   ├───────────────────────────────┤
│ 1. Systemd-Cgroups (Slice)    │   │ 1. Hierarchical Jails         │
│    Hard Core/RAM quotas       │   │    `jail -c name=tenant...`   │
│                               │   │                               │
│ 2. Firecracker Jailer         │   │ 2. ZFS Dataset Delegation     │
│    Chroot / Drop Privileges   │   │    `zfs set jailed=on...`     │
│                               │   │                               │
│ 3. Linux Bridge / veth pairs  │   │ 3. VNET Network Stack         │
│    Network Namespace isolation│   │    Virtual network interfaces │
└───────────────────────────────┘   └───────────────────────────────┘
2. The Declarative Custom Resource Definitions (CRDs)

To establish these boundaries, we define two custom schemas inside the management Kubernetes cluster: AetherTenant(defining the boundary quotas and namespaces) and AetherVirtualDeployment (which references the parent tenant).
2.1 The Multi-Tenant Boundary Schema (tenant-engineering.yaml)

This resource sets a hard ceiling on what a specific group can consume across your 640-core chassis pool.
apiVersion: core.aether.infra/v1alpha1
kind: AetherTenant
metadata:
name: engineering
namespace: aether-system
spec:
description: "Core R&D Compute Cluster Boundary"
networkIsolation:
vlanTag: 20                      # Hard-wired 802.1Q hardware VLAN tag
subnetCIDR: "10.20.20.0/24"
quotas:
maxVcpus: 128                   # Limit across the entire blade pool
maxMemoryBytes: 549755813888    # 512 GB RAM Limit
maxStorageBytes: 10995116277760 # 10 TB NVMe Pool Limit
securityProfile:
allowRootFunnels: false         # Prevents host kernel file mapping leaks
hypervisorPolicy: StrictVirtIO  # Disables legacy emulated hardware components
2.2 The Tenant-Bound Workload Schema (db-prod-01.yaml)

When a developer or automated system deploys a workload, it must map back to a defined tenant context.
apiVersion: compute.aether.infra/v1alpha1
kind: AetherVirtualDeployment
metadata:
name: db-prod-01
namespace: tenant-engineering     # Enforces RBAC boundary inside Kubernetes
spec:
tenantRef: engineering            # Explicit cryptographic mapping linkage
runtimeRequirement: bhyve
compute:
vcpus: 8
memoryBytes: 34359738368
storage:
storageClassName: gold-zfs-nvme
rootVolumeSizeGB: 200
baseImage: ubuntu-24.04-server
3. Rust Multi-Tenant Resource Verification Blueprint (tenant_validator.rs)

Running within your Kubernetes-hosted Aggregator binary, this component intercepts incoming workload creation requests. It calculates current utilization tables across the cluster nodes to ensure a new VM does not breach a tenant's predefined resource envelope before broadcasting the request to the reverse-bidding system.
// Unified Rust Structural Module Map: aether-tenant-validator

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantQuotas {
pub max_vcpus: u32,
pub max_memory_bytes: u64,
pub max_storage_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantSpec {
pub quotas: TenantQuotas,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AetherTenant {
pub name: String,
pub spec: TenantSpec,
}

pub struct ActiveClusterUtilization {
pub consumed_vcpus: u32,
pub consumed_memory_bytes: u64,
pub consumed_storage_bytes: u64,
}

pub struct AetherTenantValidator;

impl AetherTenantValidator {
/// Evaluates if an incoming workload deployment fits inside a tenant's authorized resource profile
pub fn validate_quota_ceiling(
tenant: &AetherTenant,
current_utilization: &ActiveClusterUtilization,
requested_vcpus: u32,
requested_memory_bytes: u64,
requested_storage_bytes: u64,
) -> Result<(), String> {

        // 1. Calculate projected usage increments
        let projected_vcpus = current_utilization.consumed_vcpus + requested_vcpus;
        let projected_memory = current_utilization.consumed_memory_bytes + requested_memory_bytes;
        let projected_storage = current_utilization.consumed_storage_bytes + requested_storage_bytes;

        // 2. Validate compute allocation boundaries
        if projected_vcpus > tenant.spec.quotas.max_vcpus {
            return Err(format!(
                "Quota Denied: Tenant '{}' would exceed max vCPU limit (Limit: {}, Requested Total: {})",
                tenant.name, tenant.spec.quotas.max_vcpus, projected_vcpus
            ));
        }

        // 3. Validate memory footprint boundaries
        if projected_memory > tenant.spec.quotas.max_memory_bytes {
            return Err(format!(
                "Quota Denied: Tenant '{}' would exceed memory byte footprint (Limit: {}, Requested Total: {})",
                tenant.name, tenant.spec.quotas.max_memory_bytes, projected_memory
            ));
        }

        // 4. Validate storage exhaustion boundaries
        if projected_storage > tenant.spec.quotas.max_storage_bytes {
            return Err(format!(
                "Quota Denied: Tenant '{}' would violate storage pool limits (Limit: {}, Requested Total: {})",
                tenant.name, tenant.spec.quotas.max_storage_bytes, projected_storage
            ));
        }

        println!("Multi-tenant quota verification passed for tenant: {}", tenant.name);
        Ok(())
    }
}
4. Host-Level Enforcement Mechanisms

Once a request passes the quota verification checks, the node daemon (aetherd) that wins the auction maps the tenant's structural boundaries onto local host security primitives:
4.1 The FreeBSD Execution Path (bhyve / Jails)

When the winning FreeBSD node intercepts an AetherVirtualDeployment specifying the engineering tenant, it creates an isolated execution container using native kernel primitives:
# 1. Restrict storage pathing using ZFS dataset constraints
zfs create zroot/vm/engineering/db-prod-01
zfs set quota=200G zroot/vm/engineering/db-prod-01

# 2. Spawn a specialized VNET network jail to isolate tenant traffic
jail -c name=engineering-db-prod-01 host.hostname=db-prod-01.engineering \
path=/zroot/vm/engineering/db-prod-01 vnet vnet.interface=epair0b \
exec.start="/usr/sbin/bhyve -c 8 -m 32G -H -P ..."
4.2 The Linux Execution Path (KVM / Firecracker)

If a bare-metal Linux host wins the workload, it limits resources and isolates execution using kernel namespaces and systemd slices:
# 1. Configure systemd slice cgroups dynamically to constrain CPU and Memory
systemd-run --slice=tenant-engineering.slice --unit=fc-db-prod-01 \
--property=CPUQuota=800% --property=MemoryMax=32G \
/usr/bin/jailer --id db-prod-01 --node 0 --exec-file /usr/bin/firecracker ...
The Jailer binary automatically drops root permissions, applies a hard chroot restriction to the directory, sets up a dedicated network namespace (netns), and drops process capabilities down to an unprivileged system user. This ensures that even if an attacker compromises the VM guest kernel, they remain trapped inside the tenant's namespace boundary.
5. SME Operational Guardrails: Dynamic Quorum & Tenant Defense

5.1 Noisy Neighbor Isolation (Dynamic Cgroup & Jail Tuning)

If a user VM under the operations tenant malfunctions and initiates a massive compilation loop or a memory leak, it can cause resource starvation across the shared blade hardware.
The Enforcement Strategy: The local Rust node daemon monitors system schedulers. If a tenant's combined workloads cause host memory swap cycles to spike beyond 5% overhead, the daemon calls host control scripts to lower the cgroup weight or invoke rctl modifications on the FreeBSD jail. This throttles the rogue tenant's resource access, preserving memory bandwidth channels for higher-priority tenant workloads.
5.2 Tenant-Scoped Storage Erasure (Crypto-Shredding Fallback)

When a tenant deletes an entire workload manifest in GitOps, simply unlinking files leaves raw bit blocks on the physical 3.84TB local SSDs, running a risk of data remanence leaks across multi-tenant boundaries.
The Automation Fix: If an AetherTenant manifest is removed, the local node daemons execute a Crypto-Shredding Pipeline. When the tenant space is first provisioned, ZFS datasets and LVM block domains are encrypted with a unique per-tenant key managed in memory by the local daemon. Upon a deletion instruction, the daemon zero-fills the master crypto-key header block and issues a trim command (blkdiscard on Linux, zpool trim on FreeBSD). This instantly renders the underlying physical blocks unreadable, securing deleted multi-tenant data boundaries.
This specification rounds out the architectural design for multi-tenant isolation, connecting declarative GitOps manifests directly to low-level kernel protection mechanisms.
If you are ready to proceed, we can detail the automated shell-bootstrap scripts used to install the Rust aetherddaemon onto a clean, bare-metal FreeBSD blade, or design the gRPC metrics telemetry collection contracts used to update the status blocks of your Kubernetes CRDs. How would you like to proceed?

