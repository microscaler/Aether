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
