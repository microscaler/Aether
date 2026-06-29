Architectural Specification: GitOps Manifest Parsing Layout & Storage Mapping Specs

Component Identifier: aether-gitops-storage
Subsystem Context: Cluster Declarative State & Host Filesystem Integration
Target Hardware/OS: HPE c7000 Midplane, FreeBSD 14+ / ZFS, Linux 6.x / Thin-LVM
1. Directory Structure Blueprint (The Source of Truth)

The central Cluster Aggregator reads the intended state of the cluster from a flat hierarchical directory structure, typically synchronized via a background git pull cron task or webhook listener. The storage tree is structured to enforce separation between core multi-tenant infrastructures, storage classes, and target runtime configurations.
/etc/aether/manifests/
├── cluster.yaml                     # Unified global settings (VLAN trunks, DNS)
├── storage-classes/                 # Storage backend profiling mapping data
│   ├── gold-zfs-nvme.yaml           # High-IOPS ZFS pool parameters for bhyve
│   └── silver-lvm-sas.yaml          # Thin-LVM backing store config for Linux/KVM
├── tenants/                         # Multi-tenant resource boundary isolation
│   ├── engineering.yaml
│   └── operations.yaml
└── workloads/                       # Declarative instance specifications
├── k8s-worker-01.yaml           # Runs inside standard Linux VM
├── serverless-fn-bc4.yaml       # Runs inside an ephemeral Firecracker microVM
└── db-replica-09.yaml           # Runs inside an isolated FreeBSD bhyve node
2. Declarative Schema Specifications (YAML Parser Contracts)

The aether-gitops-reconciler reads these manifests and uses Rust's serde abstractions to validate them against strict hardware schema requirements before triggering an auction.
2.1 Storage Class Specification (gold-zfs-nvme.yaml)

apiVersion: storage.aether.infra/v1alpha1
kind: StorageClass
metadata:
name: gold-zfs-nvme
spec:
provisioner: freebsd.zfs
parameters:
zpool: zroot
datasetPath: zroot/vm
compression: lz4
deduplication: "off"
recordsize: "128K"
primarycache: all
2.2 Core Workload Specification (db-replica-09.yaml)

apiVersion: compute.aether.infra/v1alpha1
kind: AetherVirtualDeployment
metadata:
uid: d9231f45-a7b2-4ce0-9831-8f2c3b4a5e6f
name: db-replica-09
tenant: engineering
spec:
runtimeRequirement: bhyve       # Enforces routing exclusively to FreeBSD nodes
compute:
vcpus: 8
memoryBytes: 34359738368      # 32 GB RAM
storage:
storageClassName: gold-zfs-nvme
rootVolumeSizeGB: 120
baseImage: ubuntu-24.04-server-gold
networking:
vlanTag: 20
macAddress: "02:A0:C7:00:09:DB"
3. Storage Mapping Architecture & Cloning Pipelines

Because your blades are equipped with fast 3.84TB local enterprise SAS/NVMe SSDs, optimizing the raw I/O data path is critical. The system handles image slicing differently based on the OS node that wins the reverse-bid.
[ Workload Allocated to Node ]
│
┌────────────────────────┴────────────────────────┐
▼ (If Won by FreeBSD/bhyve Blade)                 ▼ (If Won by Linux/KVM Blade)
┌─────────────────────────────────┐               ┌─────────────────────────────────┐
│     ZFS Fast Snap Pipeline      │               │     Thin-LVM Layer Pipeline     │
├─────────────────────────────────┤               ├─────────────────────────────────┤
│ 1. Verify master template:      │               │ 1. Check local baseline pool:   │
│    `zroot/vm/templates/ubuntu`  │               │    `/dev/vg_aether/tp_silver`   │
│                                 │               │                                 │
│ 2. Execute 0ms raw ZFS clone:   │               │ 2. Create thin block snapshot:  │
│    `zfs clone templates/ubuntu  │               │    `lvcreate -s --thinpool...`  │
│     zroot/vm/db-replica-09`     │               │                                 │
│                                 │               │ 3. Instantly mount block disk   │
│ 3. Set properties dynamically:  │               │    as local raw loop file to     │
│    `zfs set compression=lz4...` │               │    the KVM/Firecracker device.  │
└─────────────────────────────────┘               └─────────────────────────────────┘
4. Rust Structural Serialization Blueprint (gitops_storage.rs)

