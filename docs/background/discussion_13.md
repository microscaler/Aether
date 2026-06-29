Integrating FluxCD in a management Kubernetes cluster alongside a custom Rust-based Aether Aggregator Backendbridges mature GitOps ecosystems with bare-metal blade infrastructure.
FluxCD’s role is to watch your Git repositories, validate your custom manifests, and write them directly into the Kubernetes API as Custom Resources (CRDs). The custom Rust Aether Aggregator Backend acts as a specialized Kubernetes Controller / Operator running inside that same K8S cluster. It watches for changes to those custom CRDs and translates them into the gRPC reverse-bidding and orchestration routines executed by the bare-metal daemons (aetherd) running natively on the FreeBSD and Linux blades.
[ Git Repository ] ──(Commit/Push)──► [ FluxCD Source/Kustomize Controllers ]
│
▼ (Applies to K8S API)
[ K8S Custom Resources (CRDs) ]
│
▼ (Watches CRD Events)
[ Aether Central Aggregator ]
(Running as Pod inside K8S)
│
┌─────────────────────────────┼─────────────────────────────┐
│ gRPC (VLAN 10 Trunk)        │ gRPC (VLAN 10 Trunk)        │ gRPC (VLAN 10 Trunk)
▼                             ▼                             ▼
[ Blade Node 01 ]             [ Blade Node 02 ]             [ Blade Node 16 ]
(Linux Bare-Metal)            (Linux Bare-Metal)            (FreeBSD Bare-Metal)
1. The Declarative Rollout Schema Contract

To execute rolling updates (e.g., rolling out a new golden base OS image or modifying CPU/RAM footprints), we must extend our declarative schema with an explicit updateStrategy stanza. This configuration mirrors Kubernetes deployment semantics but targets your physical blade slots.
apiVersion: compute.aether.infra/v1alpha1
kind: AetherVirtualDeployment
metadata:
name: edge-compute-cluster
namespace: tenant-prod
spec:
replicas: 6                     # Spread across our 16-blade pool
updateStrategy:
type: RollingUpdate
rollingUpdate:
maxUnavailable: 1           # Enforces that only 1 VM drops during image progression
maxSurge: 1                 # Allows 1 extra "surged" VM to provision before tearing down old ones
template:
spec:
runtimeRequirement: bhyve
compute:
vcpus: 4
memoryBytes: 17179869184
storage:
storageClassName: gold-zfs-nvme
baseImage: ubuntu-24.04-v1    # Changing this to v2 triggers the rolling reconcile loop
2. Central Aggregator Architectural Reconciliation Loop

Running inside the K8S cluster, the Aether Aggregator leverages the kube-rs crate to track custom resource lifecycle events. When FluxCD pushes a change to the K8S API, the Aggregator processes a multi-stage Rolling State Machine.
[FluxCD Applies Update to K8S API] ──► [Aggregator Intercepts Event]
│
▼
[Is Template Generation Changed?]
│
┌──────────────────────────────┴──────────────────────────────┐
▼ (No)                                                        ▼ (Yes)
[Maintain Current State]                             [Calculate Discrepancy Matrix]
│
▼
[Create "Surge" Workload Manifest]
│
▼
[Trigger Pipeline A: Reverse-Bid Auction]
│
▼
[New VM Achieves Stable Status]
│
▼
[Issue gRPC Destroy to 1 Old Generation VM]
│
▼
[Repeat Loop Until Replicas == Target]
The State Machine Rules:

Identify the Target Inventory: The Aggregator checks its in-memory map to isolate all running VM instances derived from the edge-compute-cluster manifest definition.
Calculate the Surge Ceiling: Based on maxSurge: 1, the Aggregator generates a transient workload specification payload with a appended string descriptor (e.g., edge-compute-cluster-surge-v2).
Execute the Auction: The Aggregator broadcasts this surged payload to the gRPC control plane. The bare-metal node daemons perform their reverse-bidding calculations, and a host is selected to spawn the new generation instance.
Health Validation: The Aggregator waits for the winning blade to return a PROVISION_SUCCESS gRPC frame and confirms a stable MEMBER_HEARTBEAT from the new instance for at least 30 seconds.
Scale Down Step: Once the surge is healthy, the Aggregator targets exactly one instance running the legacy v1configuration, issues an asynchronous gRPC termination directive to its hosting blade, and removes it from the inventory table.
Cycle Execution: The Aggregator repeats this cycle sequentially, keeping the infrastructure stable and within bounds until all 6 running instances match the v2 target definition.
3. Rust Control Plane Rolling Blueprint (reconciler.rs)

This Rust structure maps out the asynchronous loop running inside your Kubernetes-hosted Aggregator container pod, bridging K8S custom watch APIs directly to bare-metal gRPC networks.
// Unified Rust Structural Module Map: aether-k8s-reconciler

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStrategyType {
RollingUpdate,
Recreate,
}

#[derive(Clone, Debug)]
pub struct RollingUpdateConfig {
pub max_unavailable: u32,
pub max_surge: u32,
}

#[derive(Clone, Debug)]
pub struct DeploymentSpec {
pub replicas: u32,
pub strategy: UpdateStrategyType,
pub rolling_config: RollingUpdateConfig,
pub base_image_version: String,
pub raw_template: String,
}

