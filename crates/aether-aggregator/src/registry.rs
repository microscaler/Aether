// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use tokio::time::Instant;

/// Metadata stored for each active registered node daemon.
#[derive(Clone, Debug)]
pub struct NodeInfo {
    /// Unique identifier for the blade node.
    pub node_id: String,
    /// Private control plane gRPC endpoint (mTLS).
    pub grpc_endpoint: String,
    /// Pool profile allocation: "COMPUTE" or "INFRA".
    pub pool: String,
    /// Ephemeral attestation token for heartbeat and commands.
    pub token: String,
    /// Timestamp of last received heartbeat.
    pub last_seen_heartbeat: Instant,
}

/// In-memory thread-safe registry mapping node IDs to their active metadata.
#[derive(Default)]
pub struct NodeRegistry {
    nodes: HashMap<String, NodeInfo>,
}

impl NodeRegistry {
    /// Creates a new empty NodeRegistry.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Registers or updates a blade node in the registry.
    pub fn register(
        &mut self,
        node_id: String,
        grpc_endpoint: String,
        pool: String,
        token: String,
    ) {
        let node_info = NodeInfo {
            node_id: node_id.clone(),
            grpc_endpoint,
            pool,
            token,
            last_seen_heartbeat: Instant::now(),
        };
        self.nodes.insert(node_id, node_info);
    }

    /// Renews the heartbeat lease of a node, verifying its token.
    pub fn renew_heartbeat(&mut self, node_id: &str, token: &str) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            if node.token == token {
                node.last_seen_heartbeat = Instant::now();
                Ok(())
            } else {
                Err("Token verification failed".to_string())
            }
        } else {
            Err("Node is not registered".to_string())
        }
    }

    /// Deregisters and removes a node from the registry.
    pub fn deregister(&mut self, node_id: &str) -> Option<NodeInfo> {
        self.nodes.remove(node_id)
    }

    /// Prunes any nodes that have not reported a heartbeat within the specified duration threshold.
    /// Returns the list of pruned node IDs.
    pub fn prune_inactive_nodes(&mut self, threshold: std::time::Duration) -> Vec<String> {
        let now = Instant::now();
        let mut pruned = Vec::new();
        self.nodes.retain(|node_id, node| {
            if now.saturating_duration_since(node.last_seen_heartbeat) > threshold {
                pruned.push(node_id.clone());
                false
            } else {
                true
            }
        });
        pruned
    }

    /// Returns a list of currently active nodes.
    pub fn get_active_nodes(&self) -> Vec<NodeInfo> {
        self.nodes.values().cloned().collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_node_registration_and_renewal() {
        let mut registry = NodeRegistry::new();
        let node_id = "blade-01".to_string();
        let token = "token_123".to_string();

        registry.register(
            node_id.clone(),
            "https://127.0.0.1:50052".to_string(),
            "COMPUTE".to_string(),
            token.clone(),
        );

        let active = registry.get_active_nodes();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].node_id, node_id);

        // Heartbeat renewal
        assert!(registry.renew_heartbeat(&node_id, &token).is_ok());

        // Heartbeat renewal with wrong token fails
        assert!(registry.renew_heartbeat(&node_id, "wrong_token").is_err());

        // Heartbeat renewal with unregistered node fails
        assert!(registry.renew_heartbeat("blade-02", &token).is_err());
    }

    #[test]
    fn test_node_pruning() {
        let mut registry = NodeRegistry::new();
        let node_id = "blade-01".to_string();
        let token = "token_123".to_string();

        registry.register(
            node_id.clone(),
            "https://127.0.0.1:50052".to_string(),
            "COMPUTE".to_string(),
            token,
        );

        // Pruning with 0 threshold should prune immediately
        let pruned = registry.prune_inactive_nodes(Duration::from_secs(0));
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned[0], node_id);
        assert_eq!(registry.get_active_nodes().len(), 0);
    }

    #[test]
    fn test_node_deregistration() {
        let mut registry = NodeRegistry::new();
        let node_id = "blade-01".to_string();
        let token = "token_123".to_string();

        registry.register(
            node_id.clone(),
            "https://127.0.0.1:50052".to_string(),
            "COMPUTE".to_string(),
            token,
        );

        assert_eq!(registry.get_active_nodes().len(), 1);
        let removed = registry.deregister(&node_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().node_id, node_id);
        assert_eq!(registry.get_active_nodes().len(), 0);

        // Deregistering non-existent node returns None
        assert!(registry.deregister("blade-02").is_none());
    }
}
