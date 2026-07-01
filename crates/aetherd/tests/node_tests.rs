// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use tonic::Request;

use aether_auth::proto::aether_node_server::AetherNode;
use aether_auth::proto::{BidRequest, ExecuteVmRequest, ListVMsRequest, TeardownVmRequest};
use aether_auth::token::TokenManager;
use aetherd::bidder::Bidder;
use aetherd::migration::RealMigrationManager;
use aetherd::storage::iscsi::RealIscsiManager;
use aetherd::telemetry::TelemetryCollector;
use aetherd::AetherNodeImpl;

struct TestContext {
    _temp_dir: tempfile::TempDir,
}

type SetupResult =
    Result<(TestContext, Arc<TelemetryCollector>, Arc<Bidder>), Box<dyn std::error::Error>>;

fn setup_test_context() -> SetupResult {
    let dir = tempfile::tempdir()?;
    let loadavg_path = dir.path().join("loadavg");
    let meminfo_path = dir.path().join("meminfo");
    let nvme_temp_path = dir.path().join("nvme_temp");

    std::fs::write(&loadavg_path, "0.10 0.20 0.30 1/100 12345\n")?;
    std::fs::write(
        &meminfo_path,
        "MemTotal:        16384000 kB\nMemFree:          8192000 kB\nMemAvailable:     8192000 kB\n",
    )?;
    std::fs::write(&nvme_temp_path, "35000\n")?;

    let config = aetherd::telemetry::TelemetryConfig {
        loadavg_path: loadavg_path
            .to_str()
            .ok_or("Invalid loadavg path")?
            .to_string(),
        meminfo_path: meminfo_path
            .to_str()
            .ok_or("Invalid meminfo path")?
            .to_string(),
        nvme_temp_path: nvme_temp_path
            .to_str()
            .ok_or("Invalid nvme_temp path")?
            .to_string(),
        mount_point: "/".to_string(),
    };

    let telemetry_collector = Arc::new(TelemetryCollector::new(config));
    let bidder = Arc::new(Bidder::new(aetherd::bidder::BidderConfig::default()));

    Ok((TestContext { _temp_dir: dir }, telemetry_collector, bidder))
}

#[tokio::test]
async fn test_node_reverse_bid() -> Result<(), Box<dyn std::error::Error>> {
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let (_ctx, telemetry_collector, bidder) = setup_test_context()?;
    let migration_manager = Arc::new(RealMigrationManager::new("127.0.0.1".to_string()));
    let iscsi_manager = Arc::new(RealIscsiManager);

    let node = AetherNodeImpl::new(
        "test-node-1".to_string(),
        "COMPUTE".to_string(),
        token_manager,
        telemetry_collector,
        bidder,
        migration_manager,
        iscsi_manager,
    );

    let res = node
        .request_reverse_bid(Request::new(BidRequest {
            workload_uuid: "uuid-123".to_string(),
            cpu_request: 2,
            memory_request_bytes: 1024 * 1024 * 1024,
            disk_request_bytes: 2048,
        }))
        .await?;

    let inner = res.into_inner();
    assert_eq!(inner.node_id, "test-node-1");
    assert!(inner.score > 0);
    assert!(inner.score <= 1000);
    Ok(())
}

#[tokio::test]
async fn test_node_execute_vm_validation() -> Result<(), Box<dyn std::error::Error>> {
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let (_ctx, telemetry_collector, bidder) = setup_test_context()?;
    let migration_manager = Arc::new(RealMigrationManager::new("127.0.0.1".to_string()));
    let iscsi_manager = Arc::new(RealIscsiManager);

    let node = AetherNodeImpl::new(
        "test-node-1".to_string(),
        "COMPUTE".to_string(),
        token_manager.clone(),
        telemetry_collector,
        bidder,
        migration_manager,
        iscsi_manager,
    );

    // Generate valid token
    let token = token_manager.generate_token("test-node-1")?;

    // Valid execute_vm
    let res = node
        .execute_vm(Request::new(ExecuteVmRequest {
            token: token.clone(),
            workload_uuid: "uuid-123".to_string(),
            name: "test-vm".to_string(),
            cpu_limit: 2,
            memory_limit_bytes: 1024 * 1024 * 1024,
            image_uri: "docker://test".to_string(),
        }))
        .await?;

    assert!(res.into_inner().success);

    // Tampered token execute_vm
    let mut parts: Vec<&str> = token.split(':').collect();
    parts[3] = "invalid_signature";
    let bad_token = format!("{}:{}:{}:{}", parts[0], parts[1], parts[2], parts[3]);

    let res_err = node
        .execute_vm(Request::new(ExecuteVmRequest {
            token: bad_token,
            workload_uuid: "uuid-123".to_string(),
            name: "test-vm".to_string(),
            cpu_limit: 2,
            memory_limit_bytes: 1024 * 1024 * 1024,
            image_uri: "docker://test".to_string(),
        }))
        .await;

    let err = res_err.err().ok_or("Expected unauthenticated error")?;
    assert_eq!(err.code(), tonic::Code::Unauthenticated);

    // Clean up
    let teardown_token = token_manager.generate_token("test-node-1")?;
    let _ = node
        .teardown_vm(Request::new(TeardownVmRequest {
            token: teardown_token,
            workload_uuid: "uuid-123".to_string(),
        }))
        .await;

    Ok(())
}

