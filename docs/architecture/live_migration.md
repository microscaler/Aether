# KVM Live Migration Design & API Specification

This document details how **Project Aether** implements VMware-like live virtual machine migration utilizing low-level **QEMU/KVM** mechanisms. It outlines the core protocols, shared vs. local storage workflows, and the JSON-based **QEMU Monitor Protocol (QMP)** API contract.

---

## 1. Core Migration Paradigms

Live migration moves a running QEMU/KVM virtual machine from a source bare-metal blade host to a target host with minimal guest downtime (typically <100ms) by utilizing two distinct phases:

### A. Pre-Copy Memory Migration (Iterative)
1.  The source VM remains running and active.
2.  QEMU begins copying memory pages (RAM) from the source host to the destination host over a dedicated TCP connection.
3.  During transmission, the guest continues writing to memory, marking modified pages as "dirty".
4.  QEMU tracks these dirty pages and re-transmits them iteratively in successive rounds.
5.  If a guest writes memory faster than the network can transmit (heavy write workload), Aether triggers **Auto-Converge** to dynamically throttle the guest vCPUs, allowing the migration to converge.

### B. Stop-and-Copy Phase
1.  Once the delta of remaining unsynchronized memory pages drops below a threshold, the source QEMU process halts guest execution.
2.  The final remaining dirty pages, CPU registers, and VirtIO device states are transmitted.
3.  The destination QEMU process receives the final state, activates its vCPUs, and resumes guest execution.
4.  The source QEMU process terminates, and block storage locks are transferred.

---

## 2. Storage Architectures: Shared vs. Local

Aether supports two migration configurations depending on the storage class definition of the VM:

```
[ Shared Storage (ZFS-over-IP / SAN) ]
Source Host                                                    Target Host
 ┌───────────┐  (Memory & Device State via TCP)  ┌───────────┐
 │ Source VM │ ────────────────────────────────> │ Target VM │
 └─────┬─────┘                                   └─────┬─────┘
       │                                               │
       └───────────► [ Shared Storage Volume ] ◄───────┘

───────────────────────────────────────────────────────────────────

[ Local Storage (ZFS on Linux ZVOLs) ]
Source Host                                                    Target Host
 ┌───────────┐  (QEMU drive-mirror via NBD)      ┌───────────┐
 │ Source VM │ ────────────────────────────────> │ Target VM │
 └─────┬─────┘                                   └─────┬─────┘
       │                                               │
       ▼ (Local Write)                                 ▼ (Mirrored Write)
  [ ZVOL A ]                                      [ ZVOL B ]
```

### A. Shared Storage Migration
*   **Requirements:** VM disks reside on a shared pool accessible by both hosts.
*   **Workflow:** The source host relinquishes the lock on the shared volume, memory is copied, and the target host acquires the lock before resuming execution.
*   **Overhead:** Minimal. Only CPU and memory state are transmitted over the network.

### B. Local Storage Migration (Block Migration)
*   **Requirements:** VM disks reside on local storage (ZFS ZVOLs or LVM) of individual blades.
*   **Workflow:** Utilizes QEMU's **Network Block Device (NBD)** protocol to mirror disk writes in real-time.
    1.  The target host starts a paused QEMU instance and opens an NBD server.
    2.  The source host attaches a `drive-mirror` block job, writing block sectors to the target NBD server.
    3.  Once the block job enters the `BLOCK_JOB_READY` state, QEMU begins the memory migration phase.
    4.  After memory migration completes, the mirror job is finalized, and local writes transfer entirely to the target disk.

---

## 3. Low-Level QMP API Contracts

The following sequence outlines the exact JSON-RPC payloads sent via the QEMU Monitor Protocol (QMP) Unix sockets by `aetherd` to coordinate block and memory migration.

### Step 1: Destination Host Preparation
The target node daemon `aetherd` initializes a paused QEMU process with identical hardware specs, adding `-incoming tcp:0.0.0.0:4444`. 
If performing **local storage migration**, it starts an NBD server and exports the target block device:

```json
// Start NBD Server on port 49153
{
  "execute": "nbd-server-start",
  "arguments": {
    "addr": {
      "type": "inet",
      "data": { "host": "0.0.0.0", "port": "49153" }
    }
  }
}

// Add the target ZVOL disk to the NBD server
{
  "execute": "nbd-server-add",
  "arguments": {
    "device": "drive-virtio-disk0",
    "writable": true
  }
}
```

