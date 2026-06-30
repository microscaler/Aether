pub mod bridge;

use async_trait::async_trait;
use std::io;

/// Trait to manage guest virtual networking (Bridges, TAPs, and MAC anti-spoofing) programmatically.
#[async_trait]
pub trait BridgeManager: Send + Sync {
    /// Create a dynamic tenant bridge `br-tenant-<vlan_id>` and set it up.
    async fn create_tenant_bridge(&self, vlan_id: u16) -> io::Result<String>;

    /// Create a TAP interface, set its state UP, and bind it to the master bridge.
    async fn create_tap_device(&self, tap_name: &str, bridge_name: &str) -> io::Result<()>;

    /// Apply MAC spoofing prevention rules on the bridge for the given TAP device and allowed guest MAC address.
    async fn apply_mac_anti_spoofing(&self, tap_name: &str, allowed_mac: &str) -> io::Result<()>;

    /// Clean up the bridge, TAP device, and firewall rules.
    async fn teardown_tenant_network(&self, vlan_id: u16, tap_name: &str) -> io::Result<()>;
}
