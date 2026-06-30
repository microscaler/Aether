// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![allow(missing_docs)]

pub mod network;
pub mod registry;
pub mod scheduler;
pub mod storage;
pub mod tie_breaker;

use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use aether_auth::proto::aether_aggregator_server::AetherAggregator;
use aether_auth::proto::{
    HeartbeatRequest, HeartbeatResponse, RegisterNodeRequest, RegisterNodeResponse,
};
use aether_auth::token::TokenManager;
use registry::NodeRegistry;

/// gRPC service implementation for Aether Aggregator.
pub struct AetherAggregatorImpl {
    pub registry: Arc<RwLock<NodeRegistry>>,
    pub token_manager: Arc<TokenManager>,
}

impl AetherAggregatorImpl {
    /// Creates a new instance of AetherAggregatorImpl.
    pub fn new(registry: Arc<RwLock<NodeRegistry>>, token_manager: Arc<TokenManager>) -> Self {
        Self {
            registry,
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

        let mut registry = self.registry.write().await;
        registry.register(req.node_id, req.grpc_endpoint, req.pool, token.clone());

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

        let mut registry = self.registry.write().await;
        registry
            .renew_heartbeat(&req.node_id, &req.token)
            .map_err(Status::invalid_argument)?;

        Ok(Response::new(HeartbeatResponse { success: true }))
    }
}
