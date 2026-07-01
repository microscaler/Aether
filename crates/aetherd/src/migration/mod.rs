// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod block;
pub mod converge;
pub mod memory;
pub mod socket;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Current state of a migration operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationState {
    /// No migration active.
    Idle,
    /// Preparing destination resources (ZVOLs, sockets).
    Preparing,
    /// Destination is listening for incoming data.
    Listening,
    /// Data transfer in progress.
    Active,
    /// Migration completed successfully.
    Completed,
    /// Migration failed.
    Failed(String),
    /// Migration cancelled by user.
    Cancelled,
}

/// Parameters for a migration request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationParams {
    /// Destination node ID.
    pub destination_node: String,
    /// Destination IP address for the migration socket.
    pub destination_ip: String,
    /// Port for the migration socket.
    pub port: u16,
    /// Whether to enable TLS.
    pub use_tls: bool,
    /// Maximum bandwidth in bits per second.
    pub max_bandwidth: u64,
}

#[async_trait]
pub trait MigrationManager: Send + Sync {
    /// Initiates a migration as the source node.
    async fn start_migration(&self, vm_id: &str, params: MigrationParams) -> Result<(), String>;

    /// Prepares for an incoming migration as the destination node.
    async fn prepare_incoming(&self, vm_id: &str, port: u16, use_tls: bool) -> Result<(), String>;

    /// Queries the current status of an active migration.
    async fn query_migration_status(&self, vm_id: &str) -> Result<MigrationState, String>;

    /// Aborts an active migration.
    async fn abort_migration(&self, vm_id: &str) -> Result<(), String>;

    /// Returns the number of currently active migrations.
    async fn get_active_migration_count(&self) -> u32;

    /// Registers a VM with the migration manager.
    async fn register_vm(&self, vm_id: &str, qmp_socket: &str) -> Result<(), String>;

    /// Unregisters a VM from the migration manager.
    async fn unregister_vm(&self, vm_id: &str) -> Result<(), String>;
}

/// Real implementation of MigrationManager interacting with QEMU hypervisors.
pub struct RealMigrationManager {
    /// Bind address for incoming migrations.
    pub bind_addr: String,
    /// Map of VM ID to QMP socket path.
    pub qmp_sockets: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
}

impl RealMigrationManager {
    pub fn new(bind_addr: String) -> Self {
        Self {
            bind_addr,
            qmp_sockets: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }
}

#[async_trait]
impl MigrationManager for RealMigrationManager {
    async fn start_migration(&self, vm_id: &str, params: MigrationParams) -> Result<(), String> {
        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let migrator = memory::MemoryMigrator::new(qmp_socket.clone());
        let uri = socket::get_migration_uri(&params.destination_ip, params.port, params.use_tls);

        // 1. Enable auto-converge if requested (defaulting to true for now to ensure convergence)
        let convergence = converge::ConvergenceManager::new(qmp_socket.clone());
        convergence.enable_auto_converge().await?;

        // 2. Start mirroring if there are drives (simplified here)
        // (Placeholder for drive discovery and mirroring)

        // 3. Start memory migration
        migrator.start_migration(&uri).await?;

        Ok(())
    }

    async fn prepare_incoming(&self, vm_id: &str, port: u16, _use_tls: bool) -> Result<(), String> {
        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        // 1. Start the migration listener with attestation verification
        let socket_manager = socket::MigrationSocketManager::new(self.bind_addr.clone());
        let _actual_port = socket_manager
            .listen_for_incoming(port, "ephemeral-migration-token")
            .await?;

        let block_repl = block::BlockReplicator::new(qmp_socket.clone());
        let listen_addr = format!("0.0.0.0:{}", port);

        // 2. Prepare NBD for incoming block data
        block_repl
            .prepare_destination("drive-root", &listen_addr)
            .await?;

        Ok(())
    }

    async fn query_migration_status(&self, vm_id: &str) -> Result<MigrationState, String> {
        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        let qmp = crate::hypervisor::qemu::QmpClient::new(qmp_socket.clone());
        let resp = qmp.query_migrate().await?;

        if resp.contains("\"status\": \"completed\"") {
            Ok(MigrationState::Completed)
        } else if resp.contains("\"status\": \"active\"") {
            Ok(MigrationState::Active)
        } else if resp.contains("\"status\": \"failed\"") {
            Ok(MigrationState::Failed(resp))
        } else {
            Ok(MigrationState::Idle)
        }
    }

    async fn abort_migration(&self, vm_id: &str) -> Result<(), String> {
        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {} not found", vm_id))?;

        // Send migrate_cancel command
        let _qmp = crate::hypervisor::qemu::QmpClient::new(qmp_socket.clone());
        // Placeholder for qmp.migrate_cancel().await?;
        Ok(())
    }

    async fn get_active_migration_count(&self) -> u32 {
        let sockets = self.qmp_sockets.read().await;
        // In a real implementation, we would track active migration tasks.
        // For now, we return the count of VMs registered for migration.
        sockets.len() as u32
    }

    async fn register_vm(&self, vm_id: &str, qmp_socket: &str) -> Result<(), String> {
        let mut sockets = self.qmp_sockets.write().await;
        sockets.insert(vm_id.to_string(), qmp_socket.to_string());
        Ok(())
    }

    async fn unregister_vm(&self, vm_id: &str) -> Result<(), String> {
        let mut sockets = self.qmp_sockets.write().await;
        sockets.remove(vm_id);
        Ok(())
    }
}
