// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

/// Configuration for target file and command paths.
#[derive(Clone, Debug)]
pub struct TelemetryConfig {
    /// Path to CPU loadavg file (typically "/proc/loadavg").
    pub loadavg_path: String,
    /// Path to memory statistics file (typically "/proc/meminfo").
    pub meminfo_path: String,
    /// Path to NVMe temperature file (typically in sysfs).
    pub nvme_temp_path: String,
    /// Disk mount point to inspect.
    pub mount_point: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            loadavg_path: "/proc/loadavg".to_string(),
            meminfo_path: "/proc/meminfo".to_string(),
            nvme_temp_path: "/sys/class/nvme/nvme0/device/hwmon/hwmon1/temp1_input".to_string(),
            mount_point: "/".to_string(),
        }
    }
}

/// A structured report of the host system telemetry metrics.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemMetrics {
    /// 1-minute CPU load average.
    pub load_one: f64,
    /// 5-minute CPU load average.
    pub load_five: f64,
    /// 15-minute CPU load average.
    pub load_fifteen: f64,
    /// Total system memory capacity in bytes.
    pub mem_total: u64,
    /// Currently available memory in bytes.
    pub mem_available: u64,
    /// Total disk capacity in bytes at mount point.
    pub disk_total: u64,
    /// Available disk capacity in bytes at mount point.
    pub disk_available: u64,
    /// NVMe controller temperature in degrees Celsius.
    pub nvme_temp: f64,
    /// Number of physical/logical CPU cores.
    pub cpu_cores: u32,
}

/// Helper struct for parsing CPU load averages.
#[derive(Clone, Debug)]
pub struct LoadAvg {
    /// 1-minute load average.
    pub one: f64,
    /// 5-minute load average.
    pub five: f64,
    /// 15-minute load average.
    pub fifteen: f64,
}

/// Helper struct for parsing Memory statistics.
#[derive(Clone, Debug)]
pub struct MemoryInfo {
    /// Total memory in bytes.
    pub total_bytes: u64,
    /// Available memory in bytes.
    pub available_bytes: u64,
}

/// Helper struct for parsing Disk space information.
#[derive(Clone, Debug)]
pub struct DiskInfo {
    /// Total disk capacity in bytes.
    pub total_bytes: u64,
    /// Available disk capacity in bytes.
    pub available_bytes: u64,
}

/// Parses the system loadavg file.
pub fn parse_loadavg(path: &str) -> Result<LoadAvg, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() < 3 {
        return Err("Malformed loadavg content".to_string());
    }
    let one = parts[0].parse::<f64>().map_err(|e| e.to_string())?;
    let five = parts[1].parse::<f64>().map_err(|e| e.to_string())?;
    let fifteen = parts[2].parse::<f64>().map_err(|e| e.to_string())?;
    Ok(LoadAvg { one, five, fifteen })
}

/// Parses the system meminfo file.
pub fn parse_meminfo(path: &str) -> Result<MemoryInfo, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut total = None;
    let mut available = None;
    let mut free = None;

    for line in content.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0].trim();
        let val_str = parts[1].trim();
        let val_parts: Vec<&str> = val_str.split_whitespace().collect();
        if val_parts.is_empty() {
            continue;
        }
        let val = val_parts[0].parse::<u64>().map_err(|e| e.to_string())? * 1024; // convert kB to bytes

        if key == "MemTotal" {
            total = Some(val);
        } else if key == "MemAvailable" {
            available = Some(val);
        } else if key == "MemFree" {
            free = Some(val);
        }
    }

    let total_bytes = total.ok_or_else(|| "MemTotal not found".to_string())?;
    let available_bytes = available
        .or(free)
        .ok_or_else(|| "MemAvailable or MemFree not found".to_string())?;

    Ok(MemoryInfo {
        total_bytes,
        available_bytes,
    })
}

/// Obtains disk metrics by running POSIX `df` command (safe, no unsafe blocks).
pub fn get_disk_space(mount_point: &str) -> Result<DiskInfo, String> {
    let output = std::process::Command::new("df")
        .args(["-k", mount_point])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "df command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let out_str = String::from_utf8_lossy(&output.stdout);
    let mut lines = out_str.lines();
    let _header = lines.next().ok_or_else(|| "Empty df output".to_string())?;

    let data_line = lines
        .next()
        .ok_or_else(|| "Missing data line in df output".to_string())?;
    let parts: Vec<&str> = data_line.split_whitespace().collect();

    if parts.len() < 4 {
        return Err(format!("Malformed df output line: {}", data_line));
    }

    // POSIX df output fields: Filesystem, 1024-blocks, Used, Available
    let total_kb = parts[1].parse::<u64>().map_err(|e| e.to_string())?;
    let available_kb = parts[3].parse::<u64>().map_err(|e| e.to_string())?;

    Ok(DiskInfo {
        total_bytes: total_kb * 1024,
        available_bytes: available_kb * 1024,
    })
}

