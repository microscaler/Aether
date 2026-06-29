To move this into a production-ready compiled state, we must shift from the Python proof-of-concept to a highly performant, type-safe Rust implementation. [1]
Building the local agent daemon in Rust perfectly matches your structural goal: it compiles to a single, zero-dependency binary, maintains a near-zero memory footprint (frequently under 15MB RSS), and handles async event processing with zero-cost abstractions via tokio.
Here is the comprehensive gRPC Protobuf definition, the production-grade Rust implementation using tonic, and the corresponding architecture update for your PRD. [2]
1. The gRPC Protocol Buffer Definition (aether.proto)

This contract defines the asynchronous bidirectional communication between your central cluster controller and the autonomous local node daemons.
syntax = "proto3";
package aether.infra.v1alpha1;

// The Compute Plane Provisioning Engine Service
service AetherComputeEngine {
// Broadcast endpoint for nodes to calculate and return an autonomous reverse bid
rpc RequestReverseBid (BidRequest) returns (BidResponse);

// Concrete execution instruction sent exclusively to the winning node
rpc ExecuteProvisioning (ProvisionRequest) returns (ProvisionResponse);
}

message BidRequest {
string deployment_uid = 1;
string workload_name = 2;
string runtime_requirement = 3; // "bhyve", "firecracker", "kvm"
int32 requested_vcpus = 4;
int64 requested_memory_bytes = 5;
}

message BidResponse {
string node_id = 1;
string deployment_uid = 2;
int32 bid_score = 3;            // -1 represents hard rejection (OOM/Incompatible)
string status_message = 4;
}

message ProvisionRequest {
string deployment_uid = 1;
string workload_name = 2;
string guest_os = 3;
string requested_ip = 4;
string admin_user = 5;
string ssh_public_key = 6;
}

message ProvisionResponse {
enum StatusCode {
SUCCESS = 0;
ZFS_CLONE_FAILED = 1;
ISO_COMPILATION_FAILED = 2;
HYPERVISOR_START_FAILED = 3;
}
StatusCode code = 1;
string node_id = 2;
string message = 3;
string generated_iso_path = 4;
}
2. Rust Implementation (main.rs)

This code implements the gRPC server using tonic and tokio. It handles memory telemetry polling, reverse-bid score calculation, dynamic Cloud-Init configuration string rendering, and utilizes native asynchronous OS command binding to orchestrate makefs and zfs. [3, 4, 5]
Add these dependencies to your Cargo.toml: [6]
[dependencies]
tokio = { version = "1.0", features = ["full"] }
tonic = "0.11"
prost = "0.12"
sysinfo = "0.30" # Ultra-lightweight cross-platform telemetry framework
async-trait = "0.1"
The compiled Rust service binary:
use std::process::Command as SyncCommand;
use tokio::process::Command as AsyncCommand;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use tonic::{transport::Server, Request, Response, Status};
use sysinfo::{System, SystemExt};

// Import generated gRPC bindings from proto build
pub mod aether_proto {
tonic::include_proto!("aether.infra.v1alpha1");
}

use aether_proto::aether_compute_engine_server::{AetherComputeEngine, AetherComputeEngineServer};
use aether_proto::{BidRequest, BidResponse, ProvisionRequest, ProvisionResponse};

const NODE_ID: &str = "blade-16-freebsd";
const SUPPORTED_RUNTIME: &str = "bhyve";
const MEMORY_CHANNEL_COUNT: f64 = 2.0; // HW constraint alert

pub struct LocalAetherNode {
// Shared state if tracking metrics across multiple parallel execution loops
}

#[tonic::async_trait]
impl AetherComputeEngine for LocalAetherNode {

