# Story: Kubernetes Management Plane Recovery & Auto-Discovery Sync

*   **Status:** Draft
*   **Story ID:** `STORY-06.5`
*   **Parent Epic:** [EPIC-06: Out-of-Band Fencing & HA](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_06_oob_fencing_ha/epic_06_oob_fencing_ha.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a central Aggregator operator
I want to perform an active auto-discovery sync with all worker blades upon startup
So that the cluster recovers from a management plane crash and re-aligns K8s custom resource statuses without VM downtime
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   **FR-6.5.1:** Upon operator startup/initialization, the Aggregator MUST broadcast a `ListVMs` gRPC request to all registered node daemon (`aetherd`) endpoints.
*   **FR-6.5.2:** The `aetherd` daemon MUST respond to `ListVMs` with a comprehensive list of active hypervisor processes (QEMU-KVM and Firecracker) running on the local host, including their UUIDs, MAC/IP layouts, and resource allocations.
*   **FR-6.5.3:** The Aggregator MUST reconcile the K8s `AetherVirtualDeployment` custom resources against the discovered VM list.
*   **FR-6.5.4:** If a `AetherVirtualDeployment` resource exists but the corresponding VM is not running on any node, the Aggregator MUST trigger a new reverse-bidding auction to reschedule and provision the VM.
*   **FR-6.5.5:** If a VM is running on a blade but has no matching `AetherVirtualDeployment` CRD in Kubernetes, the Aggregator MUST command `aetherd` to gracefully shutdown the guest and clean up its allocated network/storage block resources.
*   **FR-6.5.6:** If a VM is running and matches a `AetherVirtualDeployment` CRD, the Aggregator MUST update the CRD's `.status` subresource to state `Running`, setting the IP address and node placement.

### B. Non-Functional Requirements
*   **NFR-6.5.1:** The auto-discovery broadcast MUST run concurrently across all nodes with a hard network timeout of 5 seconds to prevent startup blockages.
*   **NFR-6.5.2:** The discovery sweep MUST NOT disrupt the execution of active VM guest processes or their host network interfaces.
*   **NFR-6.5.3:** Mutual TLS (mTLS) credentials and tokens MUST be verified on the discovery gRPC loop.

## 3. Technical Implementation Details

### A. Affected Codebases & Files
*   **Crate:** `crates/aether-aggregator`
*   **Target Files:**
    *   [crates/aether-aggregator/src/reconciler.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/reconciler.rs): Add operator initialization hook to trigger auto-discovery scan.
    *   [crates/aether-aggregator/src/ha/discovery.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/src/ha/discovery.rs): Implement discovery broadcast and status re-alignment loop.
*   **Crate:** `crates/aetherd`
*   **Target Files:**
    *   [crates/aetherd/src/grpc/server.rs](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/src/grpc/server.rs): Implement `ListVMs` endpoint, scanning processes and active ZVOL mappings.
*   **Proto:**
    *   [proto/aether.proto](file:///Users/casibbald/Workspace/remote/microscaler/Aether/proto/aether.proto): Define `ListVMsRequest` and `ListVMsResponse` message schemas.

### B. Detailed Design
*   **`ListVMs` RPC:**
    ```protobuf
    message ListVMsRequest {}

    message VMDetails {
        string uuid = 1;
        string name = 2;
        string state = 3; // e.g. RUNNING, BALLOONED
        string ip_address = 4;
        string mac_address = 5;
    }

    message ListVMsResponse {
        repeated VMDetails vms = 1;
    }
    ```
*   **Startup Sync Coordinator:**
    An initialization task spawned before starting the main operator reconciler loop. It holds a write lock on the `ClusterState` Node Registry, blocks incoming mutations until all nodes respond or timeout, and updates K8s resource statuses.

## 4. Acceptance Criteria

*   **Criteria 1:**
    *   **Given** A K8s management cluster recovering from an etcd database restore containing 3 running VM specs
    *   **When** The `aether-aggregator` operator boots up and connects to 2 active host blades
    *   **Then** It discovers the 3 running hypervisor VMs, matches them to the specs, and transitions their CRD statuses to `Running` without restarting the VMs.
*   **Criteria 2:**
    *   **Given** An orphaned VM running on Blade 2 which was deleted in K8s while the control plane was offline
    *   **When** The recovery sync completes
    *   **Then** The Aggregator instructs Blade 2 to clean up the orphaned VM, freeing up local memory and network allocations.

## 5. Verification Plan

### A. Automated Tests
*   **Unit Tests:** Run `cargo test -p aether-aggregator ha::discovery_tests`
*   **Integration Tests:** Run `cargo test --test k8s_plane_recovery_sync` to verify status reconciliation of matched, orphaned, and stale VM specs.

### B. Manual Verification
*   **Step 1:** Spin up mock node daemons hosting running VM instances.
*   **Step 2:** Stop the Aether Aggregator, change K8s CRD configurations (delete one, keep two), and start the Aggregator.
*   **Step 3:** Confirm the deleted VM is torn down on the node, and the remaining two have status `Running` reconstructed.
