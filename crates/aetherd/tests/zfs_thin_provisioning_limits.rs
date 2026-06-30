use aetherd::storage::zfs::MockZvolManager;
use aetherd::storage::ZvolManager;

#[tokio::test]
async fn test_mock_zvol_lifecycle() {
    // 1. Initialize Mock ZVOL Manager
    let manager =
        MockZvolManager::try_new("tank".to_string()).expect("Failed to initialize MockZvolManager");

    // 2. Create ZVOL
    let zvol_name = "test-volume";
    let initial_size = 10 * 1024 * 1024; // 10 MiB
    let dev_path = manager
        .create_zvol(zvol_name, initial_size)
        .await
        .expect("Failed to create ZVOL");

    assert!(
        std::path::Path::new(&dev_path).exists(),
        "ZVOL device file does not exist"
    );
    let metadata = std::fs::metadata(&dev_path).expect("Failed to read device file metadata");
    assert_eq!(metadata.len(), initial_size, "Initial size mismatch");

    // 3. Create Snapshot
    let snapshot_name = "snap1";
    manager
        .create_snapshot(zvol_name, snapshot_name)
        .await
        .expect("Failed to create snapshot");

    // 4. Create Thin Clone
    let clone_name = "test-volume-clone";
    let clone_dev_path = manager
        .clone_zvol(&format!("{}@{}", zvol_name, snapshot_name), clone_name)
        .await
        .expect("Failed to create thin clone");

    assert!(
        std::path::Path::new(&clone_dev_path).exists(),
        "Clone device file does not exist"
    );
    let clone_metadata = std::fs::metadata(&clone_dev_path).expect("Failed to read clone metadata");
    assert_eq!(clone_metadata.len(), initial_size, "Clone size mismatch");

    // 5. Rollback
    manager
        .rollback_zvol(zvol_name, snapshot_name)
        .await
        .expect("Failed to rollback ZVOL");

    // 6. Resize
    let new_size = 20 * 1024 * 1024; // 20 MiB
    manager
        .resize_zvol(zvol_name, new_size)
        .await
        .expect("Failed to resize ZVOL");

    let resized_metadata = std::fs::metadata(&dev_path).expect("Failed to read resized metadata");
    assert_eq!(resized_metadata.len(), new_size, "Resized size mismatch");

    // 7. Destroy
    manager
        .destroy_zvol(zvol_name)
        .await
        .expect("Failed to destroy ZVOL");
    assert!(
        !std::path::Path::new(&dev_path).exists(),
        "ZVOL device file still exists after destroy"
    );

    manager
        .destroy_zvol(clone_name)
        .await
        .expect("Failed to destroy Clone");
    assert!(
        !std::path::Path::new(&clone_dev_path).exists(),
        "Clone device file still exists after destroy"
    );
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
#[tokio::test]
async fn test_real_zvol_lifecycle_conditional() {
    use aetherd::storage::zfs::RealZvolManager;
    use std::env;

    // Only run if ZFS_TEST_POOL is explicitly provided in the environment
    if let Ok(pool_name) = env::var("ZFS_TEST_POOL") {
        log::info!(
            "Running real ZFS integration tests against pool: {}",
            pool_name
        );
        let manager = RealZvolManager::new(pool_name);

        // Run ARC Cache setup (requires root privileges - will log warning if failing)
        let _ = manager.configure_arc_cache_limit().await;

        let zvol_name = format!("test-zvol-{}", uuid::Uuid::new_v4());
        let initial_size = 5 * 1024 * 1024; // 5 MiB

        // Create ZVOL
        let dev_path = manager
            .create_zvol(&zvol_name, initial_size)
            .await
            .expect("Failed to create real ZVOL");

        assert!(!dev_path.is_empty(), "Device path should not be empty");

        // Create Snapshot
        let snapshot_name = "snap-test";
        manager
            .create_snapshot(&zvol_name, snapshot_name)
            .await
            .expect("Failed to create real snapshot");

        // Clone ZVOL
        let clone_name = format!("test-clone-{}", uuid::Uuid::new_v4());
        let clone_dev_path = manager
            .clone_zvol(&format!("{}@{}", zvol_name, snapshot_name), &clone_name)
            .await
            .expect("Failed to clone real zvol");

        assert!(
            !clone_dev_path.is_empty(),
            "Clone device path should not be empty"
        );

        // Rollback
        manager
            .rollback_zvol(&zvol_name, snapshot_name)
            .await
            .expect("Failed to rollback real ZVOL");

        // Resize
        let new_size = 10 * 1024 * 1024; // 10 MiB
        manager
            .resize_zvol(&zvol_name, new_size)
            .await
            .expect("Failed to resize real ZVOL");

        // Destroy Clone
        manager
            .destroy_zvol(&clone_name)
            .await
            .expect("Failed to destroy real clone");

        // Destroy ZVOL
        manager
            .destroy_zvol(&zvol_name)
            .await
            .expect("Failed to destroy real ZVOL");
    } else {
        println!("Skipping real ZFS integration test (ZFS_TEST_POOL env var not set)");
    }
}
