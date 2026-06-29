To route traffic seamlessly between a containerized management Kubernetes cluster and the decentralized bare-metal blade interfaces, your design must bridge the logical cloud-native network layers inside Kubernetes with the physical backplane topologies configured in the HPE Virtual Connect (VC) modules.
The primary challenge is ensuring that the Aether Aggregator, running as a standard pod inside Kubernetes, can natively emit layer-2 tagged packets and maintain direct, un-NATed gRPC endpoints to the physical blades across VLAN 10 (Control Bus) and VLAN 999 (OOB Management). [1]
1. Physical to Logical Network Topology

We establish this bridge by running the management Kubernetes cluster on a dedicated Management / Utility Tier. This can be implemented using three physical top-of-rack (ToR) mini-appliances or three dedicated utility blades inside the chassis.
The physical interfaces of these Kubernetes nodes connect directly to the upstream enterprise ToR switches, which are linked to the rear external ports (X1-X4) of the c7000 Virtual Connect modules via LACP trunks.
[ Aether Aggregator Pod ]        [ K8S CNI Pod (Cilium/Multus) ]
│                                       │
▼ (Virtual veth pair)                   ▼ (Direct Layer-2 Pass-through)
┌────────────────────────────────────────────────────────┐
│           Kubernetes Node Linux Kernel Network Space   │
│   ├── `bond0` (Physical LACP Link Aggregation)        │
│   ├── `bond0.10` -> Bridge: `br-control` (VLAN 10)     │
│   └── `bond0.999` -> Bridge: `br-oob`    (VLAN 999)    │
└───────────────────────────┬────────────────────────────┘
│
▼ (Dual 10Gb DAC Physical Trunks)
┌────────────────────────────────────────────────────────┐
│            Top-of-Rack (ToR) Core Switches             │
│         (802.1Q Tagged Trunk Ports: 10, 11, 999)       │
└───────────────────────────┬────────────────────────────┘
│
▼ (HPE VC Uplink Ports: X1/X2)
┌────────────────────────────────────────────────────────┐
│     HPE c7000 Virtual Connect FlexFabric Modules       │
│   (Hardware-slices packets down the passive midplane)   │
└───────────────────────────┬────────────────────────────┘
│
┌─────────────────────┼─────────────────────┐
▼ (Slot 1 Midplane)   ▼ (Slot 2 Midplane)   ▼ (Slot 16 Midplane)
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│ Blade Node 01│      │ Blade Node 02│      │ Blade Node 16│
│ (Linux/KVM)  │      │ (Linux/KVM)  │      │(FreeBSD/bhyve)│
└──────────────┘      └──────────────┘      └──────────────┘
2. Advanced Kubernetes CNI Architecture (Multus + Cilium)

Standard Kubernetes networking (like default Flannel or Calico) encapsulates all pod traffic inside a single flat overlay network (like VXLAN or Geneve) assigned to a single host interface. This setup prevents the Aether Aggregator pod from interacting directly with independent bare-metal VLAN channels. [2, 3, 4, 5, 6]
To bypass this limitation, Project Aether implements a Dual-CNI Architecture utilizing Multus CNI paired with Cilium: [7, 8]
Cilium (Primary CNI): Handles standard Kubernetes cluster operations, pod-to-pod API traffic, internet egress, and FluxCD Git synchronization loops using safe, eBPF-driven routing policies. [9, 10, 11]
Multus CNI (Secondary CNI): Acts as a meta-plugin or "network plumber." It allows the Aether Aggregator pod to latch onto multiple physical interfaces simultaneously. Multus attaches a secondary, raw virtual interface directly inside the Aggregator pod's Linux network namespace, linking it to the host’s physical bond0.10 sub-interface.[12, 13, 14, 15]
2.1 Declarative NetworkAttachmentDefinition Custom Resource

This manifest instructs Multus how to construct the bridge pipeline from the physical ToR switches down into the K8S cluster substrate:
apiVersion: "k8s.cni.cncf.io/v1"
kind: NetworkAttachmentDefinition
metadata:
name: aether-control-midplane
namespace: tenant-system
spec:
config: '{
"cniVersion": "0.3.1",
"type": "bridge",
"bridge": "br-control",
"isGateway": false,
"ipam": {
"type": "static"
}
}'
2.2 Pod Deployment Infrastructure Binding Manifest

