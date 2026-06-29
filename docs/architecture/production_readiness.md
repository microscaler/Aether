# Project Aether: Production Readiness & Operations Architecture

This document defines the operational, safety, and cloud-native integration architecture required to transition Project Aether from an initial v1 design into a hardened, production-ready platform. 

It addresses critical gaps in **Resource Overcommit Policies**, **Disaster Recovery (ZFS replication)**, **Kubernetes CRD Reconciler loops (Finalizers)**, **Secure Secrets Management**, and **Image Caching**.

---

## 1. Resource Scheduling & Overcommit Policies (Virtualization Plane)

Running dense blade chassis configurations requires strict resource overcommit models to maximize capacity while preventing host Out-Of-Memory (OOM) panics.

```
┌────────────────────────────────────────────────────────┐
│                   Bare-Metal Host RAM                  │
├──────────────────────────┬─────────────────────────────┤
│   Reserved (15% Host)    │    Overcommitted VM Pool    │
│  - Linux Kernel / ZFS    │  - VM 101 RAM  (Active)     │
│  - aetherd / K8s agent   │  - VM 102 RAM  (Ballooned)  │
│  - QEMU/KVM overhead     │  - VM 103 RAM  (Ballooned)  │
└──────────────────────────┴─────────────────────────────┘
```

### A. Memory Overcommit & Dynamic Ballooning
*   **Overcommit Ceiling:** Aether permits a maximum memory overcommit ratio of **1.5x** the physical RAM (excluding ZFS ARC limits).
*   **VirtIO Balloon Driver:** All QEMU guest definitions must include a `virtio-balloon` device:
    *   **Telemetry Loop:** `aetherd` monitors host memory pressure.
    *   **Throttling:** If host free memory drops below a safety threshold (e.g., <10% physical RAM), `aetherd` issues QMP `balloon` commands to idle guest VMs, reclaiming unused guest memory to feed active VMs.
*   **ZFS ARC Tuning:** The ZFS Adaptive Replacement Cache (ARC) maximum limit is capped at **15% of physical RAM** on storage blades to prevent storage caching from starving VM execution memory.

### B. vCPU Overcommit & Throttling
*   **Allocation Ratio:** Aether permits up to **3:1 vCPU-to-Physical-Core overcommit** on Compute blades (Slots 1-8).
*   **CPU Pinning (Core Isolation):** Critical VMs (like database engines) can specify CPU pinning in the CRD. `aetherd` maps pinned vCPUs directly to dedicated CPU threads using Linux `taskset` or systemd cgroup core isolation, protecting them from noisy neighbor contention.

---

## 2. Disaster Recovery & ZFS Replication (Datacentre Operations)

Because Aether runs a "shared-nothing" storage architecture on local ZFS volumes (ZVOLs) for slots 9–16, a complete blade crash isolates that node's data. To achieve low Recovery Point Objectives (RPO):

```
 Source Blade (Slot 9)                               Target Blade (Slot 10)
┌──────────────────────┐                           ┌──────────────────────┐
│  ZVOL (Active Writes)│                           │  ZVOL (Backup)       │
│  [vm-101-disk-0]     │                           │  [vm-101-disk-0]     │
└──────────┬───────────┘                           └──────────▲───────────┘
           │ (Snapshot: delta-5min)                           │
           │                                                  │
           └──── [ zrepl / ZFS Send ] ──► (VLAN 10 Control) ──┘
```

*   **Asynchronous ZFS Replication:** Aether integrates an asynchronous block replication daemon (e.g., `zrepl` or custom shell automation) operating over the private control plane network (VLAN 10).
*   **Replication Loop:**
    1.  A cron task triggers a ZVOL snapshot every 5 minutes on the primary host (`zroot/vms/vm-101@sync-1400`).
    2.  The replication engine performs an incremental `zfs send` to a designated backup storage blade in the chassis.
    3.  The backup blade receives the stream via `zfs recv` and updates its local disk block map.
