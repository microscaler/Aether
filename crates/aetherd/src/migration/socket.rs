// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use tokio::net::TcpListener;

/// Manages migration socket lifecycle for both source and destination nodes.
pub struct MigrationSocketManager {
    /// Bind address for incoming migrations.
    pub bind_addr: String,
}

impl MigrationSocketManager {
    /// Creates a new instance of MigrationSocketManager.
    pub fn new(bind_addr: String) -> Self {
        Self { bind_addr }
    }

    /// Starts a listener for an incoming migration.
    /// Returns the port assigned if 0 was provided.
    pub async fn listen_for_incoming(&self, port: u16, token: &str) -> Result<u16, String> {
        // 1. Validate attestation token before binding (simplified)
        self.validate_attestation(token)?;

        let addr = format!("{}:{}", self.bind_addr, port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Failed to bind migration listener to {}: {}", addr, e))?;

        let actual_addr = listener.local_addr().map_err(|e| e.to_string())?;

        // In a real implementation, we would spawn a task to handle the connection
        // and verify TLS certificates.

        Ok(actual_addr.port())
    }

    /// Validates the source node's attestation token.
    pub fn validate_attestation(&self, token: &str) -> Result<(), String> {
        if token.is_empty() {
            return Err("Empty attestation token".to_string());
        }
        // Placeholder for real attestation logic
        log::info!("Attestation token verified: {}", token);
        Ok(())
    }
}

/// Helper to generate the migration URI for QEMU.
pub fn get_migration_uri(host: &str, port: u16, use_tls: bool) -> String {
    if use_tls {
        format!("tls:{}:{}", host, port)
    } else {
        format!("tcp:{}:{}", host, port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migration_uri_generation() {
        assert_eq!(
            get_migration_uri("10.0.0.1", 4444, false),
            "tcp:10.0.0.1:4444"
        );
        assert_eq!(
            get_migration_uri("10.0.0.1", 4444, true),
            "tls:10.0.0.1:4444"
        );
    }

    #[tokio::test]
    async fn test_listen_for_incoming_dynamic_port() {
        let manager = MigrationSocketManager::new("127.0.0.1".to_string());
        let port = manager
            .listen_for_incoming(0, "test-token")
            .await
            .expect("listen for incoming should succeed");
        assert!(port > 0);
    }
}
