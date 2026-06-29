// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod firecracker;

use async_trait::async_trait;

/// The common trait for managing different hypervisor execution engines.
#[async_trait]
pub trait Hypervisor: Send + Sync {
    /// Spawns the hypervisor process and boots the guest VM.
    async fn spawn(&self) -> Result<(), String>;

    /// Stops the hypervisor process and terminates the VM.
    async fn stop(&self) -> Result<(), String>;

    /// Queries the execution status of the VM.
    async fn query_status(&self) -> Result<String, String>;
}
