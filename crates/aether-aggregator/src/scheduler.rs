// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::transport::{Channel, ClientTlsConfig};

use crate::registry::NodeRegistry;
use aether_auth::proto::aether_node_client::AetherNodeClient;
use aether_auth::proto::{BidRequest, BidResponse};

/// Scheduler handles broadcasting bidding requests to registered blades
/// and selecting nodes for workloads.
pub struct Scheduler {
    registry: Arc<RwLock<NodeRegistry>>,
    client_tls_config: ClientTlsConfig,
}

impl Scheduler {
    /// Creates a new instance of the Scheduler.
    pub fn new(registry: Arc<RwLock<NodeRegistry>>, client_tls_config: ClientTlsConfig) -> Self {
        Self {
            registry,
            client_tls_config,
        }
    }

    /// Broadcasts a bid request to all active nodes and collects their bids.
    /// Closes the convergence window strictly at 250ms, returning all bids received on time.
    pub async fn broadcast_bid(
        &self,
        cpu: i32,
        memory_bytes: i64,
        disk_bytes: i64,
        workload_uuid: String,
    ) -> Vec<BidResponse> {
        let active_nodes = {
            let reg = self.registry.read().await;
            reg.get_active_nodes()
        };

        let mut set = tokio::task::JoinSet::new();

        for node in active_nodes {
            let tls_config = self.client_tls_config.clone();
            let workload_uuid_clone = workload_uuid.clone();
            let grpc_endpoint = node.grpc_endpoint.clone();
            let node_id = node.node_id.clone();

            set.spawn(async move {
                let request_future = async {
                    let channel = Channel::from_shared(grpc_endpoint)
                        .map_err(|e| e.to_string())?
                        .tls_config(tls_config)
                        .map_err(|e| e.to_string())?
                        .connect_timeout(Duration::from_millis(250))
                        .timeout(Duration::from_millis(250));

                    let connected_channel = channel.connect().await.map_err(|e| e.to_string())?;
                    let mut client = AetherNodeClient::new(connected_channel);

                    let request = tonic::Request::new(BidRequest {
                        workload_uuid: workload_uuid_clone,
                        cpu_request: cpu,
                        memory_request_bytes: memory_bytes,
                        disk_request_bytes: disk_bytes,
                    });

                    let response = client
                        .request_reverse_bid(request)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok::<BidResponse, String>(response.into_inner())
                };

                // Enforce a strict 250ms deadline on the request
                match tokio::time::timeout(Duration::from_millis(250), request_future).await {
                    Ok(Ok(bid)) => Some(bid),
                    Ok(Err(e)) => {
                        log::warn!("Bidding error from node {}: {}", node_id, e);
                        None
                    }
                    Err(_) => {
                        log::warn!("Bidding request timed out for node {}", node_id);
                        None
                    }
                }
            });
        }

        let mut bids = Vec::new();
        while let Some(res) = set.join_next().await {
            if let Ok(Some(bid)) = res {
                bids.push(bid);
            }
        }

        bids
    }

    /// Evaluates the collected bids and selects the optimal node.
    /// Filters out bids with score -1, finds the highest score, and resolves any ties deterministically.
    pub fn select_winner(
        &self,
        bids: &[BidResponse],
        ssd_wears: &std::collections::HashMap<String, f64>,
        chassis_active_vms: &std::collections::HashMap<u32, u32>,
    ) -> Result<Option<BidResponse>, String> {
        let valid_bids: Vec<&BidResponse> = bids.iter().filter(|b| b.score > 0).collect();

        if valid_bids.is_empty() {
            return Ok(None);
        }

        // Find the maximum score
        let mut max_score = 0;
        for bid in &valid_bids {
            if bid.score > max_score {
                max_score = bid.score;
            }
        }

        // Collect all bids that have the maximum score
        let tied_bids: Vec<&BidResponse> = valid_bids
            .into_iter()
            .filter(|b| b.score == max_score)
            .collect();

        if tied_bids.is_empty() {
            return Ok(None);
        }

        if tied_bids.len() == 1 {
            let winning_bid = tied_bids.first().ok_or("No bid at index 0")?;
            return Ok(Some((*winning_bid).clone()));
        }

        // We have a tie! Use the tie-breaker module to resolve it.
        let mut candidates = Vec::with_capacity(tied_bids.len());
        for bid in &tied_bids {
            let ssd_wear = *ssd_wears.get(&bid.node_id).unwrap_or(&0.0);
            candidates.push(crate::tie_breaker::TieBreakerCandidate {
                node_id: bid.node_id.clone(),
                ssd_wear,
            });
        }

        let winning_candidate = crate::tie_breaker::resolve_tie(&candidates, chassis_active_vms)?;

        // Find the original bid matching the winning candidate's node_id
        for bid in tied_bids {
            if bid.node_id == winning_candidate.node_id {
                return Ok(Some(bid.clone()));
            }
        }

        Err("Winner node ID did not match any of the tied bids".to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use aether_auth::mtls::create_client_tls_config;
    use aether_auth::mtls::test_pki::generate_test_creds;

    #[tokio::test]
    async fn test_scheduler_empty_registry() {
        let registry = Arc::new(RwLock::new(NodeRegistry::new()));
        let creds = generate_test_creds().unwrap();
        let client_tls = create_client_tls_config(
            &creds.ca_cert,
            &creds.client_cert,
            &creds.client_key,
            "localhost",
        );

        let scheduler = Scheduler::new(registry, client_tls);
        let bids = scheduler
            .broadcast_bid(2, 1024, 2048, "uuid-123".to_string())
            .await;

        assert_eq!(bids.len(), 0);
    }
}
