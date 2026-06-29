// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod bidder;
pub mod hypervisor;
pub mod telemetry;
pub mod vsock;

use std::sync::Arc;
use tonic::{Request, Response, Status};

use aether_auth::proto::aether_node_server::AetherNode;
use aether_auth::proto::{
    BidRequest, BidResponse, ExecuteVmRequest, ExecuteVmResponse, ListVMsRequest, ListVMsResponse,
    TeardownVmRequest, TeardownVmResponse,
};
use aether_auth::token::TokenManager;

/// gRPC service implementation for Aether Node Daemon.
pub struct AetherNodeImpl {
    pub node_id: String,
    pub token_manager: Arc<TokenManager>,
}

impl AetherNodeImpl {
    /// Creates a new instance of AetherNodeImpl.
    pub fn new(node_id: String, token_manager: Arc<TokenManager>) -> Self {
        Self {
            node_id,
            token_manager,
        }
    }
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
