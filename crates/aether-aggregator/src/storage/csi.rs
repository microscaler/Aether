// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

// Use the csi module from aether_auth
use aether_auth::csi::{
    controller_server::Controller, controller_service_capability, identity_server::Identity,
    list_volumes_response, node_server::Node, node_service_capability, plugin_capability,
    validate_volume_capabilities_response, volume_capability, ControllerExpandVolumeRequest,
    ControllerExpandVolumeResponse, ControllerGetCapabilitiesRequest,
    ControllerGetCapabilitiesResponse, ControllerGetVolumeRequest, ControllerGetVolumeResponse,
    ControllerModifyVolumeRequest, ControllerModifyVolumeResponse, ControllerPublishVolumeRequest,
    ControllerPublishVolumeResponse, ControllerServiceCapability, ControllerUnpublishVolumeRequest,
    ControllerUnpublishVolumeResponse, CreateSnapshotRequest, CreateSnapshotResponse,
    CreateVolumeRequest, CreateVolumeResponse, DeleteSnapshotRequest, DeleteSnapshotResponse,
    DeleteVolumeRequest, DeleteVolumeResponse, GetCapacityRequest, GetCapacityResponse,
    GetPluginCapabilitiesRequest, GetPluginCapabilitiesResponse, GetPluginInfoRequest,
    GetPluginInfoResponse, GetSnapshotRequest, GetSnapshotResponse, ListSnapshotsRequest,
    ListSnapshotsResponse, ListVolumesRequest, ListVolumesResponse, NodeExpandVolumeRequest,
    NodeExpandVolumeResponse, NodeGetCapabilitiesRequest, NodeGetCapabilitiesResponse,
    NodeGetInfoRequest, NodeGetInfoResponse, NodeGetVolumeStatsRequest, NodeGetVolumeStatsResponse,
    NodePublishVolumeRequest, NodePublishVolumeResponse, NodeServiceCapability,
    NodeStageVolumeRequest, NodeStageVolumeResponse, NodeUnpublishVolumeRequest,
    NodeUnpublishVolumeResponse, NodeUnstageVolumeRequest, NodeUnstageVolumeResponse,
    PluginCapability, ProbeRequest, ProbeResponse, ValidateVolumeCapabilitiesRequest,
    ValidateVolumeCapabilitiesResponse, Volume,
};

/// Represents the in-memory state of a provisioned CSI volume.
#[derive(Clone, Debug)]
pub struct VolumeState {
    pub volume_id: String,
    pub name: String,
    pub capacity_bytes: i64,
    pub published_to_nodes: HashSet<String>,
    pub staged: bool,
    pub published: bool,
}

/// Aether CSI Driver implementing standard CSI Identity, Controller, and Node services.
///
/// # Architecture Note (Production vs. Mock)
/// - **Production:** Volume claims are never mounted from the local blade running the VM.
///   Storage nodes provision ZVOLs and export them as iSCSI targets. Compute blades run
///   initiator logins (`open-iscsi`/`iscsiadm`) over the dedicated VLAN 11 Storage Network (MTU 9000),
///   mapping targets locally as `/dev/sdX` which are then attached to VM hypervisor drives.
/// - **Mock / Testing:** Stages and publishes block capabilities as regular files and
///   filesystem capabilities as directories to simulate the lifecycle of initiator-attached
///   block mappings under local development environments.
pub struct AetherCsiDriver {
    pub node_id: String,
    pub volumes: Arc<RwLock<HashMap<String, VolumeState>>>,
    pub name_to_id: Arc<RwLock<HashMap<String, String>>>,
}