#[tokio::test]
async fn test_node_teardown_vm_validation() -> Result<(), Box<dyn std::error::Error>> {
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let (_ctx, telemetry_collector, bidder) = setup_test_context()?;
    let migration_manager = Arc::new(RealMigrationManager::new("127.0.0.1".to_string()));
    let iscsi_manager = Arc::new(RealIscsiManager);

    let node = AetherNodeImpl::new(
        "test-node-1".to_string(),
        "COMPUTE".to_string(),
        token_manager.clone(),
        telemetry_collector,
        bidder,
        migration_manager,
        iscsi_manager,
    );

    // Generate valid token
    let token = token_manager.generate_token("test-node-1")?;

    // Execute first
    let exec_res = node
        .execute_vm(Request::new(ExecuteVmRequest {
            token: token.clone(),
            workload_uuid: "uuid-123".to_string(),
            name: "test-vm".to_string(),
            cpu_limit: 2,
            memory_limit_bytes: 1024 * 1024 * 1024,
            image_uri: "docker://test".to_string(),
        }))
        .await?;
    assert!(exec_res.into_inner().success);

    // Valid teardown_vm
    let teardown_token = token_manager.generate_token("test-node-1")?;
    let res = node
        .teardown_vm(Request::new(TeardownVmRequest {
            token: teardown_token,
            workload_uuid: "uuid-123".to_string(),
        }))
        .await?;

    assert!(res.into_inner().success);

    // Tampered token teardown_vm
    let mut parts: Vec<&str> = token.split(':').collect();
    parts[3] = "invalid_signature";
    let bad_token = format!("{}:{}:{}:{}", parts[0], parts[1], parts[2], parts[3]);

    let res_err = node
        .teardown_vm(Request::new(TeardownVmRequest {
            token: bad_token,
            workload_uuid: "uuid-123".to_string(),
        }))
        .await;

    let err = res_err.err().ok_or("Expected unauthenticated error")?;
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
    Ok(())
}

#[tokio::test]
async fn test_node_list_vms() -> Result<(), Box<dyn std::error::Error>> {
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let (_ctx, telemetry_collector, bidder) = setup_test_context()?;
    let migration_manager = Arc::new(RealMigrationManager::new("127.0.0.1".to_string()));
    let iscsi_manager = Arc::new(RealIscsiManager);

    let node = AetherNodeImpl::new(
        "test-node-1".to_string(),
        "COMPUTE".to_string(),
        token_manager.clone(),
        telemetry_collector,
        bidder,
        migration_manager,
        iscsi_manager,
    );

    let token = token_manager.generate_token("test-node-1")?;

    let res = node.list_v_ms(Request::new(ListVMsRequest {})).await?;
    assert!(res.into_inner().vms.is_empty());

    // Spawn VM
    let _ = node
        .execute_vm(Request::new(ExecuteVmRequest {
            token: token.clone(),
            workload_uuid: "uuid-123".to_string(),
            name: "test-vm".to_string(),
            cpu_limit: 2,
            memory_limit_bytes: 1024 * 1024 * 1024,
            image_uri: "docker://test".to_string(),
        }))
        .await?;

    let res2 = node.list_v_ms(Request::new(ListVMsRequest {})).await?;
    assert_eq!(res2.into_inner().vms.len(), 1);

    // Cleanup VM
    let teardown_token = token_manager.generate_token("test-node-1")?;
    let _ = node
        .teardown_vm(Request::new(TeardownVmRequest {
            token: teardown_token,
            workload_uuid: "uuid-123".to_string(),
        }))
        .await;

    Ok(())
}
