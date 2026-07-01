// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::hypervisor::qemu::QmpClient;

/// Orchestrates block level replication using NBD and QMP drive-mirror.
pub struct BlockReplicator {
    qmp_socket: String,
}

impl BlockReplicator {
    /// Creates a new BlockReplicator instance.
    pub fn new(qmp_socket: String) -> Self {
        Self { qmp_socket }
    }

    /// Prepares the destination node to receive a block stream.
    pub async fn prepare_destination(
        &self,
        device_id: &str,
        listen_addr: &str,
    ) -> Result<(), String> {
        let qmp = QmpClient::new(self.qmp_socket.clone());

        // 1. Start NBD server
        qmp.nbd_server_start(listen_addr).await?;

        // 2. Add device to NBD server
        qmp.nbd_server_add(device_id).await?;

        Ok(())
    }

    /// Initiates mirroring from the source node.
    pub async fn start_mirroring(
        &self,
        device_id: &str,
        remote_nbd_uri: &str,
    ) -> Result<(), String> {
        let qmp = QmpClient::new(self.qmp_socket.clone());

        // Start drive-mirror to the remote NBD target
        qmp.drive_mirror(device_id, remote_nbd_uri).await?;

        Ok(())
    }
}