    // --- Phase 1: The Autonomous Reverse-Bidding Logic ---
    async fn request_reverse_bid(
        &self,
        request: Request<BidRequest>,
    ) -> Result<Response<BidResponse>, Status> {
        let req = request.into_inner();
        
        // Immediate rejection if runtime requirement target mismatches node type
        if req.runtime_requirement != SUPPORTED_RUNTIME {
            return Ok(Response::new(BidResponse {
                node_id: NODE_ID.to_string(),
                deployment_uid: req.deployment_uid,
                bid_score: -1,
                status_message: "Incompatible hypervisor runtime engine requested.".to_string(),
            }));
        }

        // Query memory telemetry dynamically via lightning-fast raw platform hooks
        let mut sys = System::new_all();
        sys.refresh_memory();
        let available_mem_bytes = sys.available_memory();

        if (req.requested_memory_bytes as u64) >= available_mem_bytes {
            return Ok(Response::new(BidResponse {
                node_id: NODE_ID.to_string(),
                deployment_uid: req.deployment_uid,
                bid_score: -1,
                status_message: "Hard rejection: OOM boundary violation risk flagged.".to_string(),
            }));
        }

        // Core reverse-bidding calculation matrix
        let base_score: f64 = 1000.0;
        let memory_utilization_ratio = (req.requested_memory_bytes as f64) / (available_mem_bytes as f64);
        
        // Penalize the bid score because our physical channels are unpopulated (only 2 out of 12 filled)
        let bandwidth_penalty = (1.0 - (MEMORY_CHANNEL_COUNT / 12.0)) * 300.0;
        let calculated_score = base_score - (memory_utilization_ratio * 500.0) - bandwidth_penalty;
        
        let final_score = (calculated_score.max(1.0)) as i32;

        Ok(Response::new(BidResponse {
            node_id: NODE_ID.to_string(),
            deployment_uid: req.deployment_uid,
            bid_score: final_score,
            status_message: format!("Accepting workload bidding profile. Score: {}", final_score),
        }))
    }