impl AetherCsiDriver {
    /// Creates a new instance of AetherCsiDriver.
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            volumes: Arc::new(RwLock::new(HashMap::new())),
            name_to_id: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl Identity for AetherCsiDriver {
    async fn get_plugin_info(
        &self,
        _request: Request<GetPluginInfoRequest>,
    ) -> Result<Response<GetPluginInfoResponse>, Status> {
        Ok(Response::new(GetPluginInfoResponse {
            name: "aether-csi-driver".to_string(),
            vendor_version: "0.1.0".to_string(),
            manifest: HashMap::new(),
        }))
    }

    async fn get_plugin_capabilities(
        &self,
        _request: Request<GetPluginCapabilitiesRequest>,
    ) -> Result<Response<GetPluginCapabilitiesResponse>, Status> {
        let cap = PluginCapability {
            r#type: Some(plugin_capability::Type::Service(
                plugin_capability::Service {
                    r#type: plugin_capability::service::Type::ControllerService as i32,
                },
            )),
        };
        Ok(Response::new(GetPluginCapabilitiesResponse {
            capabilities: vec![cap],
        }))
    }

    async fn probe(
        &self,
        _request: Request<ProbeRequest>,
    ) -> Result<Response<ProbeResponse>, Status> {
        Ok(Response::new(ProbeResponse { ready: Some(true) }))
    }
}

#[tonic::async_trait]
impl Controller for AetherCsiDriver {
    async fn create_volume(
        &self,
        request: Request<CreateVolumeRequest>,
    ) -> Result<Response<CreateVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.name.is_empty() {
            return Err(Status::invalid_argument("Volume name is required"));
        }

        let mut name_to_id = self.name_to_id.write().await;
        let mut volumes = self.volumes.write().await;

        if let Some(vol_id) = name_to_id.get(&req.name) {
            if let Some(vol) = volumes.get(vol_id) {
                // Idempotency check: if requested size is compatible
                let requested_size = req
                    .capacity_range
                    .as_ref()
                    .map(|c| c.required_bytes)
                    .unwrap_or(0);
                if requested_size > 0 && vol.capacity_bytes < requested_size {
                    return Err(Status::already_exists(format!(
                        "Volume {} exists but with smaller size {} than requested {}",
                        req.name, vol.capacity_bytes, requested_size
                    )));
                }
                return Ok(Response::new(CreateVolumeResponse {
                    volume: Some(Volume {
                        capacity_bytes: vol.capacity_bytes,
                        volume_id: vol.volume_id.clone(),
                        volume_context: HashMap::new(),
                        content_source: None,
                        accessible_topology: Vec::new(),
                    }),
                }));
            }
        }

        // Determine size
        let capacity_bytes = if let Some(ref range) = req.capacity_range {
            if range.required_bytes > 0 {
                range.required_bytes
            } else if range.limit_bytes > 0 {
                range.limit_bytes
            } else {
                10 * 1024 * 1024 * 1024 // 10 GiB default
            }
        } else {
            10 * 1024 * 1024 * 1024 // 10 GiB default
        };

        // Generate new volume_id
        let volume_id = format!("vol-{}", uuid::Uuid::new_v4());

        let new_vol = VolumeState {
            volume_id: volume_id.clone(),
            name: req.name.clone(),
            capacity_bytes,
            published_to_nodes: HashSet::new(),
            staged: false,
            published: false,
        };

        log::info!(
            "Created simulated Aether ZVOL structure for volume ID: {} [name: {}, size: {} bytes]",
            volume_id,
            req.name,
            capacity_bytes
        );

        volumes.insert(volume_id.clone(), new_vol);
        name_to_id.insert(req.name, volume_id.clone());

