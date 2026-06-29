Architectural Specification: HPE Virtual Connect 10Gb Physical Networking Layer

Component Identifier: aether-network-vc
Subsystem Context: Physical Midplane & Backplane Network Fabric
Target Hardware: 2 x HPE Virtual Connect FlexFabric 10Gb/24-port Modules (installed in interconnect bays 1 and 2 of the c7000 chassis) [1]
1. Network Philosophy & Midplane Architecture

The HPE BladeSystem c7000 midplane uses hardwired signal traces connecting each blade server slot to the interconnect bays at the rear of the chassis. To build a robust multi-tenant cloud environment without complex, resource-heavy software-defined networking (SDN) overlays, Project Aether offloads packet tagging, segregation, and high availability directly to the HPE Virtual Connect (VC) hardware layer. [2]
Instead of assigning distinct physical interfaces to different types of traffic on each blade, we configure HPE Flex-10 Multi-Module Link Aggregation (Mlag). This carves the physical dual-port 10Gb FlexibleLOM network cards (LOM1:1 and LOM1:2) on each BL460c Gen10 blade into four distinct virtual functions (FlexNICs) per port, for a total of eight virtual network interfaces per blade. This division is handled entirely in hardware by the VC module before the operating system even boots.
[ 16 x HPE ProLiant BL460c Gen10 Server Blades ]
│ (Dual-Port 10Gb FlexLOM Connections)
▼
┌────────────────────────────────────────────────────────┐
│            HPE c7000 Passive Midplane Backplane        │
└────────┬───────────────────────────────────────┬───────┘
│                                       │
▼ (Port 1 Hardwired Trace)              ▼ (Port 2 Hardwired Trace)
┌───────────────────────────┐           ┌───────────────────────────┐
│ Virtual Connect Module 1  │◄─────────►│ Virtual Connect Module 2  │
│    (Interconnect Bay 1)   │   XFI     │    (Interconnect Bay 2)   │
└────────┬──────────────────┘ Stacking  └────────┬──────────────────┘
│                     Links             │
│ LACP Uplink Trunk (Ports X1-X4)       │ LACP Uplink Trunk (Ports X1-X4)
▼                                       ▼
┌────────────────────────────────────────────────────────┐
│           To Core Enterprise Tor Switches              │
└────────────────────────────────────────────────────────┘
2. Flex-10 Hardware Slice Configuration Matrix

Each BL460c blade's physical dual-port NIC is divided into four distinct hardware FlexNIC interfaces. This hardware-level allocation optimizes bandwidth across the chassis midplane and separates infrastructure management from tenant application networks.
FlexNIC Instance	Host OS Mapping (Linux/FreeBSD)	Assigned Bandwidth (Min - Max)	Network Type	VLAN Binding Policy
FlexNIC 1 (Port 1)	eth0 / cxl0	1.0 Gbps - 10.0 Gbps	Aether Control Plane	Untagged / Native VLAN 10 (gRPC & Heartbeats)
FlexNIC 2 (Port 2)	eth1 / cxl1	1.0 Gbps - 10.0 Gbps	Aether Storage Plane	Untagged / Native VLAN 11 (ZFS Snap sync / NVMe-oF)
FlexNIC 3 (Port 1)	eth2 / cxl2	2.0 Gbps - 10.0 Gbps	Tenant Workload Trunk	802.1Q Trunked VLANs 20-99 (K8s & Jails Data)
FlexNIC 4 (Port 2)	eth3 / cxl3	1.0 Gbps - 5.0 Gbps	OOB / Host Management	Untagged / Native VLAN 999 (Host SSH & Telemetry)
3. Virtual Connect CLI Profile Configuration Script

To configure a poor man's private cloud, an administrator must log into the Virtual Connect Manager (VCM) CLI via SSH and execute the following configuration script. This automates network provisioning across the 16-slot midplane backplane.
# ==============================================================================
# PROJECT AETHER CORE NETWORKING HARDWARE SETUP SCRIPT FOR HPE VCM CLI
# ==============================================================================

### 1. DEFINE PHYSICAL UPLINK NETWORKS (LACP TRUNKS TO TOP-OF-RACK SWITCHES)
add uplinkset Core-Uplink-A
add uplinkport Enclosure1:1:X1 uplinkset=Core-Uplink-A
add uplinkport Enclosure1:1:X2 uplinkset=Core-Uplink-A

add uplinkset Core-Uplink-B
add uplinkport Enclosure1:2:X1 uplinkset=Core-Uplink-B
add uplinkport Enclosure1:2:X2 uplinkset=Core-Uplink-B

### 2. CONSTRUCT AUTOMATED LOGICAL NETWORKS WITH SPECIFIC VLAN TAGS
add network Aether-Control-Bus vlantag=10 uplinkset=Core-Uplink-A
add network Aether-Storage-Bus vlantag=11 uplinkset=Core-Uplink-B
add network Host-OOB-Management vlantag=999 uplinkset=Core-Uplink-A

# Create a trunked network group to pass multi-tenant VLAN tags down to the hosts
add network-group Tenant-Workload-Trunk
add network Tenant-K8s-Data vlantag=20 networkgroup=Tenant-Workload-Trunk
add network Tenant-Jail-Data vlantag=30 networkgroup=Tenant-Workload-Trunk
add network Tenant-Isolated-VMs vlantag=40 networkgroup=Tenant-Workload-Trunk

### 3. DEFINE AND APPLY THE SERVER BLADE PROFILE TEMPLATE
# This template is cloned across all 16 slots to guarantee identical hardware execution paths
add profile-template Aether-Blade-Template