### Step 2: Source Host Disk Mirroring (For Local Storage Only)
The source node daemon `aetherd` initiates mirroring of the local ZVOL to the target NBD server:

```json
{
  "execute": "drive-mirror",
  "arguments": {
    "device": "drive-virtio-disk0",
    "job-id": "mirror-job-0",
    "target": "nbd:target-host-ip:49153:exportname=drive-virtio-disk0",
    "sync": "full",
    "mode": "existing"
  }
}
```
*   `aetherd` polls the job status using `{ "execute": "query-block-jobs" }` or listens for the `BLOCK_JOB_READY` event.

### Step 3: Source Host Memory Migration
Once the storage is in sync, the source daemon configures migration safety capabilities and initiates the memory transfer:

```json
// Enforce safe handoffs before final switchover
{
  "execute": "migrate-set-capabilities",
  "arguments": {
    "capabilities": [
      { "capability": "pause-before-switchover", "state": true },
      { "capability": "auto-converge", "state": true }
    ]
  }
}

// Set maximum bandwidth limit (e.g., 10 Gbps)
{
  "execute": "migrate-set-parameters",
  "arguments": {
    "max-bandwidth": 1250000000
  }
}

// Start memory migration
{
  "execute": "migrate",
  "arguments": {
    "uri": "tcp:target-host-ip:4444"
  }
}
```

### Step 4: Monitor & Complete
*   The source daemon polls progress via `{ "execute": "query-migrate" }`.
*   Once QEMU pauses execution, the block job is completed:
    ```json
    {
      "execute": "block-job-complete",
      "arguments": { "device": "mirror-job-0" }
    }
    ```
*   The target host resumes VM execution, and the source host cleans up its local VM files.

---

## 4. Aether v1 Coordination Loop

The central **Aether Aggregator** acts as the high-level coordinator of this workflow, executing a two-phase commit over gRPC control channels on **VLAN 10**:

```
Central Aggregator                  Source aetherd                     Target aetherd
        │                                  │                                  │
        │─── 1. PrepareTarget(Spec) ───────┼─────────────────────────────────>│
        │<── 2. TargetReady(NBD Port) ─────┼──────────────────────────────────│
        │                                  │                                  │
        │─── 3. StartMigration(TargetIP) ─>│                                  │
        │                                  │── [QMP: drive-mirror]            │
        │                                  │── [QMP: migrate memory]          │
        │                                  │                                  │
        │<── 4. MigrationComplete ─────────│                                  │
        │                                                                     │
        │─── 5. CommitPlacement(TargetNode) ──────────────────────────────────┘
```
This loop ensures that the global K8s state registry is only updated with the new node location after both storage and memory handoffs are successfully validated.

---

## 5. Proxmox Source Code Reference & Alignment

To trace the engineering lineage of this protocol, Aether's implementation maps directly to concrete subroutines found in the official **Proxmox VE `qemu-server` repository** (`git.proxmox.com/?p=qemu-server.git`):

### A. Migration Orchestrator (`PVE/QemuMigrate.pm`)
The lifecycle flow of memory and storage migration is governed by the following Perl subroutines in `QemuMigrate.pm`:
*   **`phase1` (Preparation):** Checks destination storage accessibility and locks resources. Aether adapts this step within the Aggregator's pre-flight check loop.
*   **`phase2` (Execution & Mirroring):** Spawns the target paused QEMU process with the `-incoming` flag. If local storage is detected, it starts the target NBD server (via `nbd_start` in `QemuServer.pm`) and fires up QMP `drive-mirror` jobs on the source host.
*   **`phase2_late` (Sync & Memory Transfer):** Polls the mirror job status until the block devices report a `BLOCK_JOB_READY` state, then issues the QMP `migrate` command over TCP.
*   **`phase3` (Cleanup & Handoff):** Once memory migration reports completion, it issues the final QMP `block-job-complete` command on the source, unlocks the VM configuration, shuts down the source QEMU process, and releases disk reservations.

### B. QMP Client & Helpers (`PVE/QemuServer.pm`)
Low-level QMP JSON-RPC encapsulation is handled in `QemuServer.pm`:
*   **`vm_qmp_command`:** The primary wrapper that opens Unix sockets and transmits JSON-RPC requests directly to QEMU. Aether implements this natively in Rust inside `aetherd` using async socket streams.
*   **`nbd_start`:** The subroutine that sends QMP `nbd-server-start` and `nbd-server-add` commands to initialize target-side storage write targets. Aether maps this directly into `aetherd`'s incoming storage controller loop.