        Ok(Response::new(CreateVolumeResponse {
            volume: Some(Volume {
                capacity_bytes,
                volume_id,
                volume_context: HashMap::new(),
                content_source: None,
                accessible_topology: Vec::new(),
            }),
        }))
    }

    async fn delete_volume(
        &self,
        request: Request<DeleteVolumeRequest>,
    ) -> Result<Response<DeleteVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }

        let mut volumes = self.volumes.write().await;
        let mut name_to_id = self.name_to_id.write().await;

        if let Some(vol) = volumes.remove(&req.volume_id) {
            name_to_id.remove(&vol.name);
            log::info!(
                "Deleted simulated ZVOL structure for volume ID: {}",
                req.volume_id
            );
        }

        Ok(Response::new(DeleteVolumeResponse {}))
    }

    async fn controller_publish_volume(
        &self,
        request: Request<ControllerPublishVolumeRequest>,
    ) -> Result<Response<ControllerPublishVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }
        if req.node_id.is_empty() {
            return Err(Status::invalid_argument("Node ID is required"));
        }

        let mut volumes = self.volumes.write().await;
        if let Some(vol) = volumes.get_mut(&req.volume_id) {
            vol.published_to_nodes.insert(req.node_id.clone());
            log::info!("Volume {} published to node {}", req.volume_id, req.node_id);

            let mut publish_context = HashMap::new();
            publish_context.insert(
                "device_path".to_string(),
                format!("/dev/zvol/tank/{}", req.volume_id),
            );
            return Ok(Response::new(ControllerPublishVolumeResponse {
                publish_context,
            }));
        }

        Err(Status::not_found(format!(
            "Volume {} not found",
            req.volume_id
        )))
    }

    async fn controller_unpublish_volume(
        &self,
        request: Request<ControllerUnpublishVolumeRequest>,
    ) -> Result<Response<ControllerUnpublishVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }

        let mut volumes = self.volumes.write().await;
        if let Some(vol) = volumes.get_mut(&req.volume_id) {
            vol.published_to_nodes.remove(&req.node_id);
            log::info!(
                "Volume {} unpublished from node {}",
                req.volume_id,
                req.node_id
            );
        }

        Ok(Response::new(ControllerUnpublishVolumeResponse {}))
    }

    async fn validate_volume_capabilities(
        &self,
        request: Request<ValidateVolumeCapabilitiesRequest>,
    ) -> Result<Response<ValidateVolumeCapabilitiesResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }

        let volumes = self.volumes.read().await;
        if !volumes.contains_key(&req.volume_id) {
            return Err(Status::not_found(format!(
                "Volume {} not found",
                req.volume_id
            )));
        }

        let confirmed = Some(validate_volume_capabilities_response::Confirmed {
            volume_context: req.volume_context,
            volume_capabilities: req.volume_capabilities,
            parameters: req.parameters,
            mutable_parameters: HashMap::new(),
        });

        Ok(Response::new(ValidateVolumeCapabilitiesResponse {
            confirmed,
            message: String::new(),
        }))
    }

    async fn list_volumes(
        &self,
        _request: Request<ListVolumesRequest>,
    ) -> Result<Response<ListVolumesResponse>, Status> {
        let volumes = self.volumes.read().await;
        let entries = volumes
            .values()
            .map(|vol| list_volumes_response::Entry {
                volume: Some(Volume {
                    capacity_bytes: vol.capacity_bytes,
                    volume_id: vol.volume_id.clone(),
                    volume_context: HashMap::new(),
                    content_source: None,
                    accessible_topology: Vec::new(),
                }),
                status: None,
            })
            .collect();

        Ok(Response::new(ListVolumesResponse {
            entries,
            next_token: String::new(),
        }))
    }

    async fn get_capacity(
        &self,
        _request: Request<GetCapacityRequest>,
    ) -> Result<Response<GetCapacityResponse>, Status> {
        Ok(Response::new(GetCapacityResponse {
            available_capacity: 1_000_000_000_000, // 1 TB dummy
            maximum_volume_size: None,
            minimum_volume_size: None,
        }))
    }

    async fn controller_get_capabilities(
        &self,
        _request: Request<ControllerGetCapabilitiesRequest>,
    ) -> Result<Response<ControllerGetCapabilitiesResponse>, Status> {
        let caps = vec![
            ControllerServiceCapability {
                r#type: Some(controller_service_capability::Type::Rpc(
                    controller_service_capability::Rpc {
                        r#type: controller_service_capability::rpc::Type::CreateDeleteVolume as i32,
                    },
                )),
            },
            ControllerServiceCapability {
                r#type: Some(controller_service_capability::Type::Rpc(
                    controller_service_capability::Rpc {
                        r#type: controller_service_capability::rpc::Type::PublishUnpublishVolume
                            as i32,
                    },
                )),
            },
        ];
        Ok(Response::new(ControllerGetCapabilitiesResponse {
            capabilities: caps,
        }))
    }

    async fn create_snapshot(
        &self,
        _request: Request<CreateSnapshotRequest>,
    ) -> Result<Response<CreateSnapshotResponse>, Status> {
        Err(Status::unimplemented("create_snapshot is not implemented"))
    }

    async fn delete_snapshot(
        &self,
        _request: Request<DeleteSnapshotRequest>,
    ) -> Result<Response<DeleteSnapshotResponse>, Status> {
        Err(Status::unimplemented("delete_snapshot is not implemented"))
    }

    async fn list_snapshots(
        &self,
        _request: Request<ListSnapshotsRequest>,
    ) -> Result<Response<ListSnapshotsResponse>, Status> {
        Err(Status::unimplemented("list_snapshots is not implemented"))
    }

    async fn get_snapshot(
        &self,
        _request: Request<GetSnapshotRequest>,
    ) -> Result<Response<GetSnapshotResponse>, Status> {
        Err(Status::unimplemented("get_snapshot is not implemented"))
    }

    async fn controller_expand_volume(
        &self,
        _request: Request<ControllerExpandVolumeRequest>,
    ) -> Result<Response<ControllerExpandVolumeResponse>, Status> {
        Err(Status::unimplemented(
            "controller_expand_volume is not implemented",
        ))
    }

    async fn controller_get_volume(
        &self,
        _request: Request<ControllerGetVolumeRequest>,
    ) -> Result<Response<ControllerGetVolumeResponse>, Status> {
        Err(Status::unimplemented(
            "controller_get_volume is not implemented",
        ))
    }

    async fn controller_modify_volume(
        &self,
        _request: Request<ControllerModifyVolumeRequest>,
    ) -> Result<Response<ControllerModifyVolumeResponse>, Status> {
        Err(Status::unimplemented(
            "controller_modify_volume is not implemented",
        ))
    }
}

