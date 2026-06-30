// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::transport::Server;
use tonic::Request;

use aether_auth::mtls::{create_client_tls_config, create_server_tls_config};
use aether_auth::proto::aether_aggregator_client::AetherAggregatorClient;
use aether_auth::proto::aether_node_server::AetherNodeServer;
use aether_auth::proto::RegisterNodeRequest;
use aether_auth::token::TokenManager;
use aetherd::AetherNodeImpl;

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

    let pool = "COMPUTE".to_string();
    let telemetry_collector = Arc::new(aetherd::telemetry::TelemetryCollector::new(
        aetherd::telemetry::TelemetryConfig::default(),
    ));
    let bidder = Arc::new(aetherd::bidder::Bidder::new(
        aetherd::bidder::BidderConfig::default(),
    ));

    // Expose Node Daemon Server API
    let daemon_service = AetherNodeImpl::new(
        node_id.clone(),
        pool.clone(),
        token_manager.clone(),
        telemetry_collector,
        bidder,
    );

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
