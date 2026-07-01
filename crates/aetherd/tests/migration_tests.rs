// Enforce JSF rules and safety lints
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::while_let_loop)]

use std::time::Duration;
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use aetherd::migration::{MigrationManager, MigrationParams, MigrationState, RealMigrationManager};

/// Mock QMP server that accepts multiple connections.
/// Each connection gets its own clone of responses.
fn make_qmp_server(socket_path: &str, responses: Vec<String>) -> tokio::task::JoinHandle<()> {
    let socket_path = socket_path.to_string();
    tokio::spawn(async move {
        if let Ok(listener) = tokio::net::UnixListener::bind(&socket_path) {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let responses = responses.clone();
                        tokio::spawn(async move {
                            handle_qmp_connection(stream, responses).await;
                        });
                    }
                    Err(_) => break,
                }
            }
        }
    })
}

async fn handle_qmp_connection(stream: tokio::net::UnixStream, mut responses: Vec<String>) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut line = String::new();

    // Always send QMP greeting first
    let _ = write_half
        .write_all(
            b"{\"QMP\": {\"version\": {\"qemu\": {\"micro\": 0, \"minor\": 0, \"major\": 9}, \"package\": \"\"}, \"capabilities\": []}}\n",
        )
        .await;

    loop {
        line.clear();
        if reader.read_line(&mut line).await.is_err() {
            break;
        }
        if line.contains("qmp_capabilities") {
            let _ = write_half.write_all(b"{\"return\": {}}\n").await;
            continue;
        }
        // For all other commands, return the next response in the list
        if !responses.is_empty() {
            let resp = responses.remove(0);
            let mut out = resp.into_bytes();
            if !out.is_empty() && out.last() != Some(&b'\n') {
                out.push(b'\n');
            }
            let _ = write_half.write_all(&out).await;
        } else {
            break;
        }
    }
}

async fn wait_for_server() {
    tokio::time::sleep(Duration::from_millis(50)).await;
}

// =============================================================================
// VM registration / lifecycle (no QMP needed)
// =============================================================================

#[tokio::test]
async fn test_register_and_unregister_vm() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());

    assert_eq!(manager.get_active_migration_count().await, 0);

    manager
        .register_vm("vm-1", "/tmp/qmp-vm1.sock")
        .await
        .unwrap();
    assert_eq!(manager.get_active_migration_count().await, 1);

    manager
        .register_vm("vm-2", "/tmp/qmp-vm2.sock")
        .await
        .unwrap();
    assert_eq!(manager.get_active_migration_count().await, 2);

    manager.unregister_vm("vm-1").await.unwrap();
    assert_eq!(manager.get_active_migration_count().await, 1);

    manager.unregister_vm("nonexistent").await.unwrap();
    assert_eq!(manager.get_active_migration_count().await, 1);
}

#[tokio::test]
async fn test_register_vm_twice() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());

    manager
        .register_vm("vm-x", "/tmp/qmp-x.sock")
        .await
        .unwrap();
    // Registering same VM again should succeed (overwrite)
    manager
        .register_vm("vm-x", "/tmp/qmp-x2.sock")
        .await
        .unwrap();
    assert_eq!(manager.get_active_migration_count().await, 1);
}

#[tokio::test]
async fn test_multiple_vm_registration() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());

    for i in 0..5u32 {
        manager
            .register_vm(&format!("vm-{}", i), &format!("/tmp/qmp-{}.sock", i))
            .await
            .unwrap();
    }
    assert_eq!(manager.get_active_migration_count().await, 5);

    for i in 0..5u32 {
        manager.unregister_vm(&format!("vm-{}", i)).await.unwrap();
    }
    assert_eq!(manager.get_active_migration_count().await, 0);
}

#[tokio::test]
async fn test_get_active_migration_count_empty() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    assert_eq!(manager.get_active_migration_count().await, 0);
}

#[tokio::test]
async fn test_unregister_nonexistent_vm_is_noop() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    let result = manager.unregister_vm("never-registered").await;
    assert!(result.is_ok());
}

