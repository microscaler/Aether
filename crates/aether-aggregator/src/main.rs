// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;

use aether_aggregator::registry::NodeRegistry;
use aether_aggregator::AetherAggregatorImpl;
use aether_auth::mtls::create_server_tls_config;
use aether_auth::proto::aether_aggregator_server::AetherAggregatorServer;
use aether_auth::token::TokenManager;

#[tokio::main]
#[allow(clippy::unwrap_used)] // Allowed strictly on entrypoint startup path
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    println!("Starting Aether Aggregator on {addr}");

    // Dynamic PKI generation for development / testing out of the box
    let creds = aether_auth::mtls::test_pki::generate_test_creds()?;
    let server_tls_config =
        create_server_tls_config(&creds.ca_cert, &creds.server_cert, &creds.server_key);

    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let registry = Arc::new(RwLock::new(NodeRegistry::new()));

    // Spawn a background task to prune inactive nodes every 3 seconds
    let registry_clone = Arc::clone(&registry);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            interval.tick().await;
            let mut reg = registry_clone.write().await;
            let pruned = reg.prune_inactive_nodes(std::time::Duration::from_secs(15));
            if !pruned.is_empty() {
                log::warn!("Pruned inactive nodes: {:?}", pruned);
            }
        }
    });

    let aggregator = AetherAggregatorImpl::new(registry, token_manager);

    Server::builder()
        .tls_config(server_tls_config)?
        .add_service(AetherAggregatorServer::new(aggregator))
        .serve(addr)
        .await?;

    Ok(())
}
