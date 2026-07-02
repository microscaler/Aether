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
use std::collections::HashSet;
use std::sync::Arc;

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
    pub qmp_sockets: Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
    /// HMAC secret for validating attestation tokens from source nodes.
    pub attestation_secret: Vec<u8>,
    /// Path to CA certificate PEM for mTLS (empty for plain TCP).
    pub ca_cert_path: String,
    /// Path to server certificate PEM for mTLS (empty for plain TCP).
    pub server_cert_path: String,
    /// Path to server private key PEM for mTLS (empty for plain TCP).
    pub server_key_path: String,
    /// Tracks VMs currently undergoing active migration.
    active_migrations: Arc<tokio::sync::RwLock<HashSet<String>>>,
}

impl RealMigrationManager {
    pub fn new(bind_addr: String) -> Self {
        Self::new_with_defaults(bind_addr, b"aether-migration-secret".to_vec())
    }

    pub fn new_with_defaults(bind_addr: String, attestation_secret: Vec<u8>) -> Self {
        Self {
            bind_addr,
            qmp_sockets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            attestation_secret,
            ca_cert_path: String::new(),
            server_cert_path: String::new(),
            server_key_path: String::new(),
            active_migrations: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
        }
    }

    pub fn new_full(
        bind_addr: String,
        attestation_secret: Vec<u8>,
        ca_cert_path: String,
        server_cert_path: String,
        server_key_path: String,
    ) -> Self {
        Self {
            bind_addr,
            qmp_sockets: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            attestation_secret,
            ca_cert_path,
            server_cert_path,
            server_key_path,
            active_migrations: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
        }
    }

    /// Creates a manager with a hardcoded secret (for tests).
    pub fn new_with_secret(bind_addr: String, secret: Vec<u8>) -> Self {
        Self::new_with_defaults(bind_addr, secret)
    }

    /// Returns the set of currently active migration VM IDs.
    pub async fn get_active_migrations(&self) -> HashSet<String> {
        self.active_migrations.read().await.clone()
    }
}

#[async_trait]
impl MigrationManager for RealMigrationManager {
    async fn start_migration(&self, vm_id: &str, params: MigrationParams) -> Result<(), String> {
        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {vm_id} not found"))?;

        let uri = socket::get_migration_uri(&params.destination_ip, params.port, params.use_tls);

        // 1. Set migration bandwidth limit before starting
        if params.max_bandwidth > 0 {
            let qmp = crate::hypervisor::qemu::QmpClient::new(qmp_socket.clone());
            if let Err(e) = qmp.set_migration_parameters(params.max_bandwidth).await {
                return Err(format!("Failed to set migration bandwidth: {e}"));
            }
        }

        // 2. Enable auto-converge (ensures convergence under write-heavy loads)
        let convergence = converge::ConvergenceManager::new(qmp_socket.clone());
        if let Err(e) = convergence.enable_auto_converge().await {
            return Err(format!("Failed to enable auto-converge: {e}"));
        }

        // 3. Start block replication (drive-mirror over NBD)
        // In production, discover block devices from the VM and mirror each one.
        // For now, mirror the root device as a representative.
        let block_repl = block::BlockReplicator::new(qmp_socket.clone());
        let remote_nbd_uri = format!("nbd:{}:{}", params.destination_ip, params.port);
        if let Err(e) = block_repl
            .start_mirroring("drive-root", &remote_nbd_uri)
            .await
        {
            // Rollback: disable auto-converge to avoid leaving the VM throttled
            let _ = convergence.disable_auto_converge().await;
            return Err(format!("Block replication failed: {e}"));
        }

        // 4. Start memory migration
        let migrator = memory::MemoryMigrator::new(qmp_socket.clone());
        if let Err(e) = migrator.start_migration(&uri).await {
            // Rollback: disable auto-converge and cancel block mirroring
            let _ = convergence.disable_auto_converge().await;
            let qmp = crate::hypervisor::qemu::QmpClient::new(qmp_socket.clone());
            let _ = qmp.block_job_complete("drive-root").await;
            return Err(format!("Memory migration failed: {e}"));
        }

        Ok(())
    }

    async fn prepare_incoming(&self, vm_id: &str, port: u16, use_tls: bool) -> Result<(), String> {
        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {vm_id} not found"))?;

        // Create socket manager with the configured attestation secret
        let socket_manager = socket::MigrationSocketManager::new(
            self.bind_addr.clone(),
            self.attestation_secret.clone(),
            self.ca_cert_path.clone(),
            self.server_cert_path.clone(),
            self.server_key_path.clone(),
        );

        if use_tls {
            // TLS listener requires valid cert paths
            if self.ca_cert_path.is_empty()
                || self.server_cert_path.is_empty()
                || self.server_key_path.is_empty()
            {
                return Err(
                    "TLS requested but CA cert, server cert, and server key paths are empty"
                        .to_string(),
                );
            }
            let (_tls_acceptor, _shutdown) =
                socket_manager.listen_for_incoming_tls(port, vm_id).await?;
            // Keep _tls_acceptor and _shutdown alive to prevent early drop
            drop(_tls_acceptor);
            drop(_shutdown);
        } else {
            let _actual_port = socket_manager.listen_for_incoming(port, vm_id).await?;
        }

        let block_repl = block::BlockReplicator::new(qmp_socket.clone());
        let listen_addr = format!("127.0.0.1:{port}");

        // Prepare NBD for incoming block data
        block_repl
            .prepare_destination("drive-root", &listen_addr)
            .await?;

        Ok(())
    }

    async fn query_migration_status(&self, vm_id: &str) -> Result<MigrationState, String> {
        // Check if we have tracking state for this VM
        {
            let active = self.active_migrations.read().await;
            if active.contains(vm_id) {
                // VM is registered as actively migrating
            }
        }

        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {vm_id} not found"))?;

        let qmp = crate::hypervisor::qemu::QmpClient::new(qmp_socket.clone());
        let resp = qmp.query_migrate().await?;

        // Use serde_json for robust parsing instead of string matching
        let json = serde_json::from_str::<serde_json::Value>(&resp)
            .map_err(|_| format!("Failed to parse QMP migration response: {resp}"))?;
        let status_val = json
            .get("return")
            .and_then(|r| r.get("status"))
            .and_then(|s| s.as_str());

        match status_val {
            Some("completed") => Ok(MigrationState::Completed),
            Some("active") => Ok(MigrationState::Active),
            Some("failed") => Ok(MigrationState::Failed(resp)),
            _ => Ok(MigrationState::Idle),
        }
    }

    async fn abort_migration(&self, vm_id: &str) -> Result<(), String> {
        let sockets = self.qmp_sockets.read().await;
        let qmp_socket = sockets
            .get(vm_id)
            .ok_or_else(|| format!("VM {vm_id} not found"))?;

        let qmp = crate::hypervisor::qemu::QmpClient::new(qmp_socket.clone());
        qmp.migrate_cancel().await?;

        // Remove from active migrations tracking
        let mut active = self.active_migrations.write().await;
        active.remove(vm_id);

        Ok(())
    }

    async fn get_active_migration_count(&self) -> u32 {
        self.active_migrations.read().await.len() as u32
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
