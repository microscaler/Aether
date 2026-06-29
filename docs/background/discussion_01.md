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