    // --- Phase 2: Just-In-Time Provisioning Engine ---
    async fn execute_provisioning(
        &self,
        request: Request<ProvisionRequest>,
    ) -> Result<Response<ProvisionResponse>, Status> {
        let req = request.into_inner();
        let target_vm_dir = format!("/zroot/vm/{}", req.workload_name);
        let iso_output_path = format!("{}/seed.iso", target_vm_dir);

        // Step A: Trigger native asynchronous ZFS dataset cloning sequence
        let zfs_status = AsyncCommand::new("zfs")
            .args(&[
                "clone",
                &format!("zroot/vm/templates/{}@snapshot", req.guest_os),
                &format!("zroot/vm/{}", req.workload_name)
            ])
            .status()
            .await;

        if zfs_status.is_err() || !zfs_status.unwrap().success() {
            return Ok(Response::new(ProvisionResponse {
                code: 1, // ZFS_CLONE_FAILED
                node_id: NODE_ID.to_string(),
                message: "ZFS Storage snapshot clone execution failed natively.".to_string(),
                generated_iso_path: "".to_string(),
            }));
        }

        // Step B: Generate dynamic Cloud-Init text file blocks using native memory buffers
        let user_data = format!(
            "#cloud-config\nusers:\n  - name: {}\n    ssh_authorized_keys:\n      - {}\nruncmd:\n  - echo 'Provisioned' > /etc/motd\n",
            req.admin_user, req.ssh_public_key
        );
        let meta_data = format!("local-hostname: {}\ninstance-id: i-{}\n", req.workload_name, req.deployment_uid);
        let network_config = format!(
            "network:\n  version: 2\n  ethernets:\n    vtnet0:\n      dhcp4: false\n      addresses: [{}/24]\n      gateway4: 10.20.16.1\n",
            req.requested_ip
        );

        // Create an isolated compilation workspace inside a temporary directory context
        let tmp_dir = tempfile::tempdir()?;
        let tmp_path = tmp_dir.path();

        File::create(tmp_path.join("user-data"))?.write_all(user_data.as_bytes())?;
        File::create(tmp_path.join("meta-data"))?.write_all(meta_data.as_bytes())?;
        File::create(tmp_path.join("network-config"))?.write_all(network_config.as_bytes())?;

        // Step C: Execute zero-dependency binary compilation via native FreeBSD toolchain
        let makefs_status = AsyncCommand::new("makefs")
            .args(&["-t", "cd9660", "-o", "rockridge", &iso_output_path, tmp_path.to_str().unwrap()])
            .status()
            .await;

        if makefs_status.is_err() || !makefs_status.unwrap().success() {
            return Ok(Response::new(ProvisionResponse {
                code: 2, // ISO_COMPILATION_FAILED
                node_id: NODE_ID.to_string(),
                message: "FreeBSD native makefs execution pipeline error.".to_string(),
                generated_iso_path: "".to_string(),
            }));
        }

        // Step D: Spin up the bhyve VirtIO hypervisor core execution instance
        let vm_start_status = AsyncCommand::new("vm")
            .args(&["start", &req.workload_name])
            .status()
            .await;

        if vm_start_status.is_err() || !vm_start_status.unwrap().success() {
            return Ok(Response::new(ProvisionResponse {
                code: 3, // HYPERVISOR_START_FAILED
                node_id: NODE_ID.to_string(),
                message: "Hypervisor execution start hook tracking error.".to_string(),
                generated_iso_path: iso_output_path,
            }));
        }

        Ok(Response::new(ProvisionResponse {
            code: 0, // SUCCESS
            node_id: NODE_ID.to_string(),
            message: "Autonomous provisioning protocol completed successfully.".to_string(),
            generated_iso_path: iso_output_path,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
let addr = "[::1]:50051".parse()?;
let node_service = LocalAetherNode {};

    println!("Aether Node gRPC Server initializing natively on {}...", addr);

    Server::builder()
        .add_service(AetherComputeEngineServer::new(node_service))
        .serve(addr)
        .await?;

    Ok(())
}
3. PRD & Architectural Adjustments for Rust + gRPC

Performance & Binary Requirements

Compilation Boundary: The daemon must compile statically against musl (on Linux blades) and build cleanly on FreeBSD using native toolchains without linking heavy external runtime execution engines.
Memory Management: The service must completely avoid garbage collection pauses. Real-time telemetry monitoring loops (tracking memory wait cycles and CPU registration states) must execute deterministically inside thread pools managed by tokio. [7]
gRPC Network Channel Resilience

Multiplexing Connections: The gRPC interface must leverage HTTP/2 multiplexing over the c7000's Virtual Connect backend midplane (VLAN 10).
Keep-Alive Guardrails: The node daemon will drop dead or hung control connections automatically using a 10-second TCP keep-alive timeout window. If the central control bus fails, the local Rust engine continues managing existing local bhyve instances autonomously, entering a standalone "Survival State" until communication is re-established.
Would you like to build out the build script configuration (build.rs) to automatically handle code generation for the protobuf compilation step on your FreeBSD target environments? Or should we look at the corresponding central rust orchestrator logic that collects the streaming reverse-bids from the 16 separate endpoints? [8]

[1] https://github.com
[2] https://dev.to
[3] https://medium.com
[4] https://dev.to
[5] https://dev.to
[6] https://github.com
[7] https://ritik-chopra28.medium.com
[8] https://users.rust-lang.org



On Sun, 28 Jun 2026 at 21:10, Charles Sibbald <casibbald@gmail.com> wrote:
1. Document Control & Architectural Context

Document Type: Supplemental Product Requirement Document (PRD) & Technical Spec
Component System: Project Aether Dynamic Provisioning Subsystem (aether-provisioner)
Target Environments: FreeBSD 14+ Nodes running bhyve + vm-bhyve within an HPE c7000 Chassis.
The Lifecycle Integration Challenge

In a decentralized reverse-bidding cluster, the central orchestrator does not know which node will win a workload until the 250ms bidding window closes.
Because Cloud-Init data often contains node-specific networking parameters (such as static IPs mapped to the winning blade's Virtual Connect switch port, local routing tables, and unique chassis hostname assignments), the seed.isocannot be pre-generated by the central controller.
Instead, the central orchestrator passes an abstract Intent Payload. The winning node must dynamically intercept this intent payload, fetch its local hardware constraints, generate the raw Cloud-Init string configurations natively, compile the binary seed.iso, and mount it—all within a targeted sub-second execution window.
2. Supplemental PRD: Dynamic Metadata Provisioning

Functional Requirements

FR-META-01 (Just-In-Time Generation): The worker node must compile the seed.iso locally after winning the workload bid, immediately preceding the hypervisor execution phase.
FR-META-02 (Stateless Orchestration): The central control bus shall pass only variables representing user intent (e.g., public keys, requested usernames). The winning node's local daemon must dynamically calculate host-level realities (e.g., target tap interface mapping, storage pathing, network gateway topologies matching its specific blade slot).
FR-META-03 (No External Dependencies): The ISO generation engine must rely strictly on native FreeBSD base utilities (makefs). It must not require bloated heavy dependencies like genisoimage or mkisofs to maintain the target 256MB RAM daemon footprint limit.
FR-META-04 (Automated Cleanup): Ephemeral metadata generation artifacts must be cleaned up or safely stored within the VM’s specific ZFS dataset to prevent file system pollution on the local 3.84TB enterprise SSD storage pool.
3. Technical Engineering Design

Data Flow Topology

[ Central Control Bus ] ──► (Dispatches Win Notification + Intent JSON)
│
▼
[ Winning FreeBSD Blade Daemon ]
│
┌────────────────────────────┴────────────────────────────┐
▼                                                         ▼
1. Fetch Node Telemetry                                   2. Parse Intent JSON
   (Gateways, DNS, Interfaces)                               (SSH Keys, Packages)
   │                                                         │
   └────────────────────────────┬────────────────────────────┘
   ▼
   [ Compile YAML Configuration ]
   ├── user-data  (Users, RunCmds)
   └── meta-data  (Instance-ID, Hostname)
   │
   ▼
   [ Execute `makefs` Pipeline ]
   └── Generates: `seed.iso`
   │
   ▼
   [ Trigger `bhyve` Execution ]
4. Implementation Code: The JIT Cloud-Init & Bidding Engine

The following Python script extends the previous daemon design. It sits on the FreeBSD host, acts as the reverse bidder, handles win-notifications, extracts local networking variables dynamically, writes the Cloud-Init configuration files, and outputs a bootable seed.iso.
#!/usr/bin/env python3
"""
Aether JIT Provisioning Engine & Reverse Bidder
Platform: FreeBSD 14+ / Hypervisor: bhyve
"""

import os
import sys
import json
import shutil
import tempfile
import subprocess

NODE_ID = "blade-16-freebsd"
VIRTUAL_CONNECT_SWITCH = "public-virtual-connect"

def get_local_networking():
"""
Queries local host settings to dynamically build network-data.
In production, this queries the specific Virtual Connect slot context.
"""
return {
"interface": "vtnet0", # The VirtIO interface name inside the guest
"gateway": "10.20.16.1", # Local Blade 16 midplane gateway
"dns": "1.1.1.1",
"netmask": "255.255.255.0"
}

def compile_cloud_init_strings(intent_spec, local_net, instance_name):
"""Generates valid NoCloud Cloud-Init configuration text blocks."""

    # Generate user-data string payload
    user_data = f"""#cloud-config
autoinstall: {{version: 1}}
user-data:
users:
- name: {intent_spec.get('adminUser', 'aether-root')}
gecos: Aether Managed User
sudo: ALL=(ALL) NOPASSWD:ALL
shell: /bin/bash
lock_passwd: true
ssh_authorized_keys:
- {intent_spec.get('sshKey', '')}
package_update: true
package_upgrade: true
packages:
- qemu-guest-agent
- curl
- htop
runcmd:
- [ systemctl, enable, --now, qemu-guest-agent ]
- [ echo, "Successfully Booted on {NODE_ID}", ">", /etc/motd ]
"""

    # Generate meta-data string payload with strict Cloud-Init structural requirements
    meta_data = f"""local-hostname: {instance_name}
instance-id: i-{intent_spec.get('uid', '00000000')}
"""

    # Generate network-config V2 payload mapping to the guest's VirtIO pipeline
    network_config = f"""network:
version: 2
ethernets:
{local_net['interface']}:
dhcp4: false
addresses:
- {intent_spec.get('requestedIP')}/{local_net['netmask'].split('.').count('255') * 8}
gateway4: {local_net['gateway']}
nameservers:
addresses:
- {local_net['dns']}
"""
return user_data, meta_data, network_config

def generate_seed_iso(instance_name, user_data, meta_data, network_config):
"""
Compiles raw configurations into a standardized NoCloud seed.iso
using FreeBSD's ultra-lightweight native makefs binary.
"""
target_vm_dir = f"/zroot/vm/{instance_name}"
iso_output_path = os.path.join(target_vm_dir, "seed.iso")

    # Create a temporary workspace directory to house raw file structures
    with tempfile.TemporaryDirectory() as tmpdir:
        # Write metadata requirements matching standard NoCloud cloud-init specifications
        with open(os.path.join(tmpdir, "user-data"), "w") as f:
            f.write(user_data)
        with open(os.path.join(tmpdir, "meta-data"), "w") as f:
            f.write(meta_data)
        with open(os.path.join(tmpdir, "network-config"), "w") as f:
            f.write(network_config)
            
        # Ensure the destination ZFS volume directory mount path exists
        os.makedirs(target_vm_dir, exist_ok=True)
        
        # Execute legacy-free ISO compilation via native FreeBSD makefs toolchain
        # -t cd9660 guarantees standard ISO-9660 file formatting boundaries
        cmd = ["makefs", "-t", "cd9660", "-o", "rockridge", iso_output_path, tmpdir]
        result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        
        if result.returncode != 0:
            raise RuntimeError(f"FreeBSD makefs pipeline failure: {result.stderr}")
            
    return iso_output_path

def process_win_notification(event_payload_string):
"""Executes the execution block once the central bus flags this node as the winner."""
try:
event = json.loads(event_payload_string)
instance_name = event["metadata"]["name"]
intent_spec = event["spec"]["provisioningIntent"]

        # Step 1: Query local runtime realities
        local_net = get_local_networking()
        
        # Step 2: Compile payload strings dynamically
        user_data, meta_data, network_config = compile_cloud_init_strings(
            intent_spec, local_net, instance_name
        )
        
        # Step 3: Call the binary assembly engine
        iso_path = generate_seed_iso(instance_name, user_data, meta_data, network_config)
        
        return json.dumps({
            "status": "SUCCESS",
            "node": NODE_ID,
            "generatedISO": iso_path,
            "message": f"Dynamic cloud-init seed wrapped cleanly inside {instance_name}"
        })
        
    except Exception as e:
        return json.dumps({"status": "PROVISIONING_FAILED", "error": str(e)})

if __name__ == "__main__":
# Simulated incoming event loop payload representing an orchestrator WIN notification
simulated_win_event = {
"apiVersion": "aether.infra.system/v1alpha1",
"kind": "AetherWinNotification",
"metadata": {"name": "k8s-worker-node-p01"},
"spec": {
"provisioningIntent": {
"uid": "f47ac10b-58cc",
"adminUser": "kube-admin",
"sshKey": "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJuAetherTestKey",
"requestedIP": "10.20.16.55"
}
}
}

    # Execute the just-in-time runtime processing stack simulation
    print(process_win_notification(json.dumps(simulated_win_event)))
5. Post-Execution VM Configuration Template

Once the script completes, the target VM configuration (/zroot/vm/k8s-worker-node-p01/k8s-worker-node-p01.conf) is updated by your orchestration logic to mount the generated seed.iso directly alongside the OS root image:
# --- Target Hypervisor Runtime Parameters Configuration File ---
loader="uefi"
cpu=4
memory=16GB

# Storage mapping linking root template disk and transient cloud-init media
disk0_type="virtio-blk"
disk0_name="disk0.img"      # Instant ZFS snapshot clone layout
disk1_type="virtio-blk"
disk1_name="seed.iso"       # JIT generated NoCloud packet built via makefs

# Network profile linkage mapping dynamically to physical midplane
network0_type="virtio-net"
network0_switch="public-virtual-connect"

uuid="f47ac10b-58cc-4a92-9111-ccaa88990033"
utctime="yes"
6. Edge Case Handling & Lifecycle Verification

Race Condition Handling (ISO Locked/In-Use)

If your declarative controller requests a rapid state mutation (e.g., changing the target static IP or injecting a fresh SSH key while the machine is executing a reboot instruction), the local node daemon must intercept this command. It must generate a new config tree into a separate payload string (seed_next.iso), update the .conf descriptor, and call vm reload <name> to force bhyve to unmount the old block array and latch onto the updated metadata block.
Validation Verification Matrix

To ensure your custom reverse-bidding controller can verify that the local node generated the payload properly without breaking execution streams, the local daemon must emit a SHA256 checksum of the generated seed.iso back onto the status channel of the Aether Central Control Bus. If the hash matches the intended state parameters, the deployment is marked healthy.
How would you like to handle the communication layer for this payload delivery? We can look at implementing a asynchronous listener using NATS JetStream or a lightweight gRPC event stream that feeds directly into this Python automation runner.






On Sun, 28 Jun 2026 at 21:03, Charles Sibbald <casibbald@gmail.com> wrote:
Product Requirement Document (PRD) & Product Architecture Design

Project Name: Project Aether (An Autonomous, Decentralized Multi-Tenant Compute Plane)
Target Hardware: HPE BladeSystem c7000 Enclosure (16 x HPE ProLiant BL460c Gen10 Nodes, 640 Cores, 4TB RAM, 122TB NVMe/SAS Storage)
1. Executive Summary & Core Concept

1.1 Product Vision

Project Aether is a highly efficient, declarative, open-source infrastructure controller that treats an HPE BladeSystem c7000 chassis as an autonomous, self-balancing marketplace of bare-metal computing resources. Inspired by Kubernetes Cluster API (CAPI), Aether manages multi-tenant workloads including Kubernetes worker groups, Firecracker microVMs, and isolated Linux VMs.
1.2 The Decentralized "Reverse Bid" Mechanism

Traditional orchestration layers rely on a centralized scheduler (like kube-scheduler) that maintains a global state database to push VMs onto specific target hardware. Aether replaces this model with an Autonomous Local Daemon Engine on each blade node.
When a client requests a new virtual instance, the centralized controller broadcasts a standardized workload specification payload (defining required CPU, memory, IOPS, and network isolation boundaries) to the chassis midplane. Instead of the controller assigning a host:
Each blade runs a localized daemon that evaluates its real-time telemetry, memory channel bandwidth limits, and current SLA degradation parameters.
Nodes calculate an internal "Cost/Efficiency Score" and submit an automated Reverse Bid to host the instance.
The orchestrator accepts the highest-value bid, delegating execution entirely to the winning node.
┌──────────────────────────────┐
│  Aether Central Control Bus  │
└──────────────┬───────────────┘
│
Broadcast Request │ (YAML Spec: 4 vCPU, 16GB RAM)
▼
┌─────────────────────────┼─────────────────────────┐
│                         │                         │
▼                         ▼                         ▼
┌──────────────┐          ┌──────────────┐          ┌──────────────┐
│   Blade 01   │          │   Blade 02   │          │   Blade 16   │
│ (FreeBSD/bhyve)│        │  (Linux/KVM) │          │ (Linux/Fire) │
└──────┬───────┘          └──────┬───────┘          └──────┬───────┘
│                         │                         │
│ Bid: 82ms / 0% Degrad   │ Bid: 11ms / 4% Degrad   │ Bid: REJECT (OOM)
└─────────────────────────┼─────────────────────────┘
│
▼
┌──────────────────────────────┐
│ Winner Selected: Blade 02    │
└──────────────────────────────┘
2. Product Requirements Document (PRD)

2.1 Target Audience & Use Cases

Internal DevOps / Platform Engineering Teams: Rapidly provisioning isolated, ephemerally cycled test frameworks.
Serverless / Function-as-a-Service (FaaS) Operators: Running untrusted code loops safely inside lightweight Firecracker microVM execution engines.
Multi-Tenant Linux Infrastructure Builders: Isolating mission-critical backend workloads with absolute system boundaries while optimizing hardware density.
2.2 Functional Requirements

FR-01: Declarative Lifecycle Management

The infrastructure state engine must track compute targets using a declarative syntax. Modifying the desired state configuration must trigger an immediate automated reconciliation cascade.
FR-02: Heterogeneous Architecture Runtime Support

The execution planes must simultaneously configure and maintain three distinct infrastructure runtimes across the 16 distinct server blades:
Native Bare-Metal Linux KVM Layer: Running standard Linux VMs and native Docker/Containerd engines.
Firecracker MicroVM Layer: Leveraging /dev/kvm to run sub-100ms ephemeral isolation engines.
FreeBSD bhyve Architecture Layer: Leveraging ZFS snapshots and legacy-free VirtIO models.
FR-03: Autonomous Reverse Bidding Loop

System nodes must monitor local execution matrices every 500ms.
Nodes must evaluate structural limits (including the Intel Xeon Gold 6138 6-channel memory population limits) before bidding on a payload.
Tie-breaking algorithms must optimize for chassis power usage effectiveness (PUE) and even write-wear leveling across local enterprise SAS SSD pools.
FR-04: Declarative Storage Pipeline

The cluster must deploy near-instant VM creation using underlying storage layers. For FreeBSD nodes, this requires native ZFS clones; for Linux hosts, thin-provisioned LVM snapshots or overlay blocks.
2.3 Non-Functional Requirements

NFR-01: Low Orchestration Overhead

The localized hypervisor daemon on each blade must use less than 1.5% of total system CPU cycles and maintain a maximum memory footprint of 256MB.
NFR-02: Zero Nesting Virtualization Penalties

Workloads must be assigned directly to nodes capable of running them natively. For example, Firecracker payloads must only receive bids from bare-metal Linux hosts to preserve direct execution pipelines.
NFR-03: Multi-Node Resiliency

The cluster plane must tolerate the unannounced failure of up to two physical blade server nodes without losing the underlying cluster state registry or cluster quorum.
3. High-Level Architectural Design

3.1 Network Topology & Midplane Architecture

The system utilizes the HPE Virtual Connect FlexFabric 10Gb modules built into the c7000 backplane.
┌────────────────────────────────────────────────────────────────────────┐
│                    HPE Virtual Connect 10Gb Fabrics                    │
└────────┬───────────────────────────────┬───────────────────────────────┘
│ Trunked VLANs (10, 20, 30)    │ Trunked VLANs (10, 20, 30)
▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│ Blade Node 01 (Linux)   │     │ Blade Node 16 (FreeBSD) │
│ Physical: `enp2s0`      │     │ Physical: `cxl0`        │
│                         │     │                         │
│ ├── VLAN 10 (Aether Bus)│     │ ├── VLAN 10 (Aether Bus)│
│ ├── VLAN 20 (Kube Pods) │     │ ├── VLAN 20 (bhyve Net) │
│ └── Bridge: `br-aether` │     │ └── Switch: `vm-switch` │
└─────────────────────────┘     └─────────────────────────┘
Network isolation is enforced directly on the physical backplane:
VLAN 10 (Control Bus): A secure, isolated private VLAN used exclusively for cluster state distribution and reverse bid negotiations.
VLAN 20+ (Tenant Workload Networks): Trunked data networks passed down directly into each blade's virtual networking abstractions (br-lan on Linux, vm-switch on FreeBSD vm-bhyve).
3.2 Storage Framework

Linux Worker Nodes: Configured with thin-provisioned lvm2 storage pools or standard QCOW2 backing chains built onto the local 3.84TB Enterprise SAS SSD storage blocks.
FreeBSD Worker Nodes: Formatted natively using ZFS (zroot). Base installation frameworks are kept as immutable snapshots (zroot/vm/templates/ubuntu-base@snap), allowing instances to clone instantly via ZFS backing trees.
4. Technical Engine Design & Code Implementation

4.1 Specification Definition (CRD Layout)

The central control plane exposes a unified API. The structure below defines a request submitted to the cluster via an asynchronous NATS or gRPC message bus.
{
"apiVersion": "aether.infra.system/v1alpha1",
"kind": "AetherVirtualDeployment",
"metadata": {
"uid": "a67b98d1-ef23-4b92-9111-ccaa88990033",
"name": "isolated-app-compute-04"
},
"spec": {
"runtimeRequirement": "bhyve",
"guestOS": "ubuntu-24.04-server",
"compute": {
"vcpus": 4,
"memoryBytes": 17179869184
},
"storage": {
"volumeSizeBytes": 53687091200
},
"networkInterface": {
"networkProfile": "public-virtual-connect"
}
}
}
4.2 The Local Node Autonomous Bidder Engine (Python Daemon)

The script below runs persistently on every node. It consumes broadcasted deployment requests, checks local metrics via native command interfaces, executes the bidding calculations, and returns an official reverse bid.
#!/usr/bin/env python3
"""
Aether Autonomous Bidder Daemon
Runs natively on cluster worker nodes to evaluate resource allocation availability.
"""

import os
import sys
import json
import subprocess

# Local Hardware Constant Configs
NODE_ID = "blade-16-freebsd"
SUPPORTED_RUNTIME = "bhyve"
MEMORY_CHANNEL_COUNT = 2  # Hardware limit quirk alert: Listing notes only 2 slots filled

def get_free_memory_kb():
"""Calculates active available system memory based on target OS platform."""
if sys.platform.startswith("freebsd"):
# Parse FreeBSD sysctl values for memory estimation
try:
pagesize = int(subprocess.check_output(["sysctl", "-n", "hw.pagesize"]).strip())
free_pages = int(subprocess.check_output(["sysctl", "-n", "vm.stats.vm.v_free_count"]).strip())
return (free_pages * pagesize) // 1024
except Exception:
return 0
else:
# Fallback to standard Linux /proc/meminfo parsing
if os.path.exists("/proc/meminfo"):
with open("/proc/meminfo", "r") as f:
for line in f:
if "MemAvailable" in line:
return int(line.split()[1])
return 0

def calculate_bid_score(request_spec):
"""
Computes an efficiency score. Higher scores denote optimal deployment targets.
Returns -1 if the node cannot safely satisfy the workload resource constraints.
"""
if request_spec["spec"]["runtimeRequirement"] != SUPPORTED_RUNTIME:
return -1  # Runtime mismatch

    req_mem_kb = request_spec["spec"]["compute"]["memoryBytes"] // 1024
    available_mem_kb = get_free_memory_kb()
    
    if req_mem_kb >= available_mem_kb:
        return -1  # Hard limit rejection (OOM prevention)

    # Core bidding logic scoring system
    # Penalizes deployment choices if memory bandwidth channels are constrained
    base_score = 1000
    memory_utilization_ratio = req_mem_kb / available_mem_kb
    bandwidth_penalty = (1.0 - (MEMORY_CHANNEL_COUNT / 12)) * 300  # Penalizes unpopulated channels
    
    final_score = base_score - (memory_utilization_ratio * 500) - bandwidth_penalty
    return max(1, int(final_score))

def process_broadcast_payload(json_payload_string):
"""Evaluates incoming workload specs and returns the node's reverse bid."""
try:
request = json.loads(json_payload_string)
score = calculate_bid_score(request)

        if score > 0:
            bid_response = {
                "node": NODE_ID,
                "deploymentUID": request["metadata"]["uid"],
                "bidScore": score,
                "status": "ACCEPT"
            }
        else:
            bid_response = {
                "node": NODE_ID,
                "deploymentUID": request["metadata"]["uid"],
                "bidScore": 0,
                "status": "REJECT"
            }
        return json.dumps(bid_response)
    except Exception as e:
        return json.dumps({"error": str(e), "status": "MALFORMED_REQUEST"})

if __name__ == "__main__":
# Example validation configuration simulating a broadcast receipt
sample_request = {
"apiVersion": "aether.infra.system/v1alpha1",
"kind": "AetherVirtualDeployment",
"metadata": {"uid": "a67b98d1", "name": "isolated-compute"},
"spec": {
"runtimeRequirement": "bhyve",
"compute": {"vcpus": 4, "memoryBytes": 8589934592}
}
}
print(process_broadcast_payload(json.dumps(sample_request)))
4.3 Node Provisioning Protocol Execution Payload (FreeBSD vm-bhyve Target)

Once the central bus chooses the winning node, the orchestrator issues a targeted execution directive. If a FreeBSD blade wins the bid, it uses its local storage architecture to provision the instance without requiring a persistent management agent:
#!/bin/sh
# --- Autonomous Execution Directive Script sent to the winning FreeBSD node ---
set -e

VM_NAME="isolated-app-compute-04"
TEMPLATE_NAME="ubuntu-24.04-server"
CONFIG_PATH="/zroot/vm/${VM_NAME}/${VM_NAME}.conf"

echo "=== Step 1: Executing ZFS Fast Storage Clone Pipeline ==="
# Instant clone using underlying ZFS dataset snapshots
if ! zfs list "zroot/vm/${VM_NAME}" >/dev/null 2>&1; then
zfs clone "zroot/vm/templates/${TEMPLATE_NAME}@snapshot" "zroot/vm/${VM_NAME}"
fi

echo "=== Step 2: Injecting Declarative Structural Configuration Payload ==="
cat << 'EOF' > "${CONFIG_PATH}"
# --- Autogenerated by Project Aether Orchestrator Cluster Plane ---
loader="uefi"
cpu=4
memory=16GB

# Native VirtIO Core Device Definitions
network0_type="virtio-net"
network0_switch="public-virtual-connect"
disk0_type="virtio-blk"
disk0_name="disk0.img"

# Operational System Settings
uuid="a67b98d1-ef23-4b92-9111-ccaa88990033"
utctime="yes"
auto_start="yes"
EOF

echo "=== Step 3: Resolving Local VirtIO Hypervisor Execution Runtime ==="
# Check run state and initiate execution context
CURRENT_STATE=$(vm status "${VM_NAME}" | awk '{print $2}')

if [ "${CURRENT_STATE}" != "running" ]; then
vm start "${VM_NAME}"
echo "Deployment successfully executed on target host."
else
echo "Deployment already matches requested execution state."
fi
5. Verification, Error Handling, & Edge Cases

5.1 Handling Split-Brain and Multi-Bid Collisions

Because bidding occurs asynchronously over an open network bus, two nodes could theoretically submit identical bid scores at the exact same millisecond.
The Resolution Engine: The central controller coordinates incoming responses within a strict 250ms window. If scores match exactly, a cryptographic tie-breaker selects the node with the lowest alphanumeric ID string (e.g., blade-02 wins over blade-09).
5.2 Mitigating Memory Bandwidth Degradation

As noted during initial component scoping, configuring the 256GB blades with only 2 physical sticks limits processing throughput to 16% of total capacity under heavy utilization.
The Mitigation Strategy: The telemetry gathering engine inside the Python daemon tracks hardware context values (sysctl dev.cpu or /proc/crypto). If memory wait cycles cross a 22% performance degradation threshold, the local agent automatically lowers its bid scores by 50% for memory-intensive workloads to prevent resource starvation.
5.3 Post-Deployment Health Check Loops

Once a node wins a bid and launches an instance, it must submit a cryptographic MEMBER_HEARTBEAT verification string back to the central data store every 5 seconds. If a blade stops communicating for more than three check cycles (15 seconds), the central orchestrator flags the blade as offline and automatically re-broadcasts the workload requirements for a new round of reverse bids.
This design document provides a comprehensive blueprint for building a declarative compute infrastructure tailored to the specific constraints and capabilities of your hardware configuration.
To proceed with this architecture, let me know:
Which message broker or coordination tool (NATS, gRPC/Etcd, or Redis Streams) do you want to use for the central communication bus?
Should we expand the template cloning logic to support custom cloud-init generation parameters (seed.iso) for dynamic user provisioning?
Do you want to build out the health telemetry collection dashboard?