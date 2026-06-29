use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use aether_auth::mtls::{create_client_tls_config, create_server_tls_config, test_pki};
use aether_auth::proto::aether_aggregator_server::{AetherAggregator, AetherAggregatorServer};
use aether_auth::proto::aether_aggregator_client::AetherAggregatorClient;
use aether_auth::proto::{
    HeartbeatRequest, HeartbeatResponse, RegisterNodeRequest, RegisterNodeResponse,
};
use aether_auth::token::TokenManager;

struct MockAggregator {
    token_manager: Arc<TokenManager>,
}

#[tonic::async_trait]
impl AetherAggregator for MockAggregator {
    async fn register_node(
        &self,
        request: Request<RegisterNodeRequest>,
    ) -> Result<Response<RegisterNodeResponse>, Status> {
        let req = request.into_inner();
        let token = self
            .token_manager
            .generate_token(&req.node_id)
            .map_err(Status::internal)?;
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
        self.token_manager
            .validate_token(&req.token, &req.node_id)
            .map_err(Status::unauthenticated)?;
        Ok(Response::new(HeartbeatResponse { success: true }))
    }
}

#[tokio::test]
async fn test_mtls_client_server_integration() {
    let addr: SocketAddr = "127.0.0.1:50061".parse().unwrap();
    let creds = test_pki::generate_test_creds().unwrap();

    let server_tls = create_server_tls_config(
        &creds.ca_cert,
        &creds.server_cert,
        &creds.server_key,
    );

    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let service = MockAggregator { token_manager };

    // Spawn server in background
    let server_handle = tokio::spawn(async move {
        let _ = Server::builder()
            .tls_config(server_tls)
            .unwrap()
            .add_service(AetherAggregatorServer::new(service))
            .serve(addr)
            .await;
    });

    // Small delay to let server start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect with valid mTLS client
    let client_tls = create_client_tls_config(
        &creds.ca_cert,
        &creds.client_cert,
        &creds.client_key,
        "localhost",
    );

    let channel = tonic::transport::Channel::from_static("https://127.0.0.1:50061")
        .tls_config(client_tls)
        .unwrap()
        .connect()
        .await
        .unwrap();

    let mut client = AetherAggregatorClient::new(channel);
    let res = client
        .register_node(Request::new(RegisterNodeRequest {
            node_id: "test-node".to_string(),
            grpc_endpoint: "https://127.0.0.1:50062".to_string(),
            pool: "COMPUTE".to_string(),
        }))
        .await
        .unwrap();

    assert!(res.into_inner().success);

    // Clean up server
    server_handle.abort();
}

#[tokio::test]
async fn test_mtls_client_without_cert_fails() {
    let addr: SocketAddr = "127.0.0.1:50063".parse().unwrap();
    let creds = test_pki::generate_test_creds().unwrap();

    let server_tls = create_server_tls_config(
        &creds.ca_cert,
        &creds.server_cert,
        &creds.server_key,
    );

    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let service = MockAggregator { token_manager };

    let server_handle = tokio::spawn(async move {
        let _ = Server::builder()
            .tls_config(server_tls)
            .unwrap()
            .add_service(AetherAggregatorServer::new(service))
            .serve(addr)
            .await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Try connecting with a client that only trusts the CA but does NOT send a client identity
    let client_tls = tonic::transport::ClientTlsConfig::new()
        .ca_certificate(tonic::transport::Certificate::from_pem(&creds.ca_cert))
        .domain_name("localhost");

    let channel = tonic::transport::Channel::from_static("https://127.0.0.1:50063")
        .tls_config(client_tls)
        .unwrap()
        .connect()
        .await
        .unwrap();

    let mut client = AetherAggregatorClient::new(channel);
    let res = client
        .register_node(Request::new(RegisterNodeRequest {
            node_id: "test-node".to_string(),
            grpc_endpoint: "https://127.0.0.1:50064".to_string(),
            pool: "COMPUTE".to_string(),
        }))
        .await;

    // Connection must fail due to missing client certificate
    assert!(res.is_err());

    server_handle.abort();
}
