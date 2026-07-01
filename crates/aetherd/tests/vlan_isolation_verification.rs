// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use aetherd::network::bridge::MockBridgeManager;
use aetherd::network::BridgeManager;
use std::io;

fn is_permission_error(err: &io::Error) -> bool {
    if err.kind() == io::ErrorKind::PermissionDenied {
        return true;
    }
    let s_display = err.to_string();
    let s_debug = format!("{:?}", err);
    let check = |s: &str| {
        s.contains("PermissionDenied")
            || s.contains("Permission denied")
            || s.contains("Operation not permitted")
            || s.contains("code: Some(-1)")
            || s.contains("code: Some(-13)")
            || s.contains("code: Some(NonZero(-1))")
            || s.contains("code: Some(NonZero(-13))")
    };
    if check(&s_display) || check(&s_debug) {
        return true;
    }
    if let Some(code) = err.raw_os_error() {
        if code == 1 || code == 13 {
            // EPERM = 1, EACCES = 13
            return true;
        }
    }
    false
}

#[tokio::test]
async fn test_mock_bridge_manager_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let manager = MockBridgeManager::new();
    let vlan_id = 42;
    let tap_name = "tap-mock-42";

    // 1. Create tenant bridge
    let bridge_name = manager.create_tenant_bridge(vlan_id).await?;
    assert_eq!(bridge_name, format!("br-tenant-{}", vlan_id));

    // 2. Create TAP device
    manager.create_tap_device(tap_name, &bridge_name).await?;

    // 3. Apply MAC anti-spoofing
    manager
        .apply_mac_anti_spoofing(tap_name, "52:54:00:12:34:56")
        .await?;

    // 4. Teardown
    manager.teardown_tenant_network(vlan_id, tap_name).await?;

    Ok(())
}

#[cfg(test)]
mod permission_error_tests {
    use super::*;

    #[test]
    fn test_is_permission_error_kind() {
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        assert!(is_permission_error(&err));
    }

    #[test]
    fn test_is_permission_error_raw_os_eperm() {
        let err = io::Error::from_raw_os_error(1); // EPERM
        assert!(is_permission_error(&err));
    }

    #[test]
    fn test_is_permission_error_raw_os_eacces() {
        let err = io::Error::from_raw_os_error(13); // EACCES
        assert!(is_permission_error(&err));
    }

    #[test]
    fn test_is_permission_error_string_permission_denied() {
        let err = io::Error::other("Permission denied");
        assert!(is_permission_error(&err));
    }

    #[test]
    fn test_is_permission_error_string_operation_not_permitted() {
        let err = io::Error::other("Operation not permitted");
        assert!(is_permission_error(&err));
    }

    #[test]
    fn test_is_permission_error_non_permission() {
        let err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        assert!(!is_permission_error(&err));
    }

    #[test]
    fn test_is_permission_error_io_busy() {
        let err = io::Error::new(io::ErrorKind::WouldBlock, "resource busy");
        assert!(!is_permission_error(&err));
    }
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
#[tokio::test]
async fn test_real_bridge_manager_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    use aetherd::network::bridge::RealBridgeManager;
    let manager = RealBridgeManager::new();
    let vlan_id = 999;
    let tap_name = "tap-test-999";
    let allowed_mac = "52:54:00:99:99:99";

    // 1. Create tenant bridge
    let bridge_name = match manager.create_tenant_bridge(vlan_id).await {
        Ok(name) => name,
        Err(e) if is_permission_error(&e) => {
            println!(
                "Skipping real netlink tests: insufficient permissions (must run as root/sudo)"
            );
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };
    assert_eq!(bridge_name, format!("br-tenant-{}", vlan_id));

    // 2. Create TAP device
    if let Err(e) = manager.create_tap_device(tap_name, &bridge_name).await {
        if is_permission_error(&e) {
            println!("Skipping real TAP tests: insufficient permissions");
            let _ = manager.teardown_tenant_network(vlan_id, tap_name).await;
            return Ok(());
        }
        let _ = manager.teardown_tenant_network(vlan_id, tap_name).await;
        return Err(e.into());
    }

    // 3. Apply MAC anti-spoofing
    if let Err(e) = manager.apply_mac_anti_spoofing(tap_name, allowed_mac).await {
        if is_permission_error(&e) {
            println!("Skipping real MAC anti-spoofing tests: insufficient permissions");
            let _ = manager.teardown_tenant_network(vlan_id, tap_name).await;
            return Ok(());
        }
        let _ = manager.teardown_tenant_network(vlan_id, tap_name).await;
        return Err(e.into());
    }

    // 4. Teardown
    manager.teardown_tenant_network(vlan_id, tap_name).await?;

    Ok(())
}
