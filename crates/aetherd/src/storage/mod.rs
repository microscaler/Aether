pub mod iscsi;
pub mod zfs;

use async_trait::async_trait;
use std::io;

/// Trait to manage ZFS volumes (ZVOLs) programmatically.
///
/// In the Aether systems integration design, these ZVOLs are provisioned by
/// `aetherd` on dedicated Storage nodes (Slots 9-16). They are subsequently exposed
/// as network-attached iSCSI targets over VLAN 11 (Storage Fabric) and logged into by
/// Compute nodes (initiators) running VMs.
#[async_trait]
pub trait ZvolManager: Send + Sync {
    /// Create a new ZVOL with the given name and size in bytes.
    /// Returns the absolute path to the ZVOL block device (e.g., `/dev/zvol/tank/zvol-name`).
    async fn create_zvol(&self, name: &str, size_bytes: u64) -> io::Result<String>;

    /// Create a snapshot of a ZVOL.
    async fn create_snapshot(&self, zvol_name: &str, snapshot_name: &str) -> io::Result<()>;

    /// Create a thin clone from an existing snapshot.
    /// Returns the absolute path to the cloned ZVOL block device.
    async fn clone_zvol(&self, snapshot_name: &str, clone_name: &str) -> io::Result<String>;

    /// Rollback a ZVOL to a snapshot.
    async fn rollback_zvol(&self, zvol_name: &str, snapshot_name: &str) -> io::Result<()>;

    /// Resize a ZVOL.
    async fn resize_zvol(&self, zvol_name: &str, new_size_bytes: u64) -> io::Result<()>;

    /// Destroy a ZVOL or snapshot.
    async fn destroy_zvol(&self, name: &str) -> io::Result<()>;

    /// Configure the ZFS ARC cache limit on the host to 15% of total memory.
    async fn configure_arc_cache_limit(&self) -> io::Result<()>;
}
