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

        // 1. Set migration parameters if needed (e.g. bandwidth)
        // (Placeholder for set_migration_parameters)

        // 2. Start migration
        qmp.migrate(destination_uri).await?;

        Ok(())
    }

    /// Polls for migration completion.
    pub async fn wait_for_completion(&self) -> Result<(), String> {
        let qmp = QmpClient::new(self.qmp_socket.clone());

        loop {
            let status = qmp.query_migrate().await?;
            if status.contains("\"status\": \"completed\"")
                || status.contains("\"status\":\"completed\"")
            {
                return Ok(());
            }
            if status.contains("\"status\": \"failed\"") || status.contains("\"status\":\"failed\"")
            {
                return Err(format!("Migration failed in QEMU: {}", status));
            }
            if status.contains("\"status\": \"cancelled\"")
                || status.contains("\"status\":\"cancelled\"")
            {
                return Err("Migration was cancelled".to_string());
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
