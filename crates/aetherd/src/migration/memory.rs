// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::hypervisor::qemu::QmpClient;
use std::time::Duration;

/// Orchestrates memory pre-copy migration.
pub struct MemoryMigrator {
    qmp_socket: String,
}

impl MemoryMigrator {
    /// Creates a new MemoryMigrator instance.
    pub fn new(qmp_socket: String) -> Self {
        Self { qmp_socket }
    }

    /// Starts the memory migration to the destination URI.
    pub async fn start_migration(&self, destination_uri: &str) -> Result<(), String> {
        let qmp = QmpClient::new(self.qmp_socket.clone());

        // Start migration
        qmp.migrate(destination_uri).await?;

        Ok(())
    }

    /// Polls for migration completion using structured JSON parsing.
    pub async fn wait_for_completion(&self) -> Result<(), String> {
        let qmp = QmpClient::new(self.qmp_socket.clone());

        loop {
            let status = qmp.query_migrate().await?;

            // Use serde_json for robust parsing instead of fragile string matching
            let json = serde_json::from_str::<serde_json::Value>(&status)
                .map_err(|_| format!("Failed to parse migration status: {status}"))?;
            let status_val = json
                .get("return")
                .and_then(|r| r.get("status"))
                .and_then(|s| s.as_str());

            match status_val {
                Some("completed") => return Ok(()),
                Some("failed") => return Err(format!("Migration failed in QEMU: {status}")),
                Some("cancelled") => return Err("Migration was cancelled".to_string()),
                Some("postcopy-failed") => {
                    return Err(format!("Migration failed in post-copy phase: {status}"))
                }
                // "setup", "cancelled", "error", "recover", or unknown → keep polling
                _ => {}
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
