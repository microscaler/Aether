// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use aether_auth::mtls::create_server_tls_config;
use aether_auth::proto::aether_aggregator_server::{AetherAggregator, AetherAggregatorServer};
use aether_auth::proto::{
    HeartbeatRequest, HeartbeatResponse, RegisterNodeRequest, RegisterNodeResponse,
};
use aether_auth::token::TokenManager;

/// Struct holding the active registered nodes in memory.
#[derive(Default)]
pub struct AggregatorState {
    nodes: HashMap<String, String>, // node_id -> token
}

/// gRPC service implementation for Aether Aggregator.
pub struct AetherAggregatorImpl {
    state: Arc<RwLock<AggregatorState>>,
    token_manager: Arc<TokenManager>,
}

impl AetherAggregatorImpl {
    /// Creates a new instance of AetherAggregatorImpl.
    pub fn new(token_manager: Arc<TokenManager>) -> Self {
        Self {
            state: Arc::new(RwLock::new(AggregatorState::default())),
            token_manager,
        }
    }
}

#[tonic::async_trait]
impl AetherAggregator for AetherAggregatorImpl {
    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let req = request.into_inner();
        let token = self
            .token_manager
            .generate_token(&req.node_id)
            .map_err(Status::internal)?;

        let mut state = self.state.write().await;
        state.nodes.insert(req.node_id, token.clone());

        Ok(Response::new(RegisterNodeResponse {
            success: true,
            token,
        }))
    }

    async fn send_heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();

        // Validate the ephemeral token
        self.token_manager
            .validate_token(&req.token, &req.node_id)
            .map_err(Status::unauthenticated)?;

        Ok(Response::new(HeartbeatResponse { success: true }))
    }
}

#[tokio::main]
#[allow(clippy::unwrap_used)] // Allowed strictly on entrypoint startup path
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    println!("Starting Aether Aggregator on {addr}");

    // Dynamic PKI generation for development / testing out of the box
    let creds = aether_auth::mtls::test_pki::generate_test_creds()?;
    let server_tls_config = create_server_tls_config(
        &creds.ca_cert,
        &creds.server_cert,
        &creds.server_key,
    );

    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let aggregator = AetherAggregatorImpl::new(token_manager);

    Server::builder()
        .tls_config(server_tls_config)?
        .add_service(AetherAggregatorServer::new(aggregator))
        .serve(addr)
        .await?;

    Ok(())
}
