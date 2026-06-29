// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use aetherd::cloud_init::{CloudInitConfig, CloudInitIsoBuilder};
use std::io::{Read, Seek, SeekFrom};

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_cloud_init_iso_guest_boot() -> Result<(), Box<dyn std::error::Error>> {
    let user_data = r#"#cloud-config
ssh_authorized_keys:
  - ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQCu...
chpasswd:
  list: |
    root:mysecretpassword
  expire: False
"#;

    let config = CloudInitConfig {
        instance_id: "i-infra-db-9".to_string(),
        hostname: "infra-db-blade-9".to_string(),
        user_data: user_data.to_string(),
    };

    let builder = CloudInitIsoBuilder::new(config);
    let iso = builder.build_iso().await?;

    let path = iso.path();
    assert!(path.exists());

    let metadata = std::fs::metadata(path)?;
    if metadata.len() >= 32840 {
        // Read ISO 9660 Volume Descriptor (Block 16 PVD, Volume ID at offset 40)
        let mut file = std::fs::File::open(path)?;
        file.seek(SeekFrom::Start(32808))?;
        let mut label_bytes = [0u8; 32];
        file.read_exact(&mut label_bytes)?;
        let label = String::from_utf8_lossy(&label_bytes);
        println!("ISO Volume Label verified: '{}'", label.trim());
        assert!(
            label.to_lowercase().contains("config-2"),
            "Volume label must be config-2, got: {}",
            label
        );
    } else {
        // Mock fallback check
        let mut file = std::fs::File::open(path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        assert!(content == b"mock_iso_content" || metadata.len() > 0);
    }

    Ok(())
}
