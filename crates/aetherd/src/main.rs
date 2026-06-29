// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use aether_auth::mtls::{create_client_tls_config, create_server_tls_config};
use aether_auth::proto::aether_aggregator_client::AetherAggregatorClient;
use aether_auth::proto::aether_node_server::{AetherNode, AetherNodeServer};
use aether_auth::proto::{
    BidRequest, BidResponse, ExecuteVmRequest, ExecuteVmResponse, ListVMsRequest, ListVMsResponse,
    RegisterNodeRequest, TeardownVmRequest, TeardownVmResponse,
};
use aether_auth::token::TokenManager;

/// gRPC service implementation for Aether Node Daemon.
pub struct AetherNodeImpl {
    node_id: String,
    token_manager: Arc<TokenManager>,
}

#[tonic::async_trait]
impl AetherNode for AetherNodeImpl {
    async fn request_reverse_bid(
        &self,
        _request: Request<BidRequest>,
    ) -> Result<Response<BidResponse>, Status> {
        Ok(Response::new(BidResponse {
            node_id: self.node_id.clone(),
            score: 950, // Mock healthy score
        }))
    }

    async fn execute_vm(
        &self,
        request: Request<ExecuteVmRequest>,
    ) -> Result<Response<ExecuteVmResponse>, Status> {
        let req = request.into_inner();
        self.token_manager
            .validate_token(&req.token, &self.node_id)
            .map_err(Status::unauthenticated)?;

        Ok(Response::new(ExecuteVmResponse {
            success: true,
            ip_address: "192.168.1.100".to_string(),
            mac_address: "52:54:00:12:34:56".to_string(),
            error_message: String::new(),
        }))
    }

    async fn teardown_vm(
        &self,
        request: Request<TeardownVmRequest>,
    ) -> Result<Response<TeardownVmResponse>, Status> {
        let req = request.into_inner();
        self.token_manager
            .validate_token(&req.token, &self.node_id)
            .map_err(Status::unauthenticated)?;

        Ok(Response::new(TeardownVmResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn list_v_ms(
        &self,
        _request: Request<ListVMsRequest>,
    ) -> Result<Response<ListVMsResponse>, Status> {
        Ok(Response::new(ListVMsResponse { vms: vec![] }))
    }
}

#[tokio::main]
#[allow(clippy::unwrap_used)] // Allowed strictly on entrypoint startup path
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let node_id = "blade-01".to_string();
    let local_addr: SocketAddr = "127.0.0.1:50052".parse().unwrap();

    let creds = aether_auth::mtls::test_pki::generate_test_creds()?;
    let client_tls_config = create_client_tls_config(
        &creds.ca_cert,
        &creds.client_cert,
        &creds.client_key,
        "localhost",
    );
    let server_tls_config =
        create_server_tls_config(&creds.ca_cert, &creds.server_cert, &creds.server_key);

    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));

    // Expose Node Daemon Server API
    let daemon_service = AetherNodeImpl {
        node_id: node_id.clone(),
        token_manager: token_manager.clone(),
    };

    println!("Starting Aether Node Daemon on {local_addr}");
    let server_handle = tokio::spawn(async move {
        let _ = Server::builder()
            .tls_config(server_tls_config)
            .unwrap()
            .add_service(AetherNodeServer::new(daemon_service))
            .serve(local_addr)
            .await;
    });

    // Briefly sleep to ensure Server starts
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect to Aggregator Client
    let channel = tonic::transport::Channel::from_static("https://127.0.0.1:50051")
        .tls_config(client_tls_config)?
        .connect()
        .await?;

    let mut client = AetherAggregatorClient::new(channel);
    let response = client
        .register_node(Request::new(RegisterNodeRequest {
            node_id: node_id.clone(),
            grpc_endpoint: format!("https://{local_addr}"),
            pool: "COMPUTE".to_string(),
        }))
        .await?;

    println!("Registered with Aggregator: {:?}", response.into_inner());

    // Join and block on server handle
    let _ = server_handle.await;
    Ok(())
}
