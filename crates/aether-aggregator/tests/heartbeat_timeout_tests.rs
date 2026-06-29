// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use tokio::sync::RwLock;

use aether_aggregator::registry::NodeRegistry;
use aether_aggregator::AetherAggregatorImpl;
use aether_auth::proto::aether_aggregator_server::AetherAggregator;
use aether_auth::proto::{HeartbeatRequest, RegisterNodeRequest};
use aether_auth::token::TokenManager;

#[tokio::test]
async fn test_heartbeat_timeout_pruning() -> Result<(), Box<dyn std::error::Error>> {
    // Pause time to allow fast-forwarding the tokio scheduler
    tokio::time::pause();

    let registry = Arc::new(RwLock::new(NodeRegistry::new()));
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));

    // Spawn the background pruner task
    let registry_clone = Arc::clone(&registry);
    let pruner_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            interval.tick().await;
            let mut reg = registry_clone.write().await;
            let _pruned = reg.prune_inactive_nodes(std::time::Duration::from_secs(15));
        }
    });

    let aggregator = AetherAggregatorImpl::new(Arc::clone(&registry), token_manager);

    // Register node
    let register_res = aggregator
        .register_node(tonic::Request::new(RegisterNodeRequest {
            node_id: "test-node-1".to_string(),
            grpc_endpoint: "https://127.0.0.1:50080".to_string(),
            pool: "COMPUTE".to_string(),
        }))
        .await?;

    let token = register_res.into_inner().token;

    // Verify registered
    {
        let reg = registry.read().await;
        assert_eq!(reg.get_active_nodes().len(), 1);
    }

    // Advance time by 6 seconds
    tokio::time::advance(std::time::Duration::from_secs(6)).await;
    // Yield to let the pruner task run (even though it shouldn't prune yet)
    tokio::task::yield_now().await;

    // Send heartbeat to renew lease
    let heartbeat_res = aggregator
        .send_heartbeat(tonic::Request::new(HeartbeatRequest {
            node_id: "test-node-1".to_string(),
            token: token.clone(),
        }))
        .await?;

    assert!(heartbeat_res.into_inner().success);

    // Verify still registered
    {
        let reg = registry.read().await;
        assert_eq!(reg.get_active_nodes().len(), 1);
    }

    // Advance time by 12 seconds
    tokio::time::advance(std::time::Duration::from_secs(12)).await;
    tokio::task::yield_now().await;

    // Verify still registered (less than 15s since last heartbeat)
    {
        let reg = registry.read().await;
        assert_eq!(reg.get_active_nodes().len(), 1);
    }

    // Advance time by another 4 seconds (total 16s since last heartbeat)
    tokio::time::advance(std::time::Duration::from_secs(4)).await;
    // Wait a brief moment for background pruner task to complete its transaction
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Verify pruned
    {
        let reg = registry.read().await;
        assert_eq!(reg.get_active_nodes().len(), 0);
    }

    pruner_handle.abort();
    Ok(())
}