/// Reads the NVMe controller temperature.
pub fn get_nvme_temp(path: &str) -> Result<f64, String> {
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(milli) = content.trim().parse::<f64>() {
            return Ok(milli / 1000.0);
        }
    }

    // Command fallback
    let output = std::process::Command::new("nvme")
        .args(["smart-log", "/dev/nvme0"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let out_str = String::from_utf8_lossy(&out.stdout);
            for line in out_str.lines() {
                if line.to_lowercase().contains("temperature") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() == 2 {
                        let temp_str = parts[1].trim().trim_end_matches('C').trim();
                        if let Ok(temp) = temp_str.parse::<f64>() {
                            return Ok(temp);
                        }
                    }
                }
            }
            Err("Failed to parse nvme smart-log output".to_string())
        }
        _ => {
            log::warn!("NVMe command not found or failed, using fallback temperature 35.0 C");
            Ok(35.0)
        }
    }
}

/// Collector to fetch metrics.
#[derive(Clone, Debug)]
pub struct TelemetryCollector {
    config: TelemetryConfig,
}

impl TelemetryCollector {
    /// Creates a new TelemetryCollector with the specified config.
    pub fn new(config: TelemetryConfig) -> Self {
        Self { config }
    }

    /// Collects system telemetry metrics. Falls back to mock values if reading fails.
    pub fn collect(&self) -> SystemMetrics {
        let load = parse_loadavg(&self.config.loadavg_path).unwrap_or_else(|e| {
            log::warn!("Failed to parse loadavg: {}, using mock values", e);
            LoadAvg {
                one: 0.5,
                five: 0.5,
                fifteen: 0.5,
            }
        });

        let mem = parse_meminfo(&self.config.meminfo_path).unwrap_or_else(|e| {
            log::warn!("Failed to parse meminfo: {}, using mock values", e);
            MemoryInfo {
                total_bytes: 16 * 1024 * 1024 * 1024,
                available_bytes: 8 * 1024 * 1024 * 1024,
            }
        });

        let disk = get_disk_space(&self.config.mount_point).unwrap_or_else(|e| {
            log::warn!("Failed to get disk space: {}, using mock values", e);
            DiskInfo {
                total_bytes: 100 * 1024 * 1024 * 1024,
                available_bytes: 50 * 1024 * 1024 * 1024,
            }
        });

        let nvme_temp = get_nvme_temp(&self.config.nvme_temp_path).unwrap_or_else(|e| {
            log::warn!("Failed to get NVMe temp: {}, using mock values", e);
            35.0
        });

        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);

        SystemMetrics {
            load_one: load.one,
            load_five: load.five,
            load_fifteen: load.fifteen,
            mem_total: mem.total_bytes,
            mem_available: mem.available_bytes,
            disk_total: disk.total_bytes,
            disk_available: disk.available_bytes,
            nvme_temp,
            cpu_cores,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert_eq!(config.loadavg_path, "/proc/loadavg");
        assert_eq!(config.meminfo_path, "/proc/meminfo");
    }

    #[test]
    fn test_loadavg_parsing() {
        let mut path = std::env::temp_dir();
        path.push(format!("mock_loadavg_{}", std::process::id()));
        std::fs::write(&path, "0.12 0.34 0.56 1/824 384729\n").unwrap();

        let load = parse_loadavg(path.to_str().unwrap()).unwrap();
        assert_eq!(load.one, 0.12);
        assert_eq!(load.five, 0.34);
        assert_eq!(load.fifteen, 0.56);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_loadavg_parsing_malformed() {
        let mut path = std::env::temp_dir();
        path.push(format!("mock_loadavg_bad_{}", std::process::id()));
        std::fs::write(&path, "0.12\n").unwrap();

        let res = parse_loadavg(path.to_str().unwrap());
        assert!(res.is_err());

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_meminfo_parsing() {
        let mut path = std::env::temp_dir();
        path.push(format!("mock_meminfo_{}", std::process::id()));
        std::fs::write(
            &path,
            "MemTotal:        16324024 kB\nMemFree:          9124028 kB\nMemAvailable:    12324908 kB\n",
        )
        .unwrap();

        let mem = parse_meminfo(path.to_str().unwrap()).unwrap();
        assert_eq!(mem.total_bytes, 16324024 * 1024);
        assert_eq!(mem.available_bytes, 12324908 * 1024);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_meminfo_parsing_fallback_free() {
        let mut path = std::env::temp_dir();
        path.push(format!("mock_meminfo_fallback_{}", std::process::id()));
        std::fs::write(
            &path,
            "MemTotal:        16324024 kB\nMemFree:          9124028 kB\n",
        )
        .unwrap();

        let mem = parse_meminfo(path.to_str().unwrap()).unwrap();
        assert_eq!(mem.total_bytes, 16324024 * 1024);
        assert_eq!(mem.available_bytes, 9124028 * 1024);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_nvme_temp_parsing() {
        let mut path = std::env::temp_dir();
        path.push(format!("mock_temp_{}", std::process::id()));
        std::fs::write(&path, "35000\n").unwrap();

        let temp = get_nvme_temp(path.to_str().unwrap()).unwrap();
        assert_eq!(temp, 35.0);

        std::fs::remove_file(path).unwrap();
    }
}
