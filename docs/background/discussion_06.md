Architectural Specification: Central gRPC Cluster Aggregator (The "vCenter Killer")

To maintain a zero-dependency, low-footprint control plane, the central aggregator operates as a stateless, event-driven loop. Instead of maintaining a heavy relational database, it keeps a minimal in-memory state table of active worker nodes and active workloads, delegating the physical execution to the autonomous blade daemons.
The design relies on two core architectural pipelines: The Bidding Convergence Pipeline and The Deadman Switch Failover Loop.
1. In-Memory Component State Engine

The central engine tracks the cluster using a highly synchronized, thread-safe state table protected by asynchronous read-write locks (Tokio::sync::RwLock).
┌──────────────────────────────┐
│     Aether Aggregator        │
│   In-Memory Cluster State    │
└──────────────┬───────────────┘
│
┌─────────────────────────┴─────────────────────────┐
▼                                                   ▼
┌─────────────────────────┐                         ┌─────────────────────────┐
│       NodeRegistry      │                         │    WorkloadPlacement    │
├─────────────────────────┤                         ├─────────────────────────┤
│ - NodeID (blade-01)     │                         │ - WorkloadUID (uuid)    │
│ - gRPC Endpoint URL     │                         │ - Target Node ID        │
│ - Last Heartbeat Epoch  │                         │ - Desired State Config  │
│ - Node Status Flags     │                         │ - Active Status         │
└─────────────────────────┘                         └─────────────────────────┘
2. Pipeline A: The Bidding Convergence Engine

When a new declarative workload is applied via CLI or GitOps sync, the aggregator does not evaluate where it should go. It starts a short-lived auction.
[Workload Intent Received]
│
▼
[1. Spawn Async Task Pool] ──► Dispatches gRPC `RequestReverseBid` to ALL Registered Nodes concurrently.
│
▼
[2. Open Timeout Window] ──► A strict 250ms asynchronous timer initializes.
│
├───► Node Responses collected into an execution Vector.
│
▼
[3. Evaluate & Select Winner]
│
├───► If No Valid Bids (All return -1): Workload enters `Pending_Unallocatable` state.
├───► If Valid Bids Exist: Sorts vector descending by `bid_score`.
│     └───► Tie-breaker: Lexicographical order of string NodeID.
▼
[4. Dispatch Execution Directive] ──► Sends gRPC `ExecuteProvisioning` message to the winning node.
│
▼
[5. Commit to State Engine] ──► Updates `WorkloadPlacement` map with the assigned NodeID.
3. Pipeline B: The Deadman Switch Failover Loop

This loop runs continuously on a background thread pool, acting as a lightweight, poor man's replacement for VMware vSphere High Availability (HA).
┌──────────────────────────────────────┐
│  Every 3 Seconds: Trigger Tick Loop   │
└──────────────────┬───────────────────┘
│
▼
┌──────────────────────────────────────┐
│ Iterate over `NodeRegistry` Entries  │
└──────────────────┬───────────────────┘
│
┌───────────────────────┴───────────────────────┐
▼                                               ▼
[ Node Status: Online ]                         [ Node Status: Suspect/Offline ]
│                                               │
Send gRPC Ping Request                           Calculate: Current Epoch - Last Heartbeat Epoch
│                                               │
┌────────┴────────┐                                      ▼
▼                 ▼                           Is Delta > 15 Seconds (5 missed pings)?
[Success]         [Timeout/Drop]                            │
│                 │                        ┌─────────────┴─────────────┐
No Action      Mark Status                    ▼                           ▼
as `Suspect`                [ No ]                      [ Yes ]
│                           │
Ignore Tick              Initiate Recovery
Sequence Cascades
The Recovery Sequence Cascade:

Mark the target node as Dead in the registry to prevent it from receiving future auction broadcasts.
Query the WorkloadPlacement table to isolate all Workload UIDs currently registered to that dead blade.
For each isolated workload, pull the original desired state configuration block from the metadata cache.
Re-inject those specs back into Pipeline A (The Bidding Convergence Engine).
The remaining 15 healthy blades run their autonomous resource calculations and bid to inherit the orphaned workloads. The cluster self-heals without centralized scheduling calculations.
4. Structural Rust System Blueprint

This architectural blueprint outlines how the types, structures, and asynchronous loops fit together inside the compiled Rust control plane before writing the concrete implementation logic.
// Unified Component Design for Aether Aggregator Core Control Loop

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

// Representation of a cluster worker node
struct WorkerNode {
node_id: String,
grpc_endpoint: String,
last_seen_heartbeat: Instant,
is_healthy: bool,
}

// Representation of a managed workload spec
struct VirtualWorkload {
uid: String,
name: String,
raw_yaml_definition: String,
assigned_node_id: Option<String>,
}

// The core structural state holding thread-safe memory tables
struct ClusterState {
nodes: HashMap<String, WorkerNode>,
workloads: HashMap<String, VirtualWorkload>,
}

// Main Aggregator Engine holding runtime references
pub struct vCenterKillerAggregator {
state: Arc<RwLock<ClusterState>>,
auction_timeout_ms: u64,
deadman_threshold_secs: u64,
}

impl vCenterKillerAggregator {
pub fn new() -> Self {
Self {
state: Arc::new(RwLock::new(ClusterState {
nodes: HashMap::new(),
workloads: HashMap::new(),
})),
auction_timeout_ms: 250,
deadman_threshold_secs: 15,
}
}

    /// PIELINE A: Spawns the concurrent gRPC auction loop across all blades
    pub async fn converge_and_auction_workload(&self, workload_uid: String) {
        // 1. Read healthy nodes from state table
        // 2. Map async Tokio tasks to issue `RequestReverseBid` gRPC requests to all endpoints concurrently
        // 3. Wrap execution in a `tokio::time::timeout` bounded by self.auction_timeout_ms
        // 4. Collect bid results, filter out rejections (-1), sort by score
        // 5. Select winner and dispatch gRPC `ExecuteProvisioning` to target node
        // 6. Update local state metadata mapping table
    }

    /// PIPELINE B: The background heartbeat tracking loop (vSphere HA Equivalent)
    pub async fn start_deadman_heartbeat_loop(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(3));
        
        loop {
            interval.tick().await;
            // 1. Acquire write lock on state table
            // 2. Iterate through nodes, calculate: Instant::now() - node.last_seen_heartbeat
            // 3. If delta exceeds self.deadman_threshold_secs:
            //    a. Set node.is_healthy = false
            //    b. Scan workloads table to isolate affected entities
            //    c. For each affected workload: spawn self.converge_and_auction_workload(uid)
        }
    }
}
5. Architectural Trade-offs for SME Deployments

The Shared-Storage vs. Shared-Nothing Dilemma: Because this replacement is simple, if a blade dies, the data stored on its local disks is inaccessible unless you use hyper-converged storage. For a true v1alpha HA failure recovery to work, the VMs must have their base operating systems stored on a small, shared NFS/iSCSI target pool mapping across the c7000 backplane fabrics, or run completely stateless immutable cloud-init instances where data persists externally in databases.
Split-Brain Prevention: If a blade doesn't reply to gRPC because of network congestion but is still running the VM, starting that same VM on another blade creates an IP conflict. To mitigate this in v1alpha without complex quorum software, the central aggregator uses the HPE chassis Onboard Administrator (OA) API or IPMI/iLO commands to hard power off a failed blade slot before re-auctioning its workloads (STONITH: "Shoot The Other Node In The Head").
If you would like to advance this specification, let me know if we should define the spec interfaces for the STONITH/iLO power fencing module, or design how the GitOps file synchronization watcher hooks directly into this auction engine.
