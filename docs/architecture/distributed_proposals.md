# Flintlock Distributed Architecture: Functional Contracts

This document extracts and defines the high-level functional contracts, API requirements, and state-machine transitions proposed in the **Flintlock Distributed Architecture Enhancement Proposals** (Proposals 01 through 15). 

These specifications detail how Flintlock was designed to transition from isolated, single-host hypervisors to a coordinated, multi-node cluster, providing a direct design template for **Project Aether's** development.

---

## 1. Consensus & Log Replication Contracts (Proposals 01, 09, 10)

To transition isolated host nodes into a unified cluster, the platform utilizes a **Replicated State Machine (RSM)** pattern backed by Raft:

### A. State Replication Contract
*   **Deterministic State Engine:** Every VM lifecycle event (creation, update, deletion) must be logged as a deterministic transaction entry.
*   **Log Commits:** Transactions are only applied to the cluster-wide state machine after achieving majority consensus (quorum) across the active nodes.
*   **Rejoin Synchronization:** When an offline node rejoins the cluster, it initiates a secure handshake with the active leader:
    1.  The joining node requests log reconciliation.
    2.  The leader transfers the latest compacted state snapshot.
    3.  The joining node replays remaining delta log entries to achieve state alignment.
    4.  The joining node reports consistency alignment before receiving new workloads.

### B. Log Compaction & Snapshotting
*   **Threshold-Based Triggers:** To prevent disk and memory exhaustion, logs are compacted automatically once they cross a configured transaction count or file size.
*   **State Compaction:** The active log is truncated, and the current cluster state (VM list, active hosts, IP mapping) is saved as a static, version-controlled state snapshot.

---

## 2. Distributed Workload Scheduling & Auction Contracts (Proposals 02, 08)

Dynamic scheduling replaces manual host assignment with an automated, pull-based bidding system managed by the cluster leader:

### A. Telemetry & Capacity Evaluation
*   **Host Heartbeat Metrics:** Nodes periodically report their local utilization state:
    *   `vCpuCongestion`: Active CPU load average.
    *   `MemoryAvailable`: Free memory in Megabytes.
    *   `VmActiveCount`: Count of running hypervisor instances.
    *   `StorageWriteWear`: S.M.A.R.T. health percentages for host SSDs.

### B. Bidding Contract
*   **Bid Invitation:** The leader broadcasts a scheduling request detailing the virtual specification (cores, memory, storage interfaces) to all active nodes.
*   **Dynamic Scoring:** Nodes parse the specification and compute a local bid score ($0 \leq \text{score} \leq 1000$):
    *   Nodes return `-1` (Reject) if allocation would violate host SLA thresholds or trigger OOM conditions.
    *   Healthy nodes return higher scores based on available memory channels, low thermal metrics, and low write wear.
*   **Winner Selection:** The leader selects the highest-scoring node within a strict **250ms convergence window**.

### C. Multi-Coordinator Scheduling (Scaling)
To prevent the single Raft leader from becoming a bottleneck during high-frequency API requests:
*   **Coordinator Pool:** The leader delegates scheduling tasks to a sub-pool of elected coordinator nodes.
*   **Local Cache Lookup:** Coordinators evaluate bids against a local, eventually-consistent cache of cluster resource telemetry.
*   **Conflict Resolution:** Concurrent scheduling collisions (e.g., two coordinators assigning workloads to the same host slot) are resolved by a deterministic index fallback protocol.

---

## 3. Resilience, Fencing & Partition Contracts (Proposals 04, 05, 07)

Maintaining cluster state consistency during network failures and hardware crashes relies on strict partition boundaries and watchdog controls:

### A. Network Partition Watchdog
*   **Follower Watchdog Timer:** Every worker node runs a background connectivity watchdog.
*   **Graceful Auto-Shutdown:** If a node loses connection to the cluster leader for $N$ consecutive seconds, it triggers local VM termination routines:
    1.  The local guest OS is sent a shutdown signal.
    2.  If the guest fails to halt within a grace period, the local hypervisor process is forcefully terminated.
    3.  This prevents "split-brain" states where a partitioned node continues writing to volumes that are being re-allocated elsewhere.

### B. Quorum & VM Resurrection
*   **Quorum Enforcement:** Only the partition containing the majority of active nodes (the quorum partition) can make cluster updates or elect leaders.
*   **Automated Resurrection:** Upon detecting a node drop-off via heartbeat timeouts:
    1.  The active leader marks the failed node as offline.
    2.  The leader isolates all VM IDs registered to that dead node.
    3.  The leader reads the persistent VM metadata specs and re-injects them into the scheduling queue to be spawned on healthy hosts.

---

## 4. Unified API Interface & Proxy Routing (Proposal 03)

Clients interact with a single, versioned endpoint rather than contacting individual hypervisor hosts:

*   **Global Versioning:** `/api/v1` remains backward compatible with single-host calls, while `/api/v2` endpoints handle cluster-wide operations.
*   **Location Metadata Registry:** The receiving node checks a lightweight location map (`VmID` $\rightarrow$ `HostIP`).
*   **Authoritative Proxying:** If the VM is running on another node:
    1.  The receiving node forwards the query to the hosting node's local daemon.
    2.  The host daemon queries the guest status and returns live telemetry.
    3.  The receiving node formats and forwards the response back to the client.

---

## 5. VM Migration & Operational Contracts (Proposals 06, 11, 12, 13, 14, 15)

Supporting continuous operations during host upgrades requires state checkpointing and standardized host setups:

### A. Graceful VM Migration
*   **State Checkpointing:** The source host halts VM execution and outputs a consistent memory and CPU register dump.
*   **Secure State Transfer:** The checkpoint file is streamed over a secure control channel to the target host.
*   **Restoration:** The target host initializes the hypervisor with the matching specification, mounts the cloned storage volume, and restores execution state from the transferred checkpoint file.

### B. Provisioning & Security
*   **Immutable PXE Boot:** Nodes boot over the network using an immutable OS image (Flatcar Linux), carrying the latest version-locked daemon. This eliminates host configuration drift.
*   **mTLS Encryption:** All inter-node communication (Raft sync, gRPC scheduling, migration streams) requires Mutual TLS (mTLS) certificate verification.
*   **Dynamic Configurations:** Configurations are managed centrally with version-controlled rollbacks and rolling application policies.
