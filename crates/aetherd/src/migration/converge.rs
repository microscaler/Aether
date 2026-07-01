// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::hypervisor::qemu::QmpClient;

/// Manages auto-convergence throttling for migration.
pub struct ConvergenceManager {
    qmp_socket: String,
}

impl ConvergenceManager {
    /// Creates a new ConvergenceManager instance.
    pub fn new(qmp_socket: String) -> Self {
        Self { qmp_socket }
    }

    /// Enables auto-converge for the current migration.
    pub async fn enable_auto_converge(&self) -> Result<(), String> {
        let qmp = QmpClient::new(self.qmp_socket.clone());
        qmp.set_migration_capability("auto-converge", true).await
    }

    /// Disables auto-converge.
    pub async fn disable_auto_converge(&self) -> Result<(), String> {
        let qmp = QmpClient::new(self.qmp_socket.clone());
        qmp.set_migration_capability("auto-converge", false).await
    }
}
