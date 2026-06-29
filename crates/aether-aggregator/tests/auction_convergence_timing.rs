// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use aether_aggregator::registry::NodeRegistry;
use aether_aggregator::scheduler::Scheduler;
use aether_auth::mtls::{create_client_tls_config, create_server_tls_config, test_pki};
use aether_auth::proto::aether_node_server::{AetherNode, AetherNodeServer};
use aether_auth::proto::{
    BidRequest, BidResponse, ExecuteVmRequest, ExecuteVmResponse, ListVMsRequest, ListVMsResponse,
    TeardownVmRequest, TeardownVmResponse,
};

struct MockNode {
    node_id: String,
    score: i32,
    delay: Duration,
}

#[tonic::async_trait]
impl AetherNode for MockNode {
    async fn request_reverse_bid(
        &self,
        _request: Request<BidRequest>,
    ) -> Result<Response<BidResponse>, Status> {
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        Ok(Response::new(BidResponse {
            node_id: self.node_id.clone(),
            score: self.score,
        }))
    }

    async fn execute_vm(
        &self,
        _request: Request<ExecuteVmRequest>,
    ) -> Result<Response<ExecuteVmResponse>, Status> {
        Ok(Response::new(ExecuteVmResponse {
            success: true,
            ip_address: "".to_string(),
            mac_address: "".to_string(),
            error_message: "".to_string(),
        }))
    }

    async fn teardown_vm(
        &self,
        _request: Request<TeardownVmRequest>,
    ) -> Result<Response<TeardownVmResponse>, Status> {
        Ok(Response::new(TeardownVmResponse {
            success: true,
            error_message: "".to_string(),
        }))
    }

    async fn list_v_ms(
        &self,
        _request: Request<ListVMsRequest>,
    ) -> Result<Response<ListVMsResponse>, Status> {
        Ok(Response::new(ListVMsResponse { vms: vec![] }))
    }
}

async fn start_mock_node(
    node_id: &str,
    score: i32,
    delay: Duration,
    addr: SocketAddr,
    creds: &test_pki::GeneratedCreds,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let server_tls =
        create_server_tls_config(&creds.ca_cert, &creds.server_cert, &creds.server_key);
    let service = MockNode {
        node_id: node_id.to_string(),
        score,
        delay,
    };

    let handle = tokio::spawn(async move {
        if let Ok(mut builder) = Server::builder().tls_config(server_tls) {
            let _ = builder
                .add_service(AetherNodeServer::new(service))
                .serve(addr)
                .await;
        }
    });

    // Short sleep to ensure server is listening
    tokio::time::sleep(Duration::from_millis(50)).await;
    Ok(handle)
}

#[tokio::test]
async fn test_auction_convergence_timing() -> Result<(), Box<dyn std::error::Error>> {
    let creds = test_pki::generate_test_creds()?;

    // Define ports
    let addr1: SocketAddr = "127.0.0.1:50091".parse()?;
    let addr2: SocketAddr = "127.0.0.1:50092".parse()?;
    let addr3: SocketAddr = "127.0.0.1:50093".parse()?;

    // Start 3 mock nodes:
    // Node 1: responds in 50ms (on time)
    // Node 2: responds in 120ms (on time)
    // Node 3: responds in 350ms (late, exceeds 250ms timeout)
    let h1 = start_mock_node("node-1", 900, Duration::from_millis(50), addr1, &creds).await?;
    let h2 = start_mock_node("node-2", 800, Duration::from_millis(120), addr2, &creds).await?;
    let h3 = start_mock_node("node-3", 700, Duration::from_millis(350), addr3, &creds).await?;

    let registry = Arc::new(RwLock::new(NodeRegistry::new()));
    {
        let mut reg = registry.write().await;
        // Register all three
        reg.register(
            "node-1".to_string(),
            format!("https://{}", addr1),
            "COMPUTE".to_string(),
            "token1".to_string(),
        );
        reg.register(
            "node-2".to_string(),
            format!("https://{}", addr2),
            "COMPUTE".to_string(),
            "token2".to_string(),
        );
        reg.register(
            "node-3".to_string(),
            format!("https://{}", addr3),
            "COMPUTE".to_string(),
            "token3".to_string(),
        );
    }

    let client_tls = create_client_tls_config(
        &creds.ca_cert,
        &creds.client_cert,
        &creds.client_key,
        "localhost",
    );

    let scheduler = Scheduler::new(registry, client_tls);

    // Run the bid broadcast
    let start_time = std::time::Instant::now();
    let bids = scheduler
        .broadcast_bid(4, 4096, 8192, "workload-uuid-abc".to_string())
        .await;
    let duration = start_time.elapsed();

    // Verify results
    println!("Auction finished in {:?}", duration);

    // Node 3 is late (>250ms), so it should not be in the results
    assert_eq!(bids.len(), 2);

    let mut node_ids: Vec<String> = bids.iter().map(|b| b.node_id.clone()).collect();
    node_ids.sort();
    assert_eq!(node_ids, vec!["node-1".to_string(), "node-2".to_string()]);

    let mut scores: Vec<i32> = bids.iter().map(|b| b.score).collect();
    scores.sort();
    assert_eq!(scores, vec![800, 900]);

    // Clean up
    h1.abort();
    h2.abort();
    h3.abort();

    Ok(())
}