This blueprint outlines the data models and file parsing logic inside the compiled Rust control plane, using zero-cost type abstractions to handle file changes.
// Unified Rust Structural Module Map: aether-gitops-storage

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageClassParameters {
#[serde(rename = "zpool")]
pub zpool_name: Option<String>,
#[serde(rename = "datasetPath")]
pub dataset_path: Option<String>,
pub compression: String,
pub recordsize: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageClassSpec {
pub provisioner: String, // "freebsd.zfs" | "linux.lvm"
pub parameters: StorageClassParameters,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageClassManifest {
pub metadata: HashMap<String, String>,
pub spec: StorageClassSpec,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComputeSpec {
pub vcpus: u32,
#[serde(rename = "memoryBytes")]
pub memory_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkloadStorageSpec {
#[serde(rename = "storageClassName")]
pub storage_class_name: String,
#[serde(rename = "rootVolumeSizeGB")]
pub root_volume_size_gb: u32,
#[serde(rename = "baseImage")]
pub base_image: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkloadSpec {
#[serde(rename = "runtimeRequirement")]
pub runtime_requirement: String,
pub compute: ComputeSpec,
pub storage: WorkloadStorageSpec,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkloadManifest {
pub metadata: HashMap<String, String>,
pub spec: WorkloadSpec,
}

/// Core GitOps File Parser Engine
pub struct AetherManifestParser {
pub base_manifest_path: PathBuf,
}

impl AetherManifestParser {
pub fn new(path: PathBuf) -> Self {
Self { base_manifest_path: path }
}

    /// Reads a target text file path and attempts safe Rust structural validation mapping
    pub fn parse_workload_manifest(&self, file_content: &str) -> Result<WorkloadManifest, String> {
        serde_yaml::from_str::<WorkloadManifest>(file_content)
            .map_err(|e| format!("YAML Parsing structural violation error: {}", e))
    }

    /// Computes the exact host-level shell command execution context required for storage slicing
    pub fn generate_storage_mapping_commands(&self, workload: &WorkloadManifest, storage_class: &StorageClassManifest) -> Vec<String> {
        let mut commands = Vec::new();
        let vm_name = workload.metadata.get("name").unwrap_or(&"unknown-vm".to_string()).clone();
        
        if storage_class.spec.provisioner == "freebsd.zfs" {
            let zpool = storage_class.spec.parameters.zpool_name.as_deref().unwrap_or("zroot");
            let base_ds = storage_class.spec.parameters.dataset_path.as_deref().unwrap_or("zroot/vm");
            let base_image = &workload.spec.storage.base_image;

            // Generate optimized copy-on-write atomic execution pipeline instructions
            commands.push(format!("zfs clone {}/templates/{}@snapshot {}/{}", zpool, base_image, base_ds, vm_name));
            commands.push(format!("zfs set compression={} {}/{}", storage_class.spec.parameters.compression, base_ds, vm_name));
            commands.push(format!("zfs set recordsize={} {}/{}", storage_class.spec.parameters.recordsize, base_ds, vm_name));
        } else if storage_class.spec.provisioner == "linux.lvm" {
            // Generate thin-provisioned fallback blocks for Linux worker host nodes
            commands.push(format!("lvcreate -V {}G --thinpool vg_aether/tp_silver --name lv_{}", workload.spec.storage.root_volume_size_gb, vm_name));
        }
        
        commands
    }
}
5. SME Operational Guardrails: Storage & Quorum Edge Cases

5.1 The Local Storage Orphan Problem

Because our "just enough orchestration" model uses a shared-nothing design—where VMs run on the fast local disks of individual blades—if Pipeline B (The Deadman Switch Failover Loop) triggers a STONITH power fence on a failed blade, the data writes stored on that specific node's drives are isolated until the blade comes back online.
The v1alpha Mitigation Rule: Workloads that maintain strict structural state configurations (like databases) should be declared with an immutable template mapping, forcing data persistence directories to target an external network endpoint. Alternatively, the local node daemon can run a low-priority background thread to mirror ZFS snapshots over the 10Gb Virtual Connect midplane backplane to a hot-standby blade using:
zfs send zroot/vm/db-replica-09@hourly | ssh blade-02 zfs recv zroot/vm/db-replica-09
[1]
5.2 Storage Capacity Enforcement / Overcommit Protection

To prevent a single tenant from filling up a blade's 3.84TB local SSD storage array and crashing the host OS kernel, the local Rust daemon evaluates storage limits before submitting a reverse-bid.
If the remaining local pool disk space drops below 15% capacity, the local node's bidding calculation function sets the storage metric modifier to -1. This automatically disqualifies the blade from hosting any new virtual machines until the administrator purges old local datasets.
This design provides a type-safe blueprint for mapping declarative specifications directly to bare-metal storage pools. If you want to move forward with the hardware integration, let me know if we should define the setup configurations for the HPE Virtual Connect 10Gb physical networking layer or map out the gRPC secure token exchange parametersfor the cluster control plane.

[1] https://blog.devgenius.io