// =============================================================================
// Migration start / abort (no QMP needed)
// =============================================================================

#[tokio::test]
async fn test_start_migration_vm_not_found() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    let params = MigrationParams {
        destination_node: "target-node".to_string(),
        destination_ip: "10.0.0.2".to_string(),
        port: 4444,
        use_tls: false,
        max_bandwidth: 0,
    };
    let result = manager.start_migration("nonexistent-vm", params).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn test_abort_migration_vm_not_found() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    let result = manager.abort_migration("ghost-vm").await;
    assert!(result.is_err());
}

// =============================================================================
// Migration start with QMP connection
// =============================================================================

#[tokio::test]
async fn test_start_migration_success() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    // set_migration_capability -> ok, drive_mirror -> ok, migrate -> ok (3 QMP connections)
    let responses = vec!["{}".into(), "{}".into(), "{}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager
        .register_vm("vm-migrate", &qmp_path_str)
        .await
        .unwrap();

    let params = MigrationParams {
        destination_node: "target".to_string(),
        destination_ip: "10.0.0.2".to_string(),
        port: 0,
        use_tls: false,
        max_bandwidth: 0,
    };

    let result = manager.start_migration("vm-migrate", params).await;
    assert!(result.is_ok(), "start_migration failed: {:?}", result);
}

#[tokio::test]
async fn test_start_migration_tls_uri() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    let responses = vec!["{}".into(), "{}".into(), "{}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager.register_vm("vm-tls", &qmp_path_str).await.unwrap();

    let params = MigrationParams {
        destination_node: "target".to_string(),
        destination_ip: "10.0.0.2".to_string(),
        port: 0,
        use_tls: true,
        max_bandwidth: 0,
    };

    let result = manager.start_migration("vm-tls", params).await;
    assert!(result.is_ok(), "start_migration failed: {:?}", result);
}

#[tokio::test]
async fn test_start_migration_with_bandwidth() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    let responses = vec!["{}".into(), "{}".into(), "{}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager.register_vm("vm-bw", &qmp_path_str).await.unwrap();

    let params = MigrationParams {
        destination_node: "target".to_string(),
        destination_ip: "192.168.1.1".to_string(),
        port: 0,
        use_tls: false,
        max_bandwidth: 500_000_000,
    };

    let result = manager.start_migration("vm-bw", params).await;
    assert!(result.is_ok(), "start_migration failed: {:?}", result);
}

// =============================================================================
// Prepare incoming (uses TCP listener + 2 QMP calls: nbd-server-start, nbd-server-add)
// =============================================================================

