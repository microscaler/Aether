use crate::network::{MidplaneNetworkManager, NetworkError};
use async_trait::async_trait;
use log::{error, info, warn};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EthernetNetwork {
    name: String,
    #[serde(rename = "vlanId")]
    vlan_id: u16,
    uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EthernetNetworkCollection {
    members: Vec<EthernetNetwork>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Connection {
    id: u32,
    name: String,
    #[serde(rename = "networkUri")]
    network_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerProfile {
    #[serde(rename = "type")]
    resource_type: String,
    name: String,
    uri: String,
    #[serde(rename = "serverHardwareUri")]
    server_hardware_uri: String,
    connections: Vec<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    uri: String,
    #[serde(rename = "taskState")]
    task_state: String,
}

pub struct VirtualConnectClient {
    client: Client,
    endpoint: String,
    username: String,
    password: String,
    api_version: String,
    session_token: Mutex<Option<String>>,
    /// Polling interval for asynchronous tasks (defaults to 5 seconds)
    pub poll_interval: Duration,
    /// Number of attempts for polling asynchronous tasks (defaults to 60)
    pub poll_attempts: usize,
}

impl VirtualConnectClient {
    pub fn new(endpoint: String, username: String, password: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            endpoint,
            username,
            password,
            api_version: "600".to_string(),
            session_token: Mutex::new(None),
            poll_interval: Duration::from_secs(5),
            poll_attempts: 60,
        }
    }

    /// Pre-seeds a session token for testing token refresh scenarios.
    pub async fn preseed_session_token(&self, token: String) {
        let mut token_guard = self.session_token.lock().await;
        *token_guard = Some(token);
    }

    async fn get_token(&self) -> Result<String, NetworkError> {
        let mut token_guard = self.session_token.lock().await;
        if let Some(ref token) = *token_guard {
            return Ok(token.clone());
        }

        info!(
            "Authenticating with HPE OneView REST API at {}",
            self.endpoint
        );
        let url = format!("{}/rest/login-sessions", self.endpoint);
        let login_body = json!({
            "userName": self.username,
            "password": self.password,
            "authLoginDomain": "Local"
        });

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .header("X-API-Version", &self.api_version)
            .json(&login_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status_code = response.status();
            let err_text = response.text().await.unwrap_or_default();
            error!("Login failed with status {}: {}", status_code, err_text);
            return Err(NetworkError::Authentication(format!(
                "Login returned status {}: {}",
                status_code, err_text
            )));
        }

        #[derive(Deserialize)]
        struct LoginResponse {
            #[serde(rename = "sessionID")]
            session_id: String,
        }

        let login_resp: LoginResponse = response.json().await?;
        *token_guard = Some(login_resp.session_id.clone());
        Ok(login_resp.session_id)
    }

    async fn send_request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<Response, NetworkError> {
        let mut retries = 1;
        loop {
            let token = self.get_token().await?;
            let url = format!("{}{}", self.endpoint, path);

            let mut req = self
                .client
                .request(method.clone(), &url)
                .header("auth", &token)
                .header("X-API-Version", &self.api_version)
                .header("content-type", "application/json");

            if let Some(ref b) = body {
                req = req.json(b);
            }

            let response = req.send().await?;
            if response.status() == StatusCode::UNAUTHORIZED && retries > 0 {
                warn!("Session token expired or invalid, clearing cache and retrying...");
                let mut token_guard = self.session_token.lock().await;
                *token_guard = None;
                retries -= 1;
                continue;
            }

            return Ok(response);
        }
    }

    async fn poll_task(&self, task_uri: &str) -> Result<(), NetworkError> {
        let max_attempts = self.poll_attempts;
        let mut interval = tokio::time::interval(self.poll_interval);

        for attempt in 1..=max_attempts {
            interval.tick().await;
            info!(
                "Polling task {} (attempt {}/{})",
                task_uri, attempt, max_attempts
            );

            let response = self
                .send_request(reqwest::Method::GET, task_uri, None)
                .await?;
            if !response.status().is_success() {
                return Err(NetworkError::Other(format!(
                    "Failed to fetch task status. Status: {}",
                    response.status()
                )));
            }

            let task: Task = response.json().await?;
            match task.task_state.as_str() {
                "Completed" => {
                    info!("Task {} completed successfully", task_uri);
                    return Ok(());
                }
                "Error" | "Failed" | "Terminated" => {
                    error!("Task {} failed with state: {}", task_uri, task.task_state);
                    return Err(NetworkError::Other(format!(
                        "OneView task failed: {}",
                        task.task_state
                    )));
                }
                other => {
                    info!("Task {} is still in state: {}", task_uri, other);
                }
            }
        }

        Err(NetworkError::Other(format!(
            "Timeout waiting for OneView task {} to complete",
            task_uri
        )))
    }

    async fn get_or_create_network(&self, vlan_id: u16) -> Result<String, NetworkError> {
        // 1. Search for network with matching vlan_id
        let filter_val = format!("vlanId={}", vlan_id);
        let path = format!(
            "/rest/ethernet-networks?filter={}",
            urlencoding::encode(&filter_val)
        );

        let response = self.send_request(reqwest::Method::GET, &path, None).await?;
        if response.status().is_success() {
            let collection: EthernetNetworkCollection = response.json().await?;
            if let Some(net) = collection.members.first() {
                info!("Found existing network for VLAN {}: {}", vlan_id, net.uri);
                return Ok(net.uri.clone());
            }
        }

        // 2. Create network if not found
        info!("Network for VLAN {} not found. Creating it...", vlan_id);
        let create_path = "/rest/ethernet-networks";
        let create_body = json!({
            "name": format!("Tenant-VLAN-{}", vlan_id),
            "vlanId": vlan_id,
            "ethernetNetworkType": "Tagged",
            "type": "ethernet-networkV4"
        });

        let response = self
            .send_request(reqwest::Method::POST, create_path, Some(create_body))
            .await?;

        if !response.status().is_success() {
            return Err(NetworkError::Api {
                code: response.status().to_string(),
                message: "Failed to create ethernet network".to_string(),
            });
        }

        let created_net: EthernetNetwork = response.json().await?;
        info!(
            "Successfully created network for VLAN {}: {}",
            vlan_id, created_net.uri
        );
        Ok(created_net.uri)
    }
}

#[async_trait]
impl MidplaneNetworkManager for VirtualConnectClient {
    async fn provision_vlan_interface(&self, slot: u8, vlan_id: u16) -> Result<(), NetworkError> {
        info!("Provisioning VLAN {} for slot {}", vlan_id, slot);

        // 1. Get or create the VLAN network URI
        let network_uri = self.get_or_create_network(vlan_id).await?;

        // 2. Fetch the server profile
        let profile_path = format!("/rest/server-profiles/profile-slot-{}", slot);
        let response = self
            .send_request(reqwest::Method::GET, &profile_path, None)
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(NetworkError::NotFound(format!(
                "Server profile for slot {} not found at {}",
                slot, profile_path
            )));
        }
        if !response.status().is_success() {
            return Err(NetworkError::Api {
                code: response.status().to_string(),
                message: "Failed to fetch server profile".to_string(),
            });
        }

        let mut profile: ServerProfile = response.json().await?;

        // 3. Update connection network association
        let mut modified = false;
        for conn in &mut profile.connections {
            if conn.name == "FlexNIC-1a" || conn.id == 1 {
                conn.network_uri = Some(network_uri.clone());
                modified = true;
                break;
            }
        }

        if !modified {
            return Err(NetworkError::Other(format!(
                "Could not find primary connection (FlexNIC-1a) in server profile for slot {}",
                slot
            )));
        }

        // 4. PUT updated server profile back to OneView
        let response = self
            .send_request(reqwest::Method::PUT, &profile_path, Some(json!(profile)))
            .await?;

        if response.status() == StatusCode::ACCEPTED {
            let task: Task = response.json().await?;
            self.poll_task(&task.uri).await?;
        } else if !response.status().is_success() {
            return Err(NetworkError::Api {
                code: response.status().to_string(),
                message: "Failed to update server profile".to_string(),
            });
        }

        info!("Successfully provisioned VLAN {} on slot {}", vlan_id, slot);
        Ok(())
    }

    async fn teardown_vlan_interface(&self, slot: u8, _vlan_id: u16) -> Result<(), NetworkError> {
        info!("Tearing down VLAN assignment on slot {}", slot);

        // 1. Fetch the server profile
        let profile_path = format!("/rest/server-profiles/profile-slot-{}", slot);
        let response = self
            .send_request(reqwest::Method::GET, &profile_path, None)
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(NetworkError::NotFound(format!(
                "Server profile for slot {} not found at {}",
                slot, profile_path
            )));
        }
        if !response.status().is_success() {
            return Err(NetworkError::Api {
                code: response.status().to_string(),
                message: "Failed to fetch server profile".to_string(),
            });
        }

        let mut profile: ServerProfile = response.json().await?;

        // 2. Clear network assignment
        let mut modified = false;
        for conn in &mut profile.connections {
            if conn.name == "FlexNIC-1a" || conn.id == 1 {
                if conn.network_uri.is_some() {
                    conn.network_uri = None;
                    modified = true;
                }
                break;
            }
        }

        if !modified {
            info!(
                "Primary connection on slot {} is already cleared. Nothing to teardown.",
                slot
            );
            return Ok(());
        }

        // 3. PUT updated server profile back
        let response = self
            .send_request(reqwest::Method::PUT, &profile_path, Some(json!(profile)))
            .await?;

        if response.status() == StatusCode::ACCEPTED {
            let task: Task = response.json().await?;
            self.poll_task(&task.uri).await?;
        } else if !response.status().is_success() {
            return Err(NetworkError::Api {
                code: response.status().to_string(),
                message: "Failed to update server profile".to_string(),
            });
        }

        info!("Successfully tore down VLAN assignment on slot {}", slot);
        Ok(())
    }
}
