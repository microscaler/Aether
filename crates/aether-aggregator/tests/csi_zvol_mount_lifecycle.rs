// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;
use std::path::Path;
use tonic::Request;

use aether_aggregator::storage::csi::AetherCsiDriver;
use aether_auth::csi::{
    controller_server::Controller, identity_server::Identity, node_server::Node, CapacityRange,
    ControllerPublishVolumeRequest, ControllerUnpublishVolumeRequest, CreateVolumeRequest,
    DeleteVolumeRequest, GetCapacityRequest, GetPluginCapabilitiesRequest, GetPluginInfoRequest,
    ListVolumesRequest, NodeGetCapabilitiesRequest, NodeGetInfoRequest, NodePublishVolumeRequest,
    NodeStageVolumeRequest, NodeUnpublishVolumeRequest, NodeUnstageVolumeRequest, ProbeRequest,
    ValidateVolumeCapabilitiesRequest,
};

#[tokio::test]
async fn test_csi_zvol_mount_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let driver = AetherCsiDriver::new("node-01".to_string());

    // ==========================================
    // 1. Verify Identity Service
    // ==========================================
    let info_res = Identity::get_plugin_info(&driver, Request::new(GetPluginInfoRequest {}))
        .await?
        .into_inner();
    assert_eq!(info_res.name, "aether-csi-driver");
    assert_eq!(info_res.vendor_version, "0.1.0");

    let cap_res =
        Identity::get_plugin_capabilities(&driver, Request::new(GetPluginCapabilitiesRequest {}))
            .await?
            .into_inner();
    assert_eq!(cap_res.capabilities.len(), 1);

    let probe_res = Identity::probe(&driver, Request::new(ProbeRequest {}))
        .await?
        .into_inner();
    assert_eq!(probe_res.ready, Some(true));

    // ==========================================
    // 2. Verify Controller Service (Volume Create/Idempotency/Capabilities)
    // ==========================================
    let vol_name = "test-csi-volume".to_string();
    let create_res = Controller::create_volume(
        &driver,
        Request::new(CreateVolumeRequest {
            name: vol_name.clone(),
            capacity_range: Some(CapacityRange {
                required_bytes: 5 * 1024 * 1024 * 1024, // 5 GiB
                limit_bytes: 5 * 1024 * 1024 * 1024,
            }),
            volume_capabilities: Vec::new(),
            parameters: HashMap::new(),
            secrets: HashMap::new(),
            volume_content_source: None,
            accessibility_requirements: None,
            mutable_parameters: HashMap::new(),
        }),
    )
    .await?
    .into_inner();

    let volume_opt = create_res.volume;
    assert!(volume_opt.is_some());
    let volume = volume_opt.ok_or("Volume field missing in CreateVolumeResponse")?;
    assert_eq!(volume.capacity_bytes, 5 * 1024 * 1024 * 1024);
    let volume_id = volume.volume_id;

    // Test Idempotency: creating same volume with same size should succeed and return same ID
    let create_again_res = Controller::create_volume(
        &driver,
        Request::new(CreateVolumeRequest {
            name: vol_name.clone(),
            capacity_range: Some(CapacityRange {
                required_bytes: 5 * 1024 * 1024 * 1024,
                limit_bytes: 5 * 1024 * 1024 * 1024,
            }),
            volume_capabilities: Vec::new(),
            parameters: HashMap::new(),
            secrets: HashMap::new(),
            volume_content_source: None,
            accessibility_requirements: None,
            mutable_parameters: HashMap::new(),
        }),
    )
    .await?
    .into_inner();
    let volume_again = create_again_res
        .volume
        .ok_or("Volume field missing in CreateVolumeResponse (idempotent)")?;
    assert_eq!(volume_again.volume_id, volume_id);

    // Test Idempotency failure: creating same volume with larger size should return ALREADY_EXISTS
    let create_larger_res = Controller::create_volume(
        &driver,
        Request::new(CreateVolumeRequest {
            name: vol_name.clone(),
            capacity_range: Some(CapacityRange {
                required_bytes: 10 * 1024 * 1024 * 1024, // 10 GiB (larger)
                limit_bytes: 10 * 1024 * 1024 * 1024,
            }),
            volume_capabilities: Vec::new(),
            parameters: HashMap::new(),
            secrets: HashMap::new(),
            volume_content_source: None,
            accessibility_requirements: None,
            mutable_parameters: HashMap::new(),
        }),
    )
    .await;
    assert!(create_larger_res.is_err());
    let err_status = create_larger_res.err().ok_or("Expected Err but got Ok")?;
    assert_eq!(err_status.code(), tonic::Code::AlreadyExists);

    // Verify list volumes shows the created volume
    let list_res = Controller::list_volumes(
        &driver,
        Request::new(ListVolumesRequest {
            max_entries: 10,
            starting_token: String::new(),
        }),
    )
    .await?
    .into_inner();
    assert_eq!(list_res.entries.len(), 1);

    // Verify capacity check
    let capacity_res = Controller::get_capacity(
        &driver,
        Request::new(GetCapacityRequest {
            volume_capabilities: Vec::new(),
            parameters: HashMap::new(),
            accessible_topology: None,
        }),
    )
    .await?
    .into_inner();
    assert!(capacity_res.available_capacity > 0);

    // Validate volume capabilities
    let validate_res = Controller::validate_volume_capabilities(
        &driver,
        Request::new(ValidateVolumeCapabilitiesRequest {
            volume_id: volume_id.clone(),
            volume_context: HashMap::new(),
            volume_capabilities: Vec::new(),
            parameters: HashMap::new(),
            secrets: HashMap::new(),
            mutable_parameters: HashMap::new(),
        }),
    )
    .await?
    .into_inner();
    assert!(validate_res.confirmed.is_some());

    // ==========================================
    // 3. Verify Node Service (Staging & Mount lifecycle)
    // ==========================================
    let node_info = Node::node_get_info(&driver, Request::new(NodeGetInfoRequest {}))
        .await?
        .into_inner();
    assert_eq!(node_info.node_id, "node-01");

    let node_caps =
        Node::node_get_capabilities(&driver, Request::new(NodeGetCapabilitiesRequest {}))
            .await?
            .into_inner();
    assert_eq!(node_caps.capabilities.len(), 1);

    // Generate unique temporary paths to test mounts/stages
    let temp_uuid = uuid::Uuid::new_v4();
    let stage_dir = std::env::temp_dir().join(format!("csi-stage-{}", temp_uuid));
    let publish_dir = std::env::temp_dir().join(format!("csi-publish-{}", temp_uuid));
    let stage_path_str = stage_dir.to_string_lossy().to_string();
    let publish_path_str = publish_dir.to_string_lossy().to_string();

    // Stage Volume
    let _stage_res = Node::node_stage_volume(
        &driver,
        Request::new(NodeStageVolumeRequest {
            volume_id: volume_id.clone(),
            publish_context: HashMap::new(),
            staging_target_path: stage_path_str.clone(),
            volume_capability: None,
            secrets: HashMap::new(),
            volume_context: HashMap::new(),
        }),
    )
    .await?;
    assert!(Path::new(&stage_path_str).exists());

    // Publish Volume (Mount)
    let _publish_res = Node::node_publish_volume(
        &driver,
        Request::new(NodePublishVolumeRequest {
            volume_id: volume_id.clone(),
            publish_context: HashMap::new(),
            staging_target_path: stage_path_str.clone(),
            target_path: publish_path_str.clone(),
            volume_capability: None,
            readonly: false,
            secrets: HashMap::new(),
            volume_context: HashMap::new(),
        }),
    )
    .await?;
    assert!(Path::new(&publish_path_str).exists());

    // Unpublish Volume (Unmount)
    let _unpublish_res = Node::node_unpublish_volume(
        &driver,
        Request::new(NodeUnpublishVolumeRequest {
            volume_id: volume_id.clone(),
            target_path: publish_path_str.clone(),
        }),
    )
    .await?;
    assert!(!Path::new(&publish_path_str).exists());

    // Unstage Volume
    let _unstage_res = Node::node_unstage_volume(
        &driver,
        Request::new(NodeUnstageVolumeRequest {
            volume_id: volume_id.clone(),
            staging_target_path: stage_path_str.clone(),
        }),
    )
    .await?;
    assert!(!Path::new(&stage_path_str).exists());

    // ==========================================
    // 4. Verify Publish & Deletion Workflow
    // ==========================================
    // Controller publish
    let pub_res = Controller::controller_publish_volume(
        &driver,
        Request::new(ControllerPublishVolumeRequest {
            volume_id: volume_id.clone(),
            node_id: "node-01".to_string(),
            volume_capability: None,
            readonly: false,
            secrets: HashMap::new(),
            volume_context: HashMap::new(),
        }),
    )
    .await?
    .into_inner();
    let dev_path = pub_res
        .publish_context
        .get("device_path")
        .ok_or("device_path missing in publish_context")?;
    assert!(dev_path.contains(&volume_id));

    // Controller unpublish
    let _unpub_res = Controller::controller_unpublish_volume(
        &driver,
        Request::new(ControllerUnpublishVolumeRequest {
            volume_id: volume_id.clone(),
            node_id: "node-01".to_string(),
            secrets: HashMap::new(),
        }),
    )
    .await?;

    // Delete Volume
    let _delete_res = Controller::delete_volume(
        &driver,
        Request::new(DeleteVolumeRequest {
            volume_id: volume_id.clone(),
            secrets: HashMap::new(),
        }),
    )
    .await?;

    // Verify list is empty
    let list_res_empty = Controller::list_volumes(
        &driver,
        Request::new(ListVolumesRequest {
            max_entries: 10,
            starting_token: String::new(),
        }),
    )
    .await?
    .into_inner();
    assert_eq!(list_res_empty.entries.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_csi_zvol_block_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let driver = AetherCsiDriver::new("node-01".to_string());

    // 1. Create a volume
    let vol_name = "test-csi-block-volume".to_string();
    let create_res = Controller::create_volume(
        &driver,
        Request::new(CreateVolumeRequest {
            name: vol_name.clone(),
            capacity_range: Some(CapacityRange {
                required_bytes: 1024 * 1024 * 1024, // 1 GiB
                limit_bytes: 1024 * 1024 * 1024,
            }),
            volume_capabilities: Vec::new(),
            parameters: HashMap::new(),
            secrets: HashMap::new(),
            volume_content_source: None,
            accessibility_requirements: None,
            mutable_parameters: HashMap::new(),
        }),
    )
    .await?
    .into_inner();

    let volume = create_res.volume.ok_or("Volume missing")?;
    let volume_id = volume.volume_id;

    // 2. Setup block capability
    use aether_auth::csi::volume_capability;
    let block_cap = aether_auth::csi::VolumeCapability {
        access_type: Some(volume_capability::AccessType::Block(
            volume_capability::BlockVolume {},
        )),
        access_mode: Some(volume_capability::AccessMode {
            mode: volume_capability::access_mode::Mode::SingleNodeWriter as i32,
        }),
    };

    let temp_uuid = uuid::Uuid::new_v4();
    let stage_dir = std::env::temp_dir().join(format!("csi-stage-block-{}", temp_uuid));
    let publish_dir = std::env::temp_dir().join(format!("csi-publish-block-{}", temp_uuid));
    let stage_path_str = stage_dir.to_string_lossy().to_string();
    let publish_path_str = publish_dir.to_string_lossy().to_string();

    // Stage Volume with Block Capability
    let _stage_res = Node::node_stage_volume(
        &driver,
        Request::new(NodeStageVolumeRequest {
            volume_id: volume_id.clone(),
            publish_context: HashMap::new(),
            staging_target_path: stage_path_str.clone(),
            volume_capability: Some(block_cap.clone()),
            secrets: HashMap::new(),
            volume_context: HashMap::new(),
        }),
    )
    .await?;

    // Staging path should exist as a regular file, not a directory
    assert!(Path::new(&stage_path_str).exists());
    assert!(Path::new(&stage_path_str).is_file());

    // Publish Volume with Block Capability
    let _publish_res = Node::node_publish_volume(
        &driver,
        Request::new(NodePublishVolumeRequest {
            volume_id: volume_id.clone(),
            publish_context: HashMap::new(),
            staging_target_path: stage_path_str.clone(),
            target_path: publish_path_str.clone(),
            volume_capability: Some(block_cap),
            readonly: false,
            secrets: HashMap::new(),
            volume_context: HashMap::new(),
        }),
    )
    .await?;

    // Publishing path should exist as a regular file, not a directory
    assert!(Path::new(&publish_path_str).exists());
    assert!(Path::new(&publish_path_str).is_file());

    // Unpublish Volume
    let _unpublish_res = Node::node_unpublish_volume(
        &driver,
        Request::new(NodeUnpublishVolumeRequest {
            volume_id: volume_id.clone(),
            target_path: publish_path_str.clone(),
        }),
    )
    .await?;
    assert!(!Path::new(&publish_path_str).exists());

    // Unstage Volume
    let _unstage_res = Node::node_unstage_volume(
        &driver,
        Request::new(NodeUnstageVolumeRequest {
            volume_id: volume_id.clone(),
            staging_target_path: stage_path_str.clone(),
        }),
    )
    .await?;
    assert!(!Path::new(&stage_path_str).exists());

    // Cleanup Volume
    let _delete_res = Controller::delete_volume(
        &driver,
        Request::new(DeleteVolumeRequest {
            volume_id: volume_id.clone(),
            secrets: HashMap::new(),
        }),
    )
    .await?;

    Ok(())
}
