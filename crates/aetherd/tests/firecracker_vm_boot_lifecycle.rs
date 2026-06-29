// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use aetherd::hypervisor::firecracker::{
    BootSource, Drive, FirecrackerConfig, FirecrackerHypervisor, MachineConfig,
};
use aetherd::hypervisor::Hypervisor;
use std::time::Instant;
use tempfile::tempdir;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_firecracker_vm_boot_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let config_path = dir.path().join("vm.json").to_str().unwrap().to_string();
    let log_path = dir.path().join("vm.log").to_str().unwrap().to_string();

    let config = FirecrackerConfig {
        boot_source: BootSource {
            kernel_image_path: "/path/to/kernel".to_string(),
            boot_args: "console=ttyS0 reboot=k panic=1 pci=off".to_string(),
        },
        drives: vec![Drive {
            drive_id: "rootfs".to_string(),
            path_on_host: "/path/to/rootfs".to_string(),
            is_root_device: true,
            is_read_only: false,
        }],
        machine_config: MachineConfig {
            vcpu_count: 1,
            mem_size_mib: 512,
            smt: Some(false),
        },
        network_interfaces: vec![],
    };

    let mut hypervisor = FirecrackerHypervisor::new(
        "test-vm-boot".to_string(),
        "sleep".to_string(),
        config_path,
        log_path.clone(),
        config,
    );
    hypervisor.extra_args = vec!["5".to_string()];

    // Measure boot spawn time to verify NFR-3.1.1 (< 100ms boot/spawn time)
    let start_time = Instant::now();
    hypervisor.spawn().await?;
    let duration = start_time.elapsed();
    println!("Mock Firecracker spawn completed in {:?}", duration);
    assert!(
        duration.as_millis() < 100,
        "Firecracker spawn took longer than 100ms: {:?}",
        duration
    );

    // Verify VM is running
    assert_eq!(hypervisor.query_status().await?, "RUNNING");

    // Terminate VM and verify stopped
    hypervisor.stop().await?;
    assert_eq!(hypervisor.query_status().await?, "STOPPED");

    // Verify console log file is generated
    assert!(std::path::Path::new(&log_path).exists());

    Ok(())
}
