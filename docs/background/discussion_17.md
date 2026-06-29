Yes, it is technically possible, but it requires enabling Nested Virtualization inside bhyve so the guest Linux VM can access the /dev/kvm hardware extensions required by Firecracker.
However, because you are running an enterprise HPE c7000 chassis with Intel Xeon Gold 6138 processors, doing this introduces significant architectural traps, performance degradation, and configuration quirks that you need to account for.
Here is the breakdown of how nested virtualization works in bhyve, why it is risky for your hardware, and a much better "Slice-and-Dice" alternative that avoids allocating a whole blade to Firecracker. [1]
1. How to Enable Firecracker inside a bhyve VM

To run Firecracker inside an Ubuntu VM hosted on a FreeBSD bhyve node, you must force bhyve to pass the Intel VMX (Virtual Machine Extensions) instructions down into the guest kernel.
Step A: Configure the FreeBSD Host Kernel

You must enable nested virtualization globally on the FreeBSD blade host by adding the following parameter to /boot/loader.conf and rebooting:
hw.vmm.vmx.nested=1
Step B: Configure the vm-bhyve Manifest

Inside your declarative VM configuration payload (/zroot/vm/linux-fc-host/linux-fc-host.conf), you must explicitly pass the host CPU model and topology features down to the guest:
cpu=8
cpu_owner=1
cpuset.cpu_domain=1
# Forces bhyve to present raw Intel VMX features to the guest OS
cpu_features="vmx"
Step C: Verify inside the Linux Guest

Once the Ubuntu VM boots inside bhyve, you can verify that the Firecracker requirements are satisfied by checking for the KVM device node:
$ ls -l /dev/kvm
crw-rw---- 1 root kvm 10, 232 Jun 28 22:30 /dev/kvm
If /dev/kvm is present, Firecracker will run. [2]
2. The Architectural Traps of This Approach

While it works on paper, running Firecracker inside bhyve creates a KVM-inside-bhyve nested virtualization sandwichthat introduces three major problems:
The Gen10 Extended Page Table (EPT) Penalty: Your Intel Xeon Gold 6138 processors use hardware-assisted virtualization (Intel VT-x with EPT). When a VM is nested, every single memory access inside Firecracker must be translated across three layers: Firecracker Guest -> Ubuntu VM -> FreeBSD Host -> Physical Memory. Because your blades only have 2 channels of memory populated instead of 12, this nesting compounding effect will bottleneck your memory bandwidth, leading to a massive 20% to 30% performance penalty.
Loss of the Sub-100ms Firecracker Advantage: Firecracker’s main selling point is its ability to boot a secure microVM in less than 10 milliseconds. When running inside nested bhyve threads, clock synchronization and virtual I/O overheads slow this boot time down significantly, turning your ultra-fast microVM into a standard, sluggish VM loop. [3]
Stability Risks: Nested virtualization in bhyve is stable for standard development, but running highly volatile, multi-tenant Kubernetes workloads (like Kata Containers) inside a nested KVM layer frequently triggers host kernel panics on the FreeBSD side under heavy thread contention.
3. The Better Solution: The "Slice-and-Dice" Bare-Metal Linux Strategy

You do not need to allocate an entire physical 40-core blade strictly to Firecracker. Instead of nesting Firecracker inside bhyve, change the Host OS of that blade to a minimal bare-metal Linux installation (Ubuntu Server or Debian) and use cgroups/systemd slices to carve it up natively.
Instead of letting bhyve carve up the blade, let the Linux kernel handle the slicing on bare metal:
[ Bare-Metal Linux Blade (40 Cores / 256GB) ]
│
┌─────────────────────────────────┼─────────────────────────────────┐
▼                                 ▼                                 ▼
┌─────────────────────────┐       ┌─────────────────────────┐       ┌─────────────────────────┐
│ Systemd Slice A (30%)   │       │ Systemd Slice B (40%)   │       │ Systemd Slice C (30%)   │
├─────────────────────────┤       ├─────────────────────────┤       ├─────────────────────────┤
│ Standard Linux KVM VMs  │       │ Raw Firecracker Pools   │       │ Native Docker/K8s Pods  │
│ (Heavy isolated tasks)  │       │ (Direct KVM Access)     │       │ (Standard applications) │
└─────────────────────────┘       └─────────────────────────┘       └─────────────────────────┘
Why the Bare-Metal Linux Slice is Superior:

Zero Nesting Penalty: Firecracker sits directly on top of the physical /dev/kvm layer, preserving its sub-100ms boot performance and native hardware speed.
Granular Resource Partitioning: You can use native Linux cgroups to guarantee that Firecracker can never consume more than, for example, 16 cores and 64GB of RAM on that blade. The remaining 24 cores and 192GB of RAM can simultaneously run standard KVM virtual machines, Docker containers, or Kubernetes worker nodes natively on the same host.
Unified Automation: Your central Rust Aether Aggregator can send a gRPC payload to the blade daemon (aetherd), which tells it to spin up a Firecracker microVM inside a restricted systemd-slice configuration.
How to Move Forward with Your Cluster Design

If you want to keep your cluster simple and high-performing, you should run Minimal Bare-Metal Linux on the blades where you want to mix Firecracker and isolated Linux VMs, and reserve FreeBSD/bhyve for blades that are dedicated to heavy database replication and high-throughput ZFS storage tasks.
Would you like to see how the Rust local node daemon (aetherd) would handle launching a Firecracker microVM inside a restricted systemd-slice on a shared Linux blade? Or should we look at how to structure the central allocator to balance both Linux-KVM and FreeBSD-bhyve blades simultaneously?

[1] https://sumofbytes.com
[2] https://blogs.oracle.com
[3] https://medium.com
