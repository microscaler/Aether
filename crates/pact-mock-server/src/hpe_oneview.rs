// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use axum::{
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Represents an HPE OneView Ethernet Network resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetNetwork {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub name: String,
    #[serde(rename = "vlanId")]
    pub vlan_id: u16,
    pub uri: String,
    #[serde(rename = "ethernetNetworkType")]
    pub network_type: String,
}

/// Represents a server connection in an HPE OneView Server Profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: u32,
    pub name: String,
    #[serde(rename = "networkUri")]
    pub network_uri: Option<String>,
}

/// Represents an HPE OneView Server Profile resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub name: String,
    pub uri: String,
    #[serde(rename = "serverHardwareUri")]
    pub server_hardware_uri: String,
    pub connections: Vec<Connection>,
}

/// Represents an asynchronous HPE OneView Task resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub uri: String,
    #[serde(rename = "taskState")]
    pub task_state: String,
}

/// Inner state structure tracking sessions, networks, profiles, and tasks.
pub struct InnerState {
    pub sessions: HashSet<String>,
    pub networks: HashMap<String, EthernetNetwork>,
    pub server_profiles: HashMap<String, ServerProfile>,
    pub tasks: HashMap<String, Task>,
}

/// Thread-safe shared state type.
pub type AppState = Arc<RwLock<InnerState>>;

/// Expected JSON login request body.
#[derive(Deserialize)]
pub struct LoginRequest {
    #[serde(rename = "userName")]
    pub user_name: String,
    pub password: Option<String>,
}

/// Handles HPE OneView authentication sessions.
pub async fn login_sessions(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Response {
    if payload.user_name == "admin" && payload.password.as_deref() == Some("password") {
        let session_id = format!("session-token-{}", uuid::Uuid::new_v4());
        state.write().await.sessions.insert(session_id.clone());
        info!(
            "Successful HPE OneView login. Session created: {}",
            session_id
        );
        (StatusCode::OK, Json(json!({ "sessionID": session_id }))).into_response()
    } else {
        warn!(
            "Unauthorized HPE OneView login attempt for user: {}",
            payload.user_name
        );
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "errorCode": "AUTHENTICATION_FAILED",
                "message": "Invalid username or password."
            })),
        )
            .into_response()
    }
}

/// Filter options for querying ethernet networks.
#[derive(Deserialize)]
pub struct NetworkFilter {
    pub filter: Option<String>,
}

/// Lists all configured ethernet networks, supporting filter queries.
pub async fn list_networks(
    State(state): State<AppState>,
    Query(query): Query<NetworkFilter>,
) -> Response {
    let state_read = state.read().await;
    let mut networks: Vec<EthernetNetwork> = state_read.networks.values().cloned().collect();

    if let Some(ref filter_str) = query.filter {
        info!("Applying filter on networks: {}", filter_str);
        let cleaned = filter_str.replace(['"', '\''], "");
        if let Some(stripped) = cleaned.strip_prefix("vlanId=") {
            if let Ok(vlan_val) = stripped.parse::<u16>() {
                networks.retain(|net| net.vlan_id == vlan_val);
            }
        }
    }

    let count = networks.len();
    Json(json!({
        "type": "EthernetNetworkCollection",
        "members": networks,
        "count": count,
        "total": count
    }))
    .into_response()
}

/// Expected JSON network creation body.
#[derive(Deserialize)]
pub struct CreateNetworkRequest {
    pub name: String,
    #[serde(rename = "vlanId")]
    pub vlan_id: u16,
    #[serde(rename = "ethernetNetworkType")]
    pub network_type: Option<String>,
}

