// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::sync::Arc;
use tonic::Request;

use aether_auth::proto::aether_node_server::AetherNode;
use aether_auth::proto::{BidRequest, ExecuteVmRequest, ListVMsRequest, TeardownVmRequest};
use aether_auth::token::TokenManager;
use aetherd::AetherNodeImpl;

#[tokio::test]
async fn test_node_reverse_bid() -> Result<(), Box<dyn std::error::Error>> {
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let node = AetherNodeImpl::new("test-node-1".to_string(), token_manager);

    let res = node
        .request_reverse_bid(Request::new(BidRequest {
            workload_uuid: "uuid-123".to_string(),
            cpu_request: 2,
            memory_request_bytes: 1024,
            disk_request_bytes: 2048,
        }))
        .await?;

    let inner = res.into_inner();
    assert_eq!(inner.node_id, "test-node-1");
    assert_eq!(inner.score, 950);
    Ok(())
}

#[tokio::test]
async fn test_node_execute_vm_validation() -> Result<(), Box<dyn std::error::Error>> {
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let node = AetherNodeImpl::new("test-node-1".to_string(), token_manager.clone());

    // Generate valid token
    let token = token_manager.generate_token("test-node-1")?;

    // Valid execute_vm
    let res = node
        .execute_vm(Request::new(ExecuteVmRequest {
            token: token.clone(),
            workload_uuid: "uuid-123".to_string(),
            name: "test-vm".to_string(),
            cpu_limit: 2,
            memory_limit_bytes: 1024,
            image_uri: "docker://test".to_string(),
        }))
        .await?;

    assert!(res.into_inner().success);

    // Tampered token execute_vm
    let mut parts: Vec<&str> = token.split(':').collect();
    parts[2] = "invalid_signature";
    let bad_token = format!("{}:{}:{}", parts[0], parts[1], parts[2]);

    let res_err = node
        .execute_vm(Request::new(ExecuteVmRequest {
            token: bad_token,
            workload_uuid: "uuid-123".to_string(),
            name: "test-vm".to_string(),
            cpu_limit: 2,
            memory_limit_bytes: 1024,
            image_uri: "docker://test".to_string(),
        }))
        .await;

    let err = res_err.err().ok_or("Expected unauthenticated error")?;
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
    Ok(())
}

#[tokio::test]
async fn test_node_teardown_vm_validation() -> Result<(), Box<dyn std::error::Error>> {
    let token_manager = Arc::new(TokenManager::new(
        b"supersecretkeyforauthsupersecretkeyforauth".to_vec(),
    ));
    let node = AetherNodeImpl::new("test-node-1".to_string(), token_manager.clone());

    // Generate valid token
    let token = token_manager.generate_token("test-node-1")?;

    // Valid teardown_vm
    let res = node
        .teardown_vm(Request::new(TeardownVmRequest {
            token: token.clone(),
            workload_uuid: "uuid-123".to_string(),
        }))
        .await?;

    assert!(res.into_inner().success);

    // Tampered token teardown_vm
    let mut parts: Vec<&str> = token.split(':').collect();
    parts[2] = "invalid_signature";
    let bad_token = format!("{}:{}:{}", parts[0], parts[1], parts[2]);

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
    let node = AetherNodeImpl::new("test-node-1".to_string(), token_manager);

    let res = node.list_v_ms(Request::new(ListVMsRequest {})).await?;
    assert!(res.into_inner().vms.is_empty());
    Ok(())
}
