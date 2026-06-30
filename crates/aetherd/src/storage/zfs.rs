use crate::storage::ZvolManager;
use async_trait::async_trait;
use std::io;

#[cfg(all(target_os = "linux", not(tarpaulin)))]
use zfs_core::{DataSetType, Zfs};

#[cfg(all(target_os = "linux", not(tarpaulin)))]
fn nvlist_add_uint64(nv: &mut nvpair::NvList, name: &str, value: u64) -> io::Result<()> {
    let name_c =
        std::ffi::CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { nvpair_sys::nvlist_add_uint64(nv.as_mut_ptr(), name_c.as_ptr(), value) };
    if ret != 0 {
        Err(io::Error::from_raw_os_error(ret))
    } else {
        Ok(())
    }
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
async fn read_total_memory() -> io::Result<u64> {
    let content = tokio::fs::read_to_string("/proc/meminfo").await?;
    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(kb) = parts[1].parse::<u64>() {
                    return Ok(kb * 1024); // Return total bytes
                }
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "MemTotal not found in /proc/meminfo",
    ))
}

/// Production ZFS ZVOL manager implementation for Linux.
///
/// In the Aether chassis deployment, ZVOLs are created on dedicated Storage blades
/// (Slots 9-16) and exposed over the backplane network (VLAN 11) using iSCSI
/// targets. Compute host blades log in as initiators and attach the resulting raw
/// block device locally before passing it down into compute VM hypervisors.
#[cfg(all(target_os = "linux", not(tarpaulin)))]
pub struct RealZvolManager {
    pool: String,
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
impl RealZvolManager {
    /// Create a new RealZvolManager for the specified pool.
    pub fn new(pool: String) -> Self {
        Self { pool }
    }
}

#[cfg(all(target_os = "linux", not(tarpaulin)))]
#[async_trait]
impl ZvolManager for RealZvolManager {
    async fn create_zvol(&self, name: &str, size_bytes: u64) -> io::Result<String> {
        let pool = self.pool.clone();
        let name_str = name.to_string();
        tokio::task::spawn_blocking(move || {
            let z = Zfs::new()?;
            let mut props = nvpair::NvList::new();
            nvlist_add_uint64(&mut props, "volsize", size_bytes)?;
            let dataset_path = format!("{}/{}", pool, name_str);
            z.create(&dataset_path, DataSetType::Zvol, &props)?;
            Ok(format!("/dev/zvol/{}/{}", pool, name_str))
        })
        .await?
    }

    async fn create_snapshot(&self, zvol_name: &str, snapshot_name: &str) -> io::Result<()> {
        let pool = self.pool.clone();
        let zvol = zvol_name.to_string();
        let snap = snapshot_name.to_string();
        tokio::task::spawn_blocking(move || {
            let z = Zfs::new()?;
            let snaps = vec![format!("{}/{}@{}", pool, zvol, snap)];
            z.snapshot(snaps)
                .map_err(|e| io::Error::other(format!("ZFS snapshot error: {:?}", e)))?;
            Ok(())
        })
        .await?
    }

    async fn clone_zvol(&self, snapshot_name: &str, clone_name: &str) -> io::Result<String> {
        let pool = self.pool.clone();
        let snap = snapshot_name.to_string();
        let clone = clone_name.to_string();
        tokio::task::spawn_blocking(move || {
            let z = Zfs::new()?;
            let clone_path = format!("{}/{}", pool, clone);
            let origin_path = format!("{}/{}", pool, snap);
            let mut props = nvpair::NvList::new();
            z.clone_dataset(&clone_path, &origin_path, &mut props)
                .map_err(|e| io::Error::other(format!("ZFS clone error: {:?}", e)))?;
            Ok(format!("/dev/zvol/{}/{}", pool, clone))
        })
        .await?
    }

    async fn rollback_zvol(&self, zvol_name: &str, snapshot_name: &str) -> io::Result<()> {
        let pool = self.pool.clone();
        let zvol = zvol_name.to_string();
        let snap = snapshot_name.to_string();
        tokio::task::spawn_blocking(move || {
            let z = Zfs::new()?;
            let fsname = format!("{}/{}", pool, zvol);
            z.rollback_to(&fsname, &snap)?;
            Ok(())
        })
        .await?
    }

    async fn resize_zvol(&self, zvol_name: &str, new_size_bytes: u64) -> io::Result<()> {
        let pool = self.pool.clone();
        let zvol = zvol_name.to_string();
        tokio::task::spawn_blocking(move || {
            let dataset_path = format!("{}/{}", pool, zvol);
            let status = std::process::Command::new("zfs")
                .arg("set")
                .arg(format!("volsize={}", new_size_bytes))
                .arg(&dataset_path)
                .status()?;
            if status.success() {
                Ok(())
            } else {
                Err(io::Error::other("zfs set command failed"))
            }
        })
        .await?
    }

    async fn destroy_zvol(&self, name: &str) -> io::Result<()> {
        let pool = self.pool.clone();
        let name_str = name.to_string();
        tokio::task::spawn_blocking(move || {
            let z = Zfs::new()?;
            let path = format!("{}/{}", pool, name_str);
            z.destroy(&path)?;
            Ok(())
        })
        .await?
    }