#[tonic::async_trait]
impl Node for AetherCsiDriver {
    async fn node_stage_volume(
        &self,
        request: Request<NodeStageVolumeRequest>,
    ) -> Result<Response<NodeStageVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }
        if req.staging_target_path.is_empty() {
            return Err(Status::invalid_argument("Staging target path is required"));
        }

        let is_block = if let Some(ref cap) = req.volume_capability {
            matches!(
                cap.access_type,
                Some(volume_capability::AccessType::Block(_))
            )
        } else {
            false
        };

        let mut volumes = self.volumes.write().await;
        if let Some(vol) = volumes.get_mut(&req.volume_id) {
            vol.staged = true;
            log::info!(
                "Staged volume {} at {}",
                req.volume_id,
                req.staging_target_path
            );

            let path = std::path::Path::new(&req.staging_target_path);
            if is_block {
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        Status::internal(format!(
                            "Failed to create staging parent directory: {}",
                            e
                        ))
                    })?;
                }
                tokio::fs::File::create(path).await.map_err(|e| {
                    Status::internal(format!("Failed to create staging block file: {}", e))
                })?;
            } else {
                tokio::fs::create_dir_all(path).await.map_err(|e| {
                    Status::internal(format!("Failed to create staging directory: {}", e))
                })?;
            }

            return Ok(Response::new(NodeStageVolumeResponse {}));
        }

        Err(Status::not_found(format!(
            "Volume {} not found",
            req.volume_id
        )))
    }

    async fn node_unstage_volume(
        &self,
        request: Request<NodeUnstageVolumeRequest>,
    ) -> Result<Response<NodeUnstageVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }
        if req.staging_target_path.is_empty() {
            return Err(Status::invalid_argument("Staging target path is required"));
        }

        let mut volumes = self.volumes.write().await;
        if let Some(vol) = volumes.get_mut(&req.volume_id) {
            vol.staged = false;
            log::info!(
                "Unstaged volume {} from {}",
                req.volume_id,
                req.staging_target_path
            );
        }

        let path = std::path::Path::new(&req.staging_target_path);
        if path.exists() {
            if path.is_dir() {
                tokio::fs::remove_dir_all(path).await.map_err(|e| {
                    Status::internal(format!("Failed to remove staging directory: {}", e))
                })?;
            } else {
                tokio::fs::remove_file(path).await.map_err(|e| {
                    Status::internal(format!("Failed to remove staging file: {}", e))
                })?;
            }
        }

        Ok(Response::new(NodeUnstageVolumeResponse {}))
    }

    async fn node_publish_volume(
        &self,
        request: Request<NodePublishVolumeRequest>,
    ) -> Result<Response<NodePublishVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }
        if req.target_path.is_empty() {
            return Err(Status::invalid_argument("Target path is required"));
        }

        let is_block = if let Some(ref cap) = req.volume_capability {
            matches!(
                cap.access_type,
                Some(volume_capability::AccessType::Block(_))
            )
        } else {
            false
        };

        let mut volumes = self.volumes.write().await;
        if let Some(vol) = volumes.get_mut(&req.volume_id) {
            vol.published = true;
            log::info!("Published volume {} to {}", req.volume_id, req.target_path);

            let path = std::path::Path::new(&req.target_path);
            if is_block {
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        Status::internal(format!("Failed to create parent directory: {}", e))
                    })?;
                }
                tokio::fs::File::create(path).await.map_err(|e| {
                    Status::internal(format!("Failed to create published block file: {}", e))
                })?;
            } else {
                tokio::fs::create_dir_all(path).await.map_err(|e| {
                    Status::internal(format!("Failed to create target directory: {}", e))
                })?;
            }

            return Ok(Response::new(NodePublishVolumeResponse {}));
        }

        Err(Status::not_found(format!(
            "Volume {} not found",
            req.volume_id
        )))
    }

    async fn node_unpublish_volume(
        &self,
        request: Request<NodeUnpublishVolumeRequest>,
    ) -> Result<Response<NodeUnpublishVolumeResponse>, Status> {
        let req = request.into_inner();
        if req.volume_id.is_empty() {
            return Err(Status::invalid_argument("Volume ID is required"));
        }
        if req.target_path.is_empty() {
            return Err(Status::invalid_argument("Target path is required"));
        }

        let mut volumes = self.volumes.write().await;
        if let Some(vol) = volumes.get_mut(&req.volume_id) {
            vol.published = false;
            log::info!(
                "Unpublished volume {} from {}",
                req.volume_id,
                req.target_path
            );
        }

        let path = std::path::Path::new(&req.target_path);
        if path.exists() {
            if path.is_dir() {
                tokio::fs::remove_dir_all(path).await.map_err(|e| {
                    Status::internal(format!("Failed to remove target directory: {}", e))
                })?;
            } else {
                tokio::fs::remove_file(path).await.map_err(|e| {
                    Status::internal(format!("Failed to remove target file: {}", e))
                })?;
            }
        }

        Ok(Response::new(NodeUnpublishVolumeResponse {}))
    }

    async fn node_get_volume_stats(
        &self,
        _request: Request<NodeGetVolumeStatsRequest>,
    ) -> Result<Response<NodeGetVolumeStatsResponse>, Status> {
        Err(Status::unimplemented(
            "node_get_volume_stats is not implemented",
        ))
    }

    async fn node_expand_volume(
        &self,
        _request: Request<NodeExpandVolumeRequest>,
    ) -> Result<Response<NodeExpandVolumeResponse>, Status> {
        Err(Status::unimplemented(
            "node_expand_volume is not implemented",
        ))
    }

    async fn node_get_capabilities(
        &self,
        _request: Request<NodeGetCapabilitiesRequest>,
    ) -> Result<Response<NodeGetCapabilitiesResponse>, Status> {
        let cap = NodeServiceCapability {
            r#type: Some(node_service_capability::Type::Rpc(
                node_service_capability::Rpc {
                    r#type: node_service_capability::rpc::Type::StageUnstageVolume as i32,
                },
            )),
        };
        Ok(Response::new(NodeGetCapabilitiesResponse {
            capabilities: vec![cap],
        }))
    }

    async fn node_get_info(
        &self,
        _request: Request<NodeGetInfoRequest>,
    ) -> Result<Response<NodeGetInfoResponse>, Status> {
        Ok(Response::new(NodeGetInfoResponse {
            node_id: self.node_id.clone(),
            max_volumes_per_node: 100,
            accessible_topology: None,
        }))
    }
}
