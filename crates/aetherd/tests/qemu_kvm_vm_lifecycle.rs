// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use aetherd::hypervisor::qemu::{QemuConfig, QemuHypervisor};
use aetherd::hypervisor::Hypervisor;
use std::time::Duration;
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_qemu_kvm_vm_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let qmp_socket_path = dir.path().join("qmp.sock").to_str().unwrap().to_string();
    let log_path = dir.path().join("qemu.log").to_str().unwrap().to_string();

    // Setup mock QMP Server
    let qmp_socket_path_clone = qmp_socket_path.clone();
    let qmp_server = tokio::spawn(async move {
        if let Ok(listener) = tokio::net::UnixListener::bind(&qmp_socket_path_clone) {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut reader = tokio::io::BufReader::new(&mut stream);
                // Write greeting
                if stream
                    .write_all(
                        b"{\"QMP\": {\"version\": {\"qemu\": {\"micro\": 0, \"minor\": 0, \"major\": 9}, \"package\": \"\"}, \"capabilities\": []}}\n",
                    )
                    .await
                    .is_err()
                {
                    break;
                }
                let mut line = String::new();
                if reader.read_line(&mut line).await.is_err() {
                    break;
                }
                if !line.contains("qmp_capabilities") {
                    break;
                }
                if stream.write_all(b"{\"return\": {}}\n").await.is_err() {
                    break;
                }
                line.clear();
                if reader.read_line(&mut line).await.is_err() {
                    break;
                }
                if line.contains("query-status") {
                    let _ = stream
                        .write_all(b"{\"return\": {\"running\": true, \"status\": \"running\"}}\n")
                        .await;
                }
            }
        }
    });

    let config = QemuConfig {
        vcpu_count: 2,
        mem_size_mib: 1024,
        disk_image_path: "/path/to/disk.img".to_string(),
        qmp_socket_path,
        host_tap_device: None,
    };

    let mut hypervisor = QemuHypervisor::new(
        "test-qemu-vm".to_string(),
        "sleep".to_string(),
        log_path.clone(),
        config,
    );
    hypervisor.extra_args = vec!["10".to_string()];

    // Spawn the hypervisor
    hypervisor.spawn().await?;

    // Wait for mock QMP server to get ready
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check status via QMP
    let status = hypervisor.query_status().await?;
    assert_eq!(status, "RUNNING");

    // Clean up/Stop
    hypervisor.stop().await?;
    assert_eq!(hypervisor.query_status().await?, "STOPPED");

    // Verify logs
    assert!(std::path::Path::new(&log_path).exists());

    // Join mock server
    let _ = qmp_server.await;

    Ok(())
}