/// Dynamically creates a tagged or untagged ethernet network.
pub async fn create_network(
    State(state): State<AppState>,
    Json(payload): Json<CreateNetworkRequest>,
) -> Response {
    let mut state_write = state.write().await;

    if state_write
        .networks
        .values()
        .any(|n| n.vlan_id == payload.vlan_id)
    {
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "errorCode": "DUPLICATE_RESOURCE",
                "message": format!("Network with VLAN ID {} already exists.", payload.vlan_id)
            })),
        )
            .into_response();
    }

    let network_id = format!("vlan-{}", payload.vlan_id);
    let uri = format!("/rest/ethernet-networks/{}", network_id);
    let new_network = EthernetNetwork {
        resource_type: "ethernet-networkV4".to_string(),
        name: payload.name,
        vlan_id: payload.vlan_id,
        uri: uri.clone(),
        network_type: payload.network_type.unwrap_or_else(|| "Tagged".to_string()),
    };

    state_write.networks.insert(uri, new_network.clone());
    info!(
        "Created network: {} (VLAN {})",
        new_network.name, new_network.vlan_id
    );
    (StatusCode::CREATED, Json(new_network)).into_response()
}

/// Lists all server profiles.
pub async fn list_server_profiles(State(state): State<AppState>) -> Response {
    let state_read = state.read().await;
    let profiles: Vec<ServerProfile> = state_read.server_profiles.values().cloned().collect();
    let count = profiles.len();
    Json(json!({
        "type": "ServerProfileCollection",
        "members": profiles,
        "count": count,
        "total": count
    }))
    .into_response()
}

/// Returns a specific server profile by its ID.
pub async fn get_server_profile(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let state_read = state.read().await;
    let uri = format!("/rest/server-profiles/{}", id);
    if let Some(profile) = state_read.server_profiles.get(&uri) {
        Json(profile).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "errorCode": "RESOURCE_NOT_FOUND",
                "message": format!("Server profile not found: {}", uri)
            })),
        )
            .into_response()
    }
}

/// Updates a server profile connection and spawns an asynchronous Task.
pub async fn update_server_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ServerProfile>,
) -> Response {
    let mut state_write = state.write().await;
    let uri = format!("/rest/server-profiles/{}", id);
    if state_write.server_profiles.contains_key(&uri) {
        state_write
            .server_profiles
            .insert(uri.clone(), payload.clone());
        info!("Updated server profile: {}", uri);

        let task_id = format!("task-{}", uuid::Uuid::new_v4());
        let task_uri = format!("/rest/tasks/{}", task_id);
        let task = Task {
            resource_type: "Task".to_string(),
            uri: task_uri.clone(),
            task_state: "Completed".to_string(),
        };

        state_write.tasks.insert(task_uri.clone(), task.clone());
        (StatusCode::ACCEPTED, Json(task)).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "errorCode": "RESOURCE_NOT_FOUND",
                "message": format!("Server profile not found: {}", uri)
            })),
        )
            .into_response()
    }
}

/// Returns the status of an asynchronous Task.
pub async fn get_task(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let state_read = state.read().await;
    let uri = format!("/rest/tasks/{}", id);
    if let Some(task) = state_read.tasks.get(&uri) {
        Json(task).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "errorCode": "RESOURCE_NOT_FOUND",
                "message": format!("Task not found: {}", uri)
            })),
        )
            .into_response()
    }
}

/// Middleware verifying the 'auth' session header specifically for OneView routes.
pub async fn hpe_oneview_auth_middleware(
    State(state): State<AppState>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/health" || path == "/" || path == "/rest/login-sessions" {
        return next.run(request).await;
    }

    if let Some(auth_header) = request.headers().get("auth") {
        if let Ok(token) = auth_header.to_str() {
            let state_read = state.read().await;
            if state_read.sessions.contains(token) {
                drop(state_read);
                return next.run(request).await;
            }
        }
    }

    warn!(
        "Unauthorized: missing or invalid 'auth' header on path: {}",
        path
    );
    let mut response = (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "errorCode": "UNAUTHORIZED",
            "message": "The session token is invalid or has expired."
        })),
    )
        .into_response();

    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/json"));

    response
}

/// Builds and returns the HPE OneView Router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/rest/login-sessions", post(login_sessions))
        .route(
            "/rest/ethernet-networks",
            get(list_networks).post(create_network),
        )
        .route("/rest/server-profiles", get(list_server_profiles))
        .route(
            "/rest/server-profiles/:id",
            get(get_server_profile).put(update_server_profile),
        )
        .route("/rest/tasks/:id", get(get_task))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            hpe_oneview_auth_middleware,
        ))
        .with_state(state)
}
