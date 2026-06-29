// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
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
}

#[tonic::async_trait]
impl AetherNode for MockNode {
    async fn request_reverse_bid(
        &self,
        _request: Request<BidRequest>,
    ) -> Result<Response<BidResponse>, Status> {
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
    addr: SocketAddr,
    creds: &test_pki::GeneratedCreds,
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let server_tls =
        create_server_tls_config(&creds.ca_cert, &creds.server_cert, &creds.server_key);
    let service = MockNode {
        node_id: node_id.to_string(),
        score,
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
async fn test_deterministic_scheduling_selection() -> Result<(), Box<dyn std::error::Error>> {
    let creds = test_pki::generate_test_creds()?;

    // Define ports
    let addr1: SocketAddr = "127.0.0.1:50191".parse()?;
    let addr2: SocketAddr = "127.0.0.1:50192".parse()?;
    let addr3: SocketAddr = "127.0.0.1:50193".parse()?;

    // Start 3 mock nodes, all returning an identical bid score of 850
    let h1 = start_mock_node("blade-01", 850, addr1, &creds).await?;
    let h2 = start_mock_node("blade-02", 850, addr2, &creds).await?;
    let h3 = start_mock_node("blade-03", 850, addr3, &creds).await?;

    let registry = Arc::new(RwLock::new(NodeRegistry::new()));
    {
        let mut reg = registry.write().await;
        reg.register(
            "blade-01".to_string(),
            format!("https://{}", addr1),
            "COMPUTE".to_string(),
            "token1".to_string(),
        );
        reg.register(
            "blade-02".to_string(),
            format!("https://{}", addr2),
            "COMPUTE".to_string(),
            "token2".to_string(),
        );
        reg.register(
            "blade-03".to_string(),
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

    // Broadcast the bids
    let bids = scheduler
        .broadcast_bid(4, 4096, 8192, "workload-uuid-tie".to_string())
        .await;

    assert_eq!(bids.len(), 3);

    // -------------------------------------------------------------
    // Scenario 1: Tie-breaker resolved by SSD Wear (densities are equal)
    // -------------------------------------------------------------
    // All nodes have adjacent density = 0
    let chassis_active_vms = HashMap::new();

    // SSD wears: blade-02 has the lowest wear (0.05)
    let mut ssd_wears = HashMap::new();
    ssd_wears.insert("blade-01".to_string(), 0.15);
    ssd_wears.insert("blade-02".to_string(), 0.05);
    ssd_wears.insert("blade-03".to_string(), 0.10);

    let winner = scheduler
        .select_winner(&bids, &ssd_wears, &chassis_active_vms)?
        .ok_or("Expected a winner")?;

    assert_eq!(winner.node_id, "blade-02");

    // -------------------------------------------------------------
    // Scenario 2: Tie-breaker resolved by Adjacent Slot Density
    // -------------------------------------------------------------
    // Let's populate slot 2 with 10 active VMs.
    // Slot 1 (blade-01) adjacent: slot 2 (10 VMs). Density = 10.
    // Slot 2 (blade-02) adjacent: slots 1, 3 (0 VMs). Density = 0.
    // Slot 3 (blade-03) adjacent: slots 2, 4 (10 VMs). Density = 10.
    let mut chassis_active_vms_s2 = HashMap::new();
    chassis_active_vms_s2.insert(2, 10);

    // Reconfigure SSD wear so blade-02 has HIGHER wear (0.30) than blade-01 (0.05)
    let mut ssd_wears_s2 = HashMap::new();
    ssd_wears_s2.insert("blade-01".to_string(), 0.05);
    ssd_wears_s2.insert("blade-02".to_string(), 0.30);
    ssd_wears_s2.insert("blade-03".to_string(), 0.10);

    let winner_s2 = scheduler
        .select_winner(&bids, &ssd_wears_s2, &chassis_active_vms_s2)?
        .ok_or("Expected a winner in Scenario 2")?;

    // blade-02 should win because its adjacent density (0) is lower than blade-01's (10),
    // taking precedence over its higher SSD wear.
    assert_eq!(winner_s2.node_id, "blade-02");

    // -------------------------------------------------------------
    // Scenario 3: Tie-breaker resolved by Slot Number Fallback
    // -------------------------------------------------------------
    // Both blade-02 and blade-03 have adjacent density = 0, and equal SSD wear = 0.10.
    // blade-01 is configured with higher wear (0.50).
    let chassis_active_vms_s3 = HashMap::new();
    let mut ssd_wears_s3 = HashMap::new();
    ssd_wears_s3.insert("blade-01".to_string(), 0.50);
    ssd_wears_s3.insert("blade-02".to_string(), 0.10);
    ssd_wears_s3.insert("blade-03".to_string(), 0.10);

    let winner_s3 = scheduler
        .select_winner(&bids, &ssd_wears_s3, &chassis_active_vms_s3)?
        .ok_or("Expected a winner in Scenario 3")?;

    // blade-02 should win because slot 2 < slot 3.
    assert_eq!(winner_s3.node_id, "blade-02");

    // Clean up mock servers
    h1.abort();
    h2.abort();
    h3.abort();

    Ok(())
}