When deploying the Aether Aggregator controller pod, we attach the Multus network definition via an annotation block. This tells the K8S scheduler to inject the raw Virtual Connect midplane network card directly into the container runtime boundary: [16, 17]
apiVersion: apps/v1
kind: Deployment
metadata:
name: aether-aggregator-core
namespace: tenant-system
spec:
replicas: 2
template:
metadata:
annotations:
k8s.v1.cni.cncf.io/networks: tenant-system/aether-control-midplane
spec:
containers:
- name: aggregator-engine
image: aether-registry.local/infra/aggregator:v1alpha1
# Cilium assigns standard eth0 for internal K8S connectivity.
# Multus injects net1, which maps directly onto Virtual Connect VLAN 10.
3. The Pod-Level Routing Table Matrix

Inside the compiled Rust Aether Aggregator container pod, the Linux kernel manages two separate routing tables to prevent traffic overlapping or routing asymmetry:
# Execute interactive routing table dump inside the Aggregator controller pod:
$ ip route show

# Interface `eth0` (Managed via Cilium eBPF for standard Cloud/GitOps operations)
default via 10.244.0.1 dev eth0 proto cilium
10.244.0.0/16 dev eth0 proto kernel scope link src 10.244.1.45

# Interface `net1` (Managed via Multus, bridged directly to Virtual Connect Midplane VLAN 10)
10.20.10.0/24 dev net1 proto kernel scope link src 10.20.10.2
Traffic Separation Logic:

When FluxCD triggers an infrastructure modification event, the pod communicates with the external Git repository or K8S API endpoint using the default gateway routed out of eth0.
When the Auction Loop or Heartbeat Monitor triggers an action, the Rust binary binds explicitly to the local interface address assigned to net1 (10.20.10.2). Packets bypass the Kubernetes overlay entirely, exiting the physical network interface with an 802.1Q VLAN 10 tag. They cross the ToR switch fabrics and pass down into the Virtual Connect midplane backplane to hit the blade daemons with sub-millisecond latencies.
4. SME Operational Guardrails: Network Fault Isolation

4.1 Pod Eviction and ARP Cache Invalidation (Gratuitous ARP)

If a physical hardware node running the Aether Aggregator pod fails, Kubernetes detects the node loss and re-schedules the Aggregator pod onto a secondary management host node. [18]
The Networking Challenge: The external ToR switches and the backend Virtual Connect modules will still have the old host node's physical MAC address cached in their CAM tables for that pod's static IP (10.20.10.2), leading to dropped frames during the migration.
The Automation Fix: The Rust Aggregator binary executes a network initialization routine immediately upon container startup. It spawns a thread that broadcasts a series of Gratuitous ARP (GARP) packets out of its net1interface. This forces the upstream ToR switches and the rear Virtual Connect modules to instantly flush their layer-2 cache structures and map 10.20.10.2 to the new host node's interface path, restoring control plane connectivity in under 200 milliseconds.
4.2 Handling Broadcast Storm Isolation

Because the reverse-bidding system relies on broadcasting workload requests over a shared layer-2 midplane bridge layer, a malfunctioning blade daemon could loop packets and generate a broadcast storm, saturating the control bus and causing a cluster-wide failure.
The Prevention Strategy: Within the Virtual Connect module configuration parameters, Storm Control is explicitly enabled on all internal downlink server ports assigned to the Aether-Control-Bus profile. If broadcast or multicast traffic from any single blade slot crosses 1% of the total allocated interface capacity (10Mbps), the VC hardware instantly rate-limits the port, isolating the malfunctioning blade and preserving communication across the rest of the chassis backplane.
This networking design ensures a resilient connection between your cloud-native management tools and your raw bare-metal hardware. By offloading complex isolation tasks to the Virtual Connect hardware and using Multus inside Kubernetes, the platform avoids enterprise licensing constraints while maintaining performance and reliability. [19]
We have completed the architectural definitions for the core compute, storage, security, and networking layers of the platform. If you'd like to proceed, let me know if you want to focus on the declarative CRD spec interfaces for multi-tenant tenant isolation boundaries, or draft the automated shell-bootstrap scripts used to install the Rust aetherddaemon onto a clean, bare-metal FreeBSD blade.

[1] https://www.tp-link.com
[2] https://medium.com
[3] https://docs.oracle.com
[4] https://pmc.ncbi.nlm.nih.gov
[5] https://spacelift.io
[6] https://docs.redhat.com
[7] https://docs.oracle.com
[8] https://vegard.blog.engen.priv.no
[9] https://www.codecentric.de
[10] https://rstforum.net
[11] https://www.cncf.io
[12] https://hackmd.io
[13] https://docs.cloud.google.com
[14] https://thamizhelango.medium.com
[15] https://www.redhat.com
[16] https://documentation.nokia.com
[17] https://www.mdpi.com
[18] https://www.nutanix.com
[19] https://www.redhat.com