pub struct ActiveInstanceMetadata {
pub instance_id: String,
pub hosting_node_id: String,
pub current_image_version: String,
pub is_healthy: bool,
}

pub struct ReconcilerContext {
// Client connection mappings to bare-metal blades over the c7000 midplane VLAN
pub grpc_client_pool: Arc<RwLock<HashMap<String, String>>>,
}

pub struct AetherRollingEngine;

impl AetherRollingEngine {
/// Primary reconciliation loop invoked when FluxCD pushes a spec change to the K8S API
pub async fn reconcile_deployment(
ctx: Arc<ReconcilerContext>,
desired_spec: DeploymentSpec,
mut current_active_instances: Vec<ActiveInstanceMetadata>,
) -> Result<(), String> {

        println!("Initiating rolling infrastructure reconciliation loop. Target Image: {}", desired_spec.base_image_version);

        // Core processing loop while the infrastructure states are mismatched
        while Self::count_matching_instances(&current_active_instances, &desired_spec.base_image_version) < desired_spec.replicas {
            
            let active_matching = Self::count_matching_instances(&current_active_instances, &desired_spec.base_image_version);
            let total_active = current_active_instances.len() as u32;
            let legacy_instances: Vec<&ActiveInstanceMetadata> = current_active_instances.iter()
                .filter(|i| i.current_image_version != desired_spec.base_image_version)
                .collect();

            // Guardrail Clause 1: Can we surge a new machine forward?
            if total_active < desired_spec.replicas + desired_spec.rolling_config.max_surge {
                println!("Executing Surge Injection: Broadcasting gRPC Reverse-Bid Auction for New Generation.");
                
                // Execution Path:
                // 1. Compile localized Bidding spec matching desired_spec.raw_template
                // 2. Dispatch down to Pipeline A (Auction Loop Core)
                // 3. Await winning response and successful node registration callback
                
                // Simulating safe runtime creation tracking:
                let mock_new_instance = ActiveInstanceMetadata {
                    instance_id: format!("vm-surged-{}", active_matching + 1),
                    hosting_node_id: "blade-02-linux".to_string(),
                    current_image_version: desired_spec.base_image_version.clone(),
                    is_healthy: true,
                };
                
                // Stabilize and let Cloud-Init boot loops execute
                sleep(Duration::from_secs(10)).await;
                current_active_instances.push(mock_new_instance);
                continue;
            }

            // Guardrail Clause 2: If surge limits are hit, we must prune 1 legacy instance to clear paths
            if !legacy_instances.is_empty() {
                let target_to_prune = legacy_instances.first().unwrap();
                println!("Pruning Legacy Infrastructure Target: Sending gRPC Destroy to Node {}", target_to_prune.hosting_node_id);

                // Execution Path:
                // 1. Locate gRPC channel client inside ctx.grpc_client_pool for target_to_prune.hosting_node_id
                // 2. Transmit targeted termination command frame across midplane VLAN
                // 3. Await verified return confirmation, update active state array structures
                
                let id_to_remove = target_to_prune.instance_id.clone();
                current_active_instances.retain(|i| i.instance_id != id_to_remove);
            }

            // Small delay window to balance CPU loads across the reconciler loops
            sleep(Duration::from_millis(500)).await;
        }

        println!("Rolling infrastructure adjustments successfully reconciled. Quorum cluster matches intended GitOps definitions.");
        Ok(())
    }

    fn count_matching_instances(instances: &[ActiveInstanceMetadata], version: &str) -> u32 {
        instances.iter().filter(|i| i.current_image_version == version && i.is_healthy).count() as u32
    }
}
4. SME Operational Benefits: The K8S Cloud Abstraction

By shifting the control plane engine to run as an Operator inside a standard Kubernetes cluster, small businesses get significant architectural advantages without buying heavy enterprise licenses:
Native Custom Resources (CRDs): Your system administrators can manage bare-metal infrastructure alongside standard application containers using familiar tools:
kubectl get aethervirtualdeployments -n tenant-prod
GitOps Dependency Integrity: FluxCD enforces drift correction natively. If an operator walks onto the data center floor and uses the physical blade CLI to manually destroy a virtual machine or change an IP setting on blade-16, the Aggregator running inside Kubernetes will immediately detect the state drift via its polling mechanisms. FluxCD will force-apply the Git manifest again, triggering an automated reverse-bid to heal the configuration and restore the cluster state.
Low Management Footprint: The entire management cluster can run inside a highly resilient, three-node lightweight Kubernetes cluster (like k3s) deployed on a couple of low-power mini PCs or a pair of dedicated utility blades inside the chassis. This setups a completely self-contained cloud environment inside the c7000 frame, completely isolated from vendor licensing issues.
This integration provides a clean, type-safe blueprint to merge cloud-native GitOps controllers with decentralized, bare-metal hardware pools.
Would you like to explore the gRPC state telemetry reporting contracts that the worker daemons use to update the Kubernetes CRD status blocks, or detail the network routing architecture between the management K8S cluster and the Virtual Connect backplane?
