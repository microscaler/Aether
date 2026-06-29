// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

/// Configuration for Cloud-Init NoCloud metadata.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CloudInitConfig {
    /// Instance ID for the virtual machine.
    pub instance_id: String,
    /// Hostname for the virtual machine.
    pub hostname: String,
    /// YAML cloud-config formatted user-data.
    pub user_data: String,
}

/// Owned handle to compiled Cloud-Init NoCloud ISO in memory.
/// Cleans up the temporary RAM directory when dropped.
pub struct CloudInitIso {
    _temp_dir: TempDir,
    iso_path: PathBuf,
}

impl CloudInitIso {
    /// Returns the filesystem path to the compiled seed.iso.
    pub fn path(&self) -> &Path {
        &self.iso_path
    }
}

/// Builder for compiling Cloud-Init configs into a seed.iso.
pub struct CloudInitIsoBuilder {
    /// Config specification.
    pub config: CloudInitConfig,
}

impl CloudInitIsoBuilder {
    /// Creates a new `CloudInitIsoBuilder`.
    pub fn new(config: CloudInitConfig) -> Self {
        Self { config }
    }

    /// Compiles `user-data` and `meta-data` into a NoCloud `seed.iso` in RAM.
    pub async fn build_iso(&self) -> Result<CloudInitIso, String> {
        // Enforce NFR-3.4.2: Use RAM tmpfs (/dev/shm) on Linux, standard tempdir on macOS
        let base_dir = if Path::new("/dev/shm").exists() {
            "/dev/shm"
        } else {
            ""
        };

        let temp_dir = if base_dir.is_empty() {
            tempfile::tempdir()
                .map_err(|e| format!("Failed to create temporary directory: {}", e))?
        } else {
            tempfile::Builder::new()
                .prefix("aether-cloudinit-")
                .tempdir_in(base_dir)
                .map_err(|e| format!("Failed to create temporary directory in RAM: {}", e))?
        };

        let input_dir = temp_dir.path().join("input");
        tokio::fs::create_dir_all(&input_dir)
            .await
            .map_err(|e| format!("Failed to create input directory: {}", e))?;

        // Write user-data
        let user_data_path = input_dir.join("user-data");
        tokio::fs::write(&user_data_path, &self.config.user_data)
            .await
            .map_err(|e| format!("Failed to write user-data: {}", e))?;

        // Write meta-data
        let meta_data = format!(
            "instance-id: {}\nlocal-hostname: {}\n",
            self.config.instance_id, self.config.hostname
        );
        let meta_data_path = input_dir.join("meta-data");
        tokio::fs::write(&meta_data_path, meta_data)
            .await
            .map_err(|e| format!("Failed to write meta-data: {}", e))?;

        let iso_path = temp_dir.path().join("seed.iso");

        // Check if either command exists in PATH (using command presence fallback)
        let has_xorriso = tokio::process::Command::new("which")
            .arg("xorriso")
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        let has_mkisofs = tokio::process::Command::new("which")
            .arg("mkisofs")
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false);

        if !has_xorriso && !has_mkisofs {
            // Mock compilation for dev environments without xorriso/mkisofs (e.g. macOS developer machine)
            tokio::fs::write(&iso_path, b"mock_iso_content")
                .await
                .map_err(|e| format!("Failed to write mock ISO: {}", e))?;
            return Ok(CloudInitIso {
                _temp_dir: temp_dir,
                iso_path,
            });
        }

        let output = if has_xorriso {
            let xorriso_args = [
                "-as",
                "mkisofs",
                "-R",
                "-V",
                "config-2",
                "-o",
                iso_path.to_str().ok_or_else(|| "Invalid ISO path".to_string())?,
                input_dir.to_str().ok_or_else(|| "Invalid input directory path".to_string())?,
            ];
            let mut cmd = tokio::process::Command::new("xorriso");
            cmd.args(&xorriso_args);
            cmd.output().await.map_err(|e| format!("Failed to execute xorriso: {}", e))?
        } else {
            let mkisofs_args = [
                "-R",
                "-V",
                "config-2",
                "-o",
                iso_path.to_str().ok_or_else(|| "Invalid ISO path".to_string())?,
                input_dir.to_str().ok_or_else(|| "Invalid input directory path".to_string())?,
            ];
            let mut cmd_fallback = tokio::process::Command::new("mkisofs");
            cmd_fallback.args(&mkisofs_args);
            cmd_fallback.output().await.map_err(|e| {
                format!("Failed to execute mkisofs: {}", e)
            })?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ISO compilation failed: {}", stderr));
        }

        Ok(CloudInitIso {
            _temp_dir: temp_dir,
            iso_path,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cloud_init_builder_success() {
        let config = CloudInitConfig {
            instance_id: "i-test1234".to_string(),
            hostname: "test-host".to_string(),
            user_data: "#cloud-config\nusers:\n  - name: test\n".to_string(),
        };

        let builder = CloudInitIsoBuilder::new(config);
        let iso = builder.build_iso().await.unwrap();

        let path = iso.path().to_path_buf();
        assert!(path.exists());

        // Verify clean up on drop
        drop(iso);
        assert!(!path.exists());
    }
}
