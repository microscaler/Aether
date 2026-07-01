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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
#[allow(clippy::needless_update)]
mod tests {
    use super::*;
    use aether_auth::csi::{
        volume_capability, CapacityRange, ControllerExpandVolumeRequest,
        ControllerGetCapabilitiesRequest, ControllerGetVolumeRequest,
        ControllerModifyVolumeRequest, ControllerPublishVolumeRequest,
        ControllerUnpublishVolumeRequest, CreateSnapshotRequest, CreateVolumeRequest,
        DeleteSnapshotRequest, DeleteVolumeRequest, GetCapacityRequest, GetSnapshotRequest,
        ListSnapshotsRequest, ListVolumesRequest, NodeGetCapabilitiesRequest, NodeGetInfoRequest,
        NodePublishVolumeRequest, NodeStageVolumeRequest, NodeUnpublishVolumeRequest,
        NodeUnstageVolumeRequest, ValidateVolumeCapabilitiesRequest, VolumeCapability,
    };
    use std::path::Path;
    use tonic::Code;

    fn make_driver() -> AetherCsiDriver {
        AetherCsiDriver::new("test-node".to_string())
    }

    // ====== create_volume error paths ======

    #[tokio::test]
    async fn test_create_volume_empty_name() {
        let driver = make_driver();
        let res = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_create_volume_default_capacity() {
        let driver = make_driver();
        let res = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "default-cap-vol".to_string(),
                capacity_range: None,
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let vol = res.volume.unwrap();
        // Should get 10 GiB default (line 160)
        assert_eq!(vol.capacity_bytes, 10 * 1024 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_create_volume_limit_bytes_fallback() {
        let driver = make_driver();
        let res = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "limit-fallback-vol".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 0,
                    limit_bytes: 2 * 1024 * 1024 * 1024,
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let vol = res.volume.unwrap();
        // Should get limit_bytes when required_bytes is 0 (lines 154-155)
        assert_eq!(vol.capacity_bytes, 2 * 1024 * 1024 * 1024);
    }

    // ====== delete_volume error paths ======

    #[tokio::test]
    async fn test_delete_volume_empty_id() {
        let driver = make_driver();
        let res = Controller::delete_volume(
            &driver,
            Request::new(DeleteVolumeRequest {
                volume_id: "".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_delete_volume_nonexistent() {
        let driver = make_driver();
        // Should succeed even for non-existent volume (silently no-op)
        let res = Controller::delete_volume(
            &driver,
            Request::new(DeleteVolumeRequest {
                volume_id: "vol-does-not-exist".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_ok());
    }

    // ====== controller_publish_volume error paths ======

    #[tokio::test]
    async fn test_controller_publish_empty_volume_id() {
        let driver = make_driver();
        let res = Controller::controller_publish_volume(
            &driver,
            Request::new(ControllerPublishVolumeRequest {
                volume_id: "".to_string(),
                node_id: "node-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_controller_publish_empty_node_id() {
        let driver = make_driver();
        let res = Controller::controller_publish_volume(
            &driver,
            Request::new(ControllerPublishVolumeRequest {
                volume_id: "vol-123".to_string(),
                node_id: "".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_controller_publish_not_found() {
        let driver = make_driver();
        let res = Controller::controller_publish_volume(
            &driver,
            Request::new(ControllerPublishVolumeRequest {
                volume_id: "nonexistent-vol".to_string(),
                node_id: "node-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::NotFound);
    }

    // ====== controller_unpublish_volume error paths ======

    #[tokio::test]
    async fn test_controller_unpublish_empty_volume_id() {
        let driver = make_driver();
        let res = Controller::controller_unpublish_volume(
            &driver,
            Request::new(ControllerUnpublishVolumeRequest {
                volume_id: "".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    // ====== validate_volume_capabilities error paths ======

    #[tokio::test]
    async fn test_validate_empty_volume_id() {
        let driver = make_driver();
        let res = Controller::validate_volume_capabilities(
            &driver,
            Request::new(ValidateVolumeCapabilitiesRequest {
                volume_id: "".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_validate_not_found() {
        let driver = make_driver();
        let res = Controller::validate_volume_capabilities(
            &driver,
            Request::new(ValidateVolumeCapabilitiesRequest {
                volume_id: "nonexistent".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::NotFound);
    }

    // ====== controller_get_capabilities ======

    #[tokio::test]
    async fn test_controller_get_capabilities() {
        let driver = make_driver();
        let res = Controller::controller_get_capabilities(
            &driver,
            Request::new(ControllerGetCapabilitiesRequest {}),
        )
        .await
        .unwrap()
        .into_inner();
        assert!(!res.capabilities.is_empty());
    }

    // ====== Snapshot methods (unimplemented) ======

    #[tokio::test]
    async fn test_create_snapshot_unimplemented() {
        let driver = make_driver();
        let res = Controller::create_snapshot(
            &driver,
            Request::new(CreateSnapshotRequest {
                name: "snap-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_delete_snapshot_unimplemented() {
        let driver = make_driver();
        let res = Controller::delete_snapshot(
            &driver,
            Request::new(DeleteSnapshotRequest {
                snapshot_id: "snap-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_list_snapshots_unimplemented() {
        let driver = make_driver();
        let res = Controller::list_snapshots(
            &driver,
            Request::new(ListSnapshotsRequest {
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_get_snapshot_unimplemented() {
        let driver = make_driver();
        let res = Controller::get_snapshot(
            &driver,
            Request::new(GetSnapshotRequest {
                snapshot_id: "snap-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
    }

    // ====== Controller expand/modify/volume (unimplemented) ======

    #[tokio::test]
    async fn test_controller_expand_unimplemented() {
        let driver = make_driver();
        let res = Controller::controller_expand_volume(
            &driver,
            Request::new(ControllerExpandVolumeRequest {
                volume_id: "vol-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_controller_get_volume_unimplemented() {
        let driver = make_driver();
        let res = Controller::controller_get_volume(
            &driver,
            Request::new(ControllerGetVolumeRequest {
                volume_id: "vol-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
    }

    #[tokio::test]
    async fn test_controller_modify_unimplemented() {
        let driver = make_driver();
        let res = Controller::controller_modify_volume(
            &driver,
            Request::new(ControllerModifyVolumeRequest {
                volume_id: "vol-1".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::Unimplemented);
    }

    // ====== Node service error paths ======

    #[tokio::test]
    async fn test_node_stage_empty_volume_id() {
        let driver = make_driver();
        let res = Node::node_stage_volume(
            &driver,
            Request::new(NodeStageVolumeRequest {
                volume_id: "".to_string(),
                staging_target_path: "/tmp/stage".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_node_stage_empty_staging_path() {
        let driver = make_driver();
        let res = Node::node_stage_volume(
            &driver,
            Request::new(NodeStageVolumeRequest {
                volume_id: "vol-1".to_string(),
                staging_target_path: "".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_node_stage_not_found() {
        let driver = make_driver();
        let temp_path = std::env::temp_dir().join("csi-test-stage-not-found");
        let stage_path = temp_path.to_string_lossy().to_string();
        let res = Node::node_stage_volume(
            &driver,
            Request::new(NodeStageVolumeRequest {
                volume_id: "nonexistent".to_string(),
                staging_target_path: stage_path,
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::NotFound);
    }

    #[tokio::test]
    async fn test_node_unstage_empty_volume_id() {
        let driver = make_driver();
        let res = Node::node_unstage_volume(
            &driver,
            Request::new(NodeUnstageVolumeRequest {
                volume_id: "".to_string(),
                staging_target_path: "/tmp/stage".to_string(),
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_node_unstage_empty_path() {
        let driver = make_driver();
        let res = Node::node_unstage_volume(
            &driver,
            Request::new(NodeUnstageVolumeRequest {
                volume_id: "vol-1".to_string(),
                staging_target_path: "".to_string(),
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_node_publish_empty_volume_id() {
        let driver = make_driver();
        let res = Node::node_publish_volume(
            &driver,
            Request::new(NodePublishVolumeRequest {
                volume_id: "".to_string(),
                target_path: "/tmp/publish".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_node_publish_empty_target_path() {
        let driver = make_driver();
        let res = Node::node_publish_volume(
            &driver,
            Request::new(NodePublishVolumeRequest {
                volume_id: "vol-1".to_string(),
                target_path: "".to_string(),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_node_publish_not_found() {
        let driver = make_driver();
        let temp_path = std::env::temp_dir().join("csi-test-publish-not-found");
        let target_path = temp_path.to_string_lossy().to_string();
        let res = Node::node_publish_volume(
            &driver,
            Request::new(NodePublishVolumeRequest {
                volume_id: "nonexistent".to_string(),
                target_path,
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::NotFound);
    }

    // ====== Node unpublish error paths ======

    #[tokio::test]
    async fn test_node_unpublish_empty_volume_id() {
        let driver = make_driver();
        let res = Node::node_unpublish_volume(
            &driver,
            Request::new(NodeUnpublishVolumeRequest {
                volume_id: "".to_string(),
                target_path: "/tmp/publish".to_string(),
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_node_unpublish_empty_target_path() {
        let driver = make_driver();
        let res = Node::node_unpublish_volume(
            &driver,
            Request::new(NodeUnpublishVolumeRequest {
                volume_id: "vol-1".to_string(),
                target_path: "".to_string(),
            }),
        )
        .await;
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().code(), Code::InvalidArgument);
    }

    // ====== Node get capabilities ======

    #[tokio::test]
    async fn test_node_get_capabilities() {
        let driver = make_driver();
        let res = Node::node_get_capabilities(&driver, Request::new(NodeGetCapabilitiesRequest {}))
            .await
            .unwrap()
            .into_inner();
        assert!(!res.capabilities.is_empty());
    }

    // ====== Node get info ======

    #[tokio::test]
    async fn test_node_get_info() {
        let driver = make_driver();
        let res = Node::node_get_info(&driver, Request::new(NodeGetInfoRequest {}))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(res.node_id, "test-node");
        assert_eq!(res.max_volumes_per_node, 100);
    }

    // ====== Block staging/publishing ======

    #[tokio::test]
    async fn test_node_stage_volume_block_cap() {
        let driver = make_driver();

        // Create volume first
        let create_res = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "block-test".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 1024 * 1024 * 1024,
                    limit_bytes: 1024 * 1024 * 1024,
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let volume_id = create_res.volume.unwrap().volume_id;

        let temp_path = std::env::temp_dir().join("csi-test-block-stage");
        let stage_path = temp_path.to_string_lossy().to_string();

        // Stage with block capability
        let block_cap = VolumeCapability {
            access_type: Some(volume_capability::AccessType::Block(
                volume_capability::BlockVolume {},
            )),
            access_mode: Some(volume_capability::AccessMode {
                mode: volume_capability::access_mode::Mode::SingleNodeWriter as i32,
            }),
        };
        let res = Node::node_stage_volume(
            &driver,
            Request::new(NodeStageVolumeRequest {
                volume_id: volume_id.clone(),
                staging_target_path: stage_path.clone(),
                volume_capability: Some(block_cap),
                ..Default::default()
            }),
        )
        .await;
        assert!(res.is_ok());
        assert!(Path::new(&stage_path).exists());

        // Cleanup
        let _ = std::fs::remove_file(&stage_path);
    }

    // ====== publish_context from controller publish ======

    #[tokio::test]
    async fn test_controller_publish_returns_device_path() {
        let driver = make_driver();

        // Create volume
        let create_res = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "publish-test".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 1024 * 1024 * 1024,
                    limit_bytes: 1024 * 1024 * 1024,
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let volume_id = create_res.volume.unwrap().volume_id;

        let pub_res = Controller::controller_publish_volume(
            &driver,
            Request::new(ControllerPublishVolumeRequest {
                volume_id: volume_id.clone(),
                node_id: "test-node".to_string(),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();

        // Should contain device_path in publish_context
        assert!(pub_res.publish_context.contains_key("device_path"));
    }

    // ====== list_volumes ======

    #[tokio::test]
    async fn test_list_volumes_empty() {
        let driver = make_driver();
        let res = Controller::list_volumes(
            &driver,
            Request::new(ListVolumesRequest {
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        assert!(res.entries.is_empty());
    }

    #[tokio::test]
    async fn test_list_volumes_shows_created() {
        let driver = make_driver();

        Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "list-test".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 1024 * 1024 * 1024,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap();

        let res = Controller::list_volumes(
            &driver,
            Request::new(ListVolumesRequest {
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        assert_eq!(res.entries.len(), 1);
    }

    // ====== get_capacity ======

    #[tokio::test]
    async fn test_get_capacity() {
        let driver = make_driver();
        let res = Controller::get_capacity(
            &driver,
            Request::new(GetCapacityRequest {
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        assert!(res.available_capacity > 0);
    }

    // ====== Node unpublish cleanup ======

    #[tokio::test]
    async fn test_node_unpublish_removes_path() {
        let driver = make_driver();

        // Create volume
        let create_res = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "unpublish-test".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 1024 * 1024 * 1024,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let volume_id = create_res.volume.unwrap().volume_id;

        let temp_path = std::env::temp_dir().join("csi-test-unpublish");
        let target_path = temp_path.to_string_lossy().to_string();

        // Publish to create the path
        Node::node_publish_volume(
            &driver,
            Request::new(NodePublishVolumeRequest {
                volume_id: volume_id.clone(),
                target_path: target_path.clone(),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
        assert!(Path::new(&target_path).exists());

        // Unpublish should remove the path
        Node::node_unpublish_volume(
            &driver,
            Request::new(NodeUnpublishVolumeRequest {
                volume_id: volume_id.clone(),
                target_path: target_path.clone(),
            }),
        )
        .await
        .unwrap();
        assert!(!Path::new(&target_path).exists());
    }

    // ====== Idempotent create with compatible size ======

    #[tokio::test]
    async fn test_create_volume_same_name_compatible_size() {
        let driver = make_driver();

        // Create with 10 GiB
        let create1 = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "idempotent-test".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 10 * 1024 * 1024 * 1024,
                    limit_bytes: 10 * 1024 * 1024 * 1024,
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();

        // Create same name with smaller required_bytes -> should return existing
        let create2 = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "idempotent-test".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 5 * 1024 * 1024 * 1024,
                    limit_bytes: 5 * 1024 * 1024 * 1024,
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();

        assert_eq!(
            create1.volume.as_ref().unwrap().volume_id,
            create2.volume.as_ref().unwrap().volume_id
        );
    }

    // ====== validate_volume_capabilities with valid volume ======

    #[tokio::test]
    async fn test_validate_volume_capabilities_valid() {
        let driver = make_driver();

        // Create a volume
        let create_res = Controller::create_volume(
            &driver,
            Request::new(CreateVolumeRequest {
                name: "validate-test".to_string(),
                capacity_range: Some(CapacityRange {
                    required_bytes: 1024 * 1024 * 1024,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();
        let volume_id = create_res.volume.unwrap().volume_id;

        let res = Controller::validate_volume_capabilities(
            &driver,
            Request::new(ValidateVolumeCapabilitiesRequest {
                volume_id: volume_id.clone(),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_inner();

        assert!(res.confirmed.is_some());
    }
}
