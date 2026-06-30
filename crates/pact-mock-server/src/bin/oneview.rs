// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use axum::{routing::get, Router};
use pact_mock_server::hpe_oneview::{self, Connection, InnerState, ServerProfile};
use pact_mock_server::{health_check, logging_middleware};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Seed server profiles for slot 1 to 8
    let mut server_profiles = HashMap::new();
    for slot in 1..=8 {
        let uri = format!("/rest/server-profiles/profile-slot-{}", slot);
        let profile = ServerProfile {
            resource_type: "ServerProfileV12".to_string(),
            name: format!("Blade-Profile-Slot-{}", slot),
            uri: uri.clone(),
            server_hardware_uri: format!("/rest/server-hardware/slot-{}", slot),
            connections: vec![
                Connection {
                    id: 1,
                    name: "FlexNIC-1a".to_string(),
                    network_uri: None,
                },
                Connection {
                    id: 2,
                    name: "FlexNIC-1b".to_string(),
                    network_uri: None,
                },
            ],
        };
        server_profiles.insert(uri, profile);
    }

    let state = Arc::new(RwLock::new(InnerState {
        sessions: HashSet::new(),
        networks: HashMap::new(),
        server_profiles,
        tasks: HashMap::new(),
    }));

    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .merge(hpe_oneview::router(state))
        .layer(axum::middleware::from_fn(logging_middleware));

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting HPE OneView Mock Server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