*   **Failover Execution:** If Blade 9 suffers a catastrophic hardware failure and is fenced via `aether-fence`, the central Aggregator initiates a failover auction. The auction payload directs the winning node to mount the latest replicated ZVOL snapshot copy from the backup host, resuming operations with a maximum RPO data loss of 5 minutes.

---

## 3. Kubernetes Reconciler Flow & CRD Finalizers (Kubernetes Plane)

To ensure that the physical blade states remain in sync with the declarative Kubernetes API state, the Aether Aggregator operator implements strict **Finalizer lifecycle loops**:

```
[ Client deletes AetherVirtualDeployment CRD ]
                    │
                    ▼
[ Operator intercepts delete, checks Finalizer ]
                    │
                    ▼
[ Operator issues gRPC Teardown to aetherd ]
                    │
                    ├────────────────────────┐ (If teardown fails/times out)
                    ▼ (Teardown Success)     ▼
[ Host destroys VM & ZVOL ]            [ Operator calls aether-fence ]
                    │                  [ Force shut down host via iLO ]
                    ▼                  [ Block ZVOL releases ]
[ Operator removes Finalizer ]               │
                    │                        ▼
                    └──────────────► [ Delete CRD from K8s API ]
```

*   **Finalizer Lock (`finalizers.compute.aether.infra`):** The operator attaches a finalizer to the `AetherVirtualDeployment` CRD immediately upon creation.
*   **Safe Teardown Pipeline:**
    1.  When a user deletes a VM resource, the Kubernetes API server marks it for deletion but blocks removal due to the active finalizer.
    2.  The Aether Reconciler intercepts the deletion event, identifies the active host node from the `WorkloadPlacement` map, and dispatches a gRPC teardown command.
    3.  `aetherd` halts the QEMU/Firecracker process, detaches the VLAN interface, and destroys the ZVOL blocks (or moves them to the trash bin).
    4.  Only after `aetherd` reports successful cleanup does the operator remove the finalizer, allowing the CRD to be deleted.
*   **Failsafe Fencing:** If the target worker node is unreachable or fails the teardown command, the reconciler triggers `aether-fence` to forcefully power down the slot before removing the finalizer, protecting storage fabrics from duplicate block mounts.

---

## 4. Secure Secrets Management (Security Plane)

Plaintext injection of sensitive data (passwords, private SSH keys, TLS certificates) into Git repositories violates enterprise GitOps security standards.

*   **Sealed Secrets / External Secrets:** Credentials are encrypted in the Git repository using tools like Bitnami Sealed Secrets or Mozilla SOPS, decrypting only inside the Kubernetes utility cluster.
*   **Cloud-Init Dynamic Injection:**
    1.  The Aether Aggregator reads the decrypted Kubernetes `Secret` containing guest root credentials.
    2.  The secret value is passed to the winning `aetherd` node daemon over the mTLS gRPC channel as part of the execution directive.
    3.  `aetherd` compiles these values on the fly into the temporary NoCloud Cloud-Init `user-data` payload, compiling it to `seed.iso` directly in memory (using `tmpfs` RAM drives) to prevent secrets from touching host disks.

---

## 5. Container-Image Caching & Registry Proxies (PaaS Plane)

To enable sub-100ms startup times for Firecracker microVMs, Aether cannot afford to pull OCI container images from remote external registries over WAN links for every auction.

*   **Local Enclosure Registry Mirror:** A dedicated local container registry mirror runs inside the Kubernetes utility cluster on the management blades.
*   **`aetherd` Registry Cache:**
    *   Each Compute blade runs a local `containerd` image cache.
    *   When an image version is updated, the Aggregator broadcasts a pre-pull command to the node daemons.
    *   Daemons fetch the OCI image from the local midplane registry mirror over the 10Gb Virtual Connect midplane, ensuring that the image blocks are hot-cached on local disks before the VM auction is triggered.