    async fn configure_arc_cache_limit(&self) -> io::Result<()> {
        let total_mem = read_total_memory().await?;
        let arc_max = (total_mem as f64 * 0.15) as u64;
        log::info!(
            "Configuring ZFS ARC max limit to 15% of memory: {} bytes",
            arc_max
        );

        let path = "/sys/module/zfs/parameters/zfs_arc_max";
        if let Err(e) = tokio::fs::write(path, arc_max.to_string()).await {
            log::warn!("Failed to write to /sys/module/zfs/parameters/zfs_arc_max: {}. This requires root privileges.", e);
        }

        // Persistent configuration in /etc/modprobe.d/zfs.conf
        let conf_path = "/etc/modprobe.d/zfs.conf";
        let conf_content = format!("options zfs zfs_arc_max={}\n", arc_max);
        if let Some(parent) = std::path::Path::new(conf_path).parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        if let Err(e) = tokio::fs::write(conf_path, &conf_content).await {
            log::warn!("Failed to write persistent ZFS ARC limit to {}: {}. This requires root privileges.", conf_path, e);
        }

        // Trigger update-initramfs -u if we are on a Debian/Ubuntu system
        if std::path::Path::new("/usr/sbin/update-initramfs").exists() {
            match tokio::process::Command::new("update-initramfs")
                .arg("-u")
                .status()
                .await
            {
                Ok(status) => {
                    if !status.success() {
                        log::warn!("update-initramfs command failed");
                    }
                }
                Err(e) => {
                    log::warn!("Failed to execute update-initramfs: {}", e);
                }
            }
        }

        Ok(())
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
struct MockState {
    zvols: HashMap<String, u64>,
    snapshots: HashMap<String, String>,
}

/// Simulated ZVOL manager implementation for macOS development and CI testing.
///
/// Mocks local ZVOL creation using sparse files. In testing environments, this simulates
/// the local node volume structure without requiring a native ZFS pool or iSCSI target exports.
pub struct MockZvolManager {
    _pool: String,
    state: Arc<Mutex<MockState>>,
    temp_dir: tempfile::TempDir,
}

impl MockZvolManager {
    /// Create a new MockZvolManager for test simulation.
    pub fn try_new(pool: String) -> io::Result<Self> {
        let temp_dir = tempfile::TempDir::new()?;
        Ok(Self {
            _pool: pool,
            state: Arc::new(Mutex::new(MockState::default())),
            temp_dir,
        })
    }
}

#[async_trait]
impl ZvolManager for MockZvolManager {
    async fn create_zvol(&self, name: &str, size_bytes: u64) -> io::Result<String> {
        let path = self.temp_dir.path().join(name);

        // Ensure any parent sub-directories exist
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Create a sparse file simulating a zvol block device
        let file = tokio::fs::File::create(&path).await?;
        file.set_len(size_bytes).await?;

        let mut state = self.state.lock().await;
        state.zvols.insert(name.to_string(), size_bytes);

        Ok(path.to_string_lossy().to_string())
    }

    async fn create_snapshot(&self, zvol_name: &str, snapshot_name: &str) -> io::Result<()> {
        let mut state = self.state.lock().await;
        if !state.zvols.contains_key(zvol_name) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ZVOL not found: {}", zvol_name),
            ));
        }

        let snap_key = format!("{}@{}", zvol_name, snapshot_name);
        state.snapshots.insert(snap_key, zvol_name.to_string());
        Ok(())
    }

    async fn clone_zvol(&self, snapshot_name: &str, clone_name: &str) -> io::Result<String> {
        let state = self.state.lock().await;
        let zvol_name = state.snapshots.get(snapshot_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Snapshot not found: {}", snapshot_name),
            )
        })?;

        let size = state.zvols.get(zvol_name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Origin ZVOL not found: {}", zvol_name),
            )
        })?;

        let size_val = *size;
        drop(state);

        // Create sparse file clone
        let path = self.temp_dir.path().join(clone_name);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let file = tokio::fs::File::create(&path).await?;
        file.set_len(size_val).await?;

        let mut state = self.state.lock().await;
        state.zvols.insert(clone_name.to_string(), size_val);

        Ok(path.to_string_lossy().to_string())
    }

    async fn rollback_zvol(&self, zvol_name: &str, snapshot_name: &str) -> io::Result<()> {
        let state = self.state.lock().await;
        let snap_key = format!("{}@{}", zvol_name, snapshot_name);
        if !state.snapshots.contains_key(&snap_key) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Snapshot not found: {}", snap_key),
            ));
        }
        Ok(())
    }

    async fn resize_zvol(&self, zvol_name: &str, new_size_bytes: u64) -> io::Result<()> {
        let mut state = self.state.lock().await;
        if !state.zvols.contains_key(zvol_name) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ZVOL not found: {}", zvol_name),
            ));
        }

        let path = self.temp_dir.path().join(zvol_name);
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .await?;
        file.set_len(new_size_bytes).await?;

        state.zvols.insert(zvol_name.to_string(), new_size_bytes);
        Ok(())
    }

    async fn destroy_zvol(&self, name: &str) -> io::Result<()> {
        let mut state = self.state.lock().await;

        let path = self.temp_dir.path().join(name);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
        }

        state.zvols.remove(name);
        state
            .snapshots
            .retain(|k, _| !k.starts_with(&(name.to_string() + "@")));

        Ok(())
    }

    async fn configure_arc_cache_limit(&self) -> io::Result<()> {
        log::info!("Simulating ZFS ARC cache limit setup (15% memory limit)");
        Ok(())
    }
}
