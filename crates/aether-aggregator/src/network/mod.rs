pub mod hpe_vc;

use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error code {code}: {message}")]
    Api { code: String, message: String },
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("JSON processing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Unexpected error: {0}")]
    Other(String),
}

#[async_trait]
pub trait MidplaneNetworkManager: Send + Sync {
    /// Bind a tenant VLAN tag to a specific blade slot's midplane fabric interface.
    async fn provision_vlan_interface(&self, slot: u8, vlan_id: u16) -> Result<(), NetworkError>;

    /// Unbind a tenant VLAN tag from a specific blade slot's midplane fabric.
    async fn teardown_vlan_interface(&self, slot: u8, vlan_id: u16) -> Result<(), NetworkError>;
}