# Port 1.1: Assigned to the internal secure gRPC control bus
add connection Aether-Blade-Template network=Aether-Control-Bus speed=1000 type=FlexNIC mapping=1:1-a

# Port 2.1: Assigned to the high-throughput hyper-converged storage synchronization network
add connection Aether-Blade-Template network=Aether-Storage-Bus speed=1000 type=FlexNIC mapping=2:1-a

# Port 1.2: Enforces raw 802.1Q hardware trunking for tenant workload allocation
add connection Aether-Blade-Template networkgroup=Tenant-Workload-Trunk speed=2000 type=FlexNIC mapping=1:1-b

# Port 2.2: Dedicated out-of-band management interface for clean cluster infrastructure survival
add connection Aether-Blade-Template network=Host-OOB-Management speed=1000 type=FlexNIC mapping=2:1-b

### 4. BIND TEMPLATE INSTANCES DIRECTLY TO THE PHYSICAL CHASSIS SLOTS
# Iterates over the c7000 backplane layout to carve up the hardware profiles automatically
set server-profile-assignment Enclosure1:1 profile-template=Aether-Blade-Template
set server-profile-assignment Enclosure1:2 profile-template=Aether-Blade-Template
# [Slots 3 through 15 skipped for script brevity]
set server-profile-assignment Enclosure1:16 profile-template=Aether-Blade-Template
4. Host OS Virtual Switching Layouts

Once the Virtual Connect hardware enforces the FlexNIC allocations, the host operating systems see four completely distinct, isolated physical network cards. The configurations below illustrate how the host operating system bridges these connections into the virtual machines. [3]
4.1 FreeBSD Host Configuration (/etc/rc.conf for bhyve Nodes)

On a FreeBSD blade node, the user workload network interface maps to cxl2 (Chelsio 10Gb physical port driver identifier). We map this interface to our vm-bhyve virtual switch environment:
# Enable physical management and control bus interfaces
ifconfig_cxl0="inet 10.20.10.16 netmask 255.255.255.0" # Aether Control Bus (VLAN 10)
ifconfig_cxl1="inet 10.20.11.16 netmask 255.255.255.0" # ZFS Sync / Storage Bus (VLAN 11)
ifconfig_cxl3="inet 192.168.1.16 netmask 255.255.255.0" # Host OOB SSH (VLAN 999)

# Up the raw trunk interface for guest multi-tenant mapping
ifconfig_cxl2="up"
The administrator then tells vm-bhyve to create an abstract switch that automatically creates a bridge loop over the physical trunk:
vm switch create public-virtual-connect
vm switch add public-virtual-connect cxl2
4.2 Linux Host Configuration (/etc/netplan/01-netcfg.yaml for KVM/Firecracker Nodes)

On a Linux blade node, the equivalent interface maps to standard kernel device configurations. We create native Linux network bridges (br0) to pass virtual network capabilities down into our containers and Firecracker microVMs: [4]
network:
version: 2
renderer: networkd
ethernets:
enp2s0f0: # FlexNIC 1: Control Plane Bus
dhcp4: false
addresses: [10.20.10.12/24]
enp2s0f1: # FlexNIC 2: Storage Sync Bus
dhcp4: false
addresses: [10.20.11.12/24]
enp2s0f3: # FlexNIC 4: Host OOB Management
dhcp4: false
addresses: [192.168.1.12/24]
enp2s0f2: # FlexNIC 3: Raw Workload Trunk Interface
dhcp4: false
up: true

bridges:
br-workloads:
interfaces: [enp2s0f2]
dhcp4: false
parameters:
stp: false
forward-delay: 0
5. SME Operational Guardrails: Network Resilience and Failover Matrix

5.1 Active-Active Link Failover (SmartLink Enforcement)

To prevent network black-holing—where a blade server thinks its link is active because its internal midplane trace is up, but the rear upstream core networking switch has dropped frames—we enable HPE SmartLink inside our Virtual Connect Uplink Sets.
If both external upstream ports X1 and X2 on Virtual Connect Module 1 drop offline, the VC module instantly drops the internal midplane link to FlexNIC 1 across all 16 server blades.
The host operating system's internal bonding driver immediately registers this link-state drop and shifts all control and storage traffic to FlexNIC 2 on the redundant module in Interconnect Bay 2. This switchover happens in under 100 milliseconds, preventing cluster partition or accidental STONITH fencing activations.
5.2 Network Jitter Mitigation in Reverse Bidding

If network interfaces experience transient packet serialization delay (jitter) across the midplane, it can delay the arrival of gRPC bidding messages back to the central aggregator beyond our 250ms auction timeout window.
To safeguard against network latency variations, the Aether Control Bus (VLAN 10) is explicitly assigned an 802.1p Quality of Service (QoS) hardware mapping profile of 7 (highest priority Network Control traffic)within the Virtual Connect fabric configuration. This ensures that even if a tenant virtual machine on Blade 4 saturates its link with heavy network data traffic on VLAN 20, the cluster's gRPC coordination packets bypass all internal egress queues at line rate.
This networking specification provides the final blueprint for the physical and logical communication backplane of the project. We have covered the hardware configurations, host-level network maps, and out-of-band management designs for this open-source platform.
Let me know if you would like to begin detailing the next phase of the project, such as compiling the secure gRPC cryptographic token handshake mechanisms, or if you want to look at the GitOps manifest reconciliation logic for rolling updates and infrastructure adjustments.

[1] https://www.hpe.com
[2] https://community.hpe.com
[3] https://websistent.com
[4] https://docs-cybersec.thalesgroup.com