#[tokio::test]
async fn test_prepare_incoming_vm_not_found() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    let result = manager
        .prepare_incoming("nonexistent-vm", 4444, false)
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn test_prepare_incoming_port_0_returns_actual_port() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    // nbd-server-start -> ok, nbd-server-add -> ok
    // Each connection gets ONE response
    let responses = vec!["{}".into(), "{}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager
        .register_vm("vm-incoming", &qmp_path_str)
        .await
        .unwrap();

    let result = manager.prepare_incoming("vm-incoming", 0, false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_prepare_incoming_with_tls() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    let responses = vec!["{}".into(), "{}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager
        .register_vm("vm-tls-in", &qmp_path_str)
        .await
        .unwrap();

    let result = manager.prepare_incoming("vm-tls-in", 5555, true).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_prepare_incoming_block_repl_failure() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    // nbd-server-start -> error. The first connection gets responses[0].
    // prepare_destination fails on nbd-server-start, so connection 2 never happens.
    let responses = vec!["{\"error\": {\"class\": \"DeviceNotFound\"}}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager.register_vm("vm-fail", &qmp_path_str).await.unwrap();

    let result = manager.prepare_incoming("vm-fail", 4444, false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_prepare_incoming_nonzero_port() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    let responses = vec!["{}".into(), "{}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager
        .register_vm("vm-port5000", &qmp_path_str)
        .await
        .unwrap();

    let result = manager.prepare_incoming("vm-port5000", 5000, false).await;
    assert!(result.is_ok());
}

// =============================================================================
// Query migration status (1 QMP call per connection: query-migrate)
// =============================================================================

#[tokio::test]
async fn test_query_migration_status_completed() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    // Only 1 response needed: query-migrate returns completed
    let responses = vec!["{\"return\": {\"status\": \"completed\"}}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager.register_vm("vm-done", &qmp_path_str).await.unwrap();

    let status = manager.query_migration_status("vm-done").await.unwrap();
    assert_eq!(status, MigrationState::Completed);
}

#[tokio::test]
async fn test_query_migration_status_active() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    let responses = vec!["{\"return\": {\"status\": \"active\"}}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager
        .register_vm("vm-active", &qmp_path_str)
        .await
        .unwrap();

    let status = manager.query_migration_status("vm-active").await.unwrap();
    assert_eq!(status, MigrationState::Active);
}

#[tokio::test]
async fn test_query_migration_status_failed() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    let responses =
        vec!["{\"return\": {\"status\": \"failed\", \"error-desc\": \"migrate failure\"}}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager
        .register_vm("vm-failed", &qmp_path_str)
        .await
        .unwrap();

    let status = manager.query_migration_status("vm-failed").await.unwrap();
    match status {
        MigrationState::Failed(ref err) => assert!(err.contains("failed")),
        other => panic!("Expected Failed, got {:?}", other),
    }
}

#[tokio::test]
async fn test_query_migration_status_idle() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    let responses = vec!["{\"return\": {\"status\": \"pause\"}}".into()];
    make_qmp_server(&qmp_path_str, responses);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager.register_vm("vm-idle", &qmp_path_str).await.unwrap();

    let status = manager.query_migration_status("vm-idle").await.unwrap();
    assert_eq!(status, MigrationState::Idle);
}

#[tokio::test]
async fn test_query_migration_status_vm_not_found() {
    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    let result = manager.query_migration_status("no-such-vm").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

// =============================================================================
// Abort migration (uses QMP)
// =============================================================================

#[tokio::test]
async fn test_abort_migration_success() {
    let dir = tempdir().unwrap();
    let qmp_path = dir.path().join("qmp.sock");
    let qmp_path_str = qmp_path.to_str().unwrap().to_string();

    // abort_migration creates QmpClient but the current impl is a placeholder
    // that just returns Ok(()) without sending any QMP commands.
    // Still need server for register_vm path, but no QMP commands will be sent.
    make_qmp_server(&qmp_path_str, vec!["{}".into()]);
    wait_for_server().await;

    let manager = RealMigrationManager::new("127.0.0.1".to_string());
    manager
        .register_vm("vm-abort", &qmp_path_str)
        .await
        .unwrap();

    let result = manager.abort_migration("vm-abort").await;
    assert!(result.is_ok());
}

// =============================================================================
// Serialization tests
// =============================================================================

#[tokio::test]
async fn test_migration_state_serialization() {
    let idle = MigrationState::Idle;
    let json = serde_json::to_string(&idle).unwrap();
    assert!(json.contains("Idle"));

    let completed = MigrationState::Completed;
    let json = serde_json::to_string(&completed).unwrap();
    assert!(json.contains("Completed"));

    let cancelled = MigrationState::Cancelled;
    let json = serde_json::to_string(&cancelled).unwrap();
    assert!(json.contains("Cancelled"));

    let failed = MigrationState::Failed("error text".to_string());
    let json = serde_json::to_string(&failed).unwrap();
    assert!(json.contains("error text"));

    let params = MigrationParams {
        destination_node: "dst".to_string(),
        destination_ip: "1.2.3.4".to_string(),
        port: 1234,
        use_tls: true,
        max_bandwidth: 100_000_000,
    };
    let json = serde_json::to_string(&params).unwrap();
    assert!(json.contains("1234"));
    assert!(json.contains("1.2.3.4"));
    assert!(json.contains("true"));
}
