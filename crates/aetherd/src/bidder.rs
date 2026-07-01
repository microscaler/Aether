// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::telemetry::SystemMetrics;

/// Configuration for Bidding calculations.
#[derive(Clone, Debug)]
pub struct BidderConfig {
    /// Maximum CPU load ratio (load_one / cpu_cores) allowed before disqualifying node.
    pub max_cpu_load: f64,
    /// Critical NVMe temperature (in degrees Celsius) that disqualifies the node.
    pub critical_temp: f64,
}

impl Default for BidderConfig {
    fn default() -> Self {
        Self {
            max_cpu_load: 2.0,
            critical_temp: 80.0,
        }
    }
}

/// Bidder evaluates workload requests against node metrics and calculates bids.
pub struct Bidder {
    config: BidderConfig,
}

impl Bidder {
    /// Creates a new Bidder with the specified configuration.
    pub fn new(config: BidderConfig) -> Self {
        Self { config }
    }

    /// Evaluates requested resources against system metrics and calculates a score.
    /// Returns `-1` if resources are exhausted or critical limits are exceeded.
    /// Otherwise, returns an efficiency score between 1 and 1000.
    pub fn calculate_bid(
        &self,
        metrics: &SystemMetrics,
        cpu_request: i32,
        memory_request_bytes: i64,
        disk_request_bytes: i64,
    ) -> i32 {
        // 1. Boundary & Capacity Checks
        if cpu_request <= 0 || memory_request_bytes < 0 || disk_request_bytes < 0 {
            return -1;
        }

        // Hard capacity checks (reject if request exceeds available bytes)
        if memory_request_bytes as u64 > metrics.mem_available {
            return -1;
        }
        if disk_request_bytes as u64 > metrics.disk_available {
            return -1;
        }

        // CPU load average check: disqualify if current load ratio exceeds max threshold
        if metrics.cpu_cores == 0 {
            return -1;
        }
        let cpu_load_ratio = metrics.load_one / metrics.cpu_cores as f64;
        if cpu_load_ratio > self.config.max_cpu_load {
            return -1;
        }

        // NVMe temperature check: disqualify if critical temp limit is hit
        if metrics.nvme_temp >= self.config.critical_temp {
            return -1;
        }

        // 2. Score Calculations (when resources are available)
        // Memory free fraction after allocation (higher is better)
        let mem_left = metrics
            .mem_available
            .saturating_sub(memory_request_bytes as u64);
        let mem_score = (mem_left as f64 / metrics.mem_total as f64).clamp(0.0, 1.0);

        // Disk free fraction after allocation (higher is better)
        let disk_left = metrics
            .disk_available
            .saturating_sub(disk_request_bytes as u64);
        let disk_score = (disk_left as f64 / metrics.disk_total as f64).clamp(0.0, 1.0);

        // CPU score representing load headroom
        let cpu_score = (1.0 - (cpu_load_ratio / self.config.max_cpu_load)).clamp(0.0, 1.0);

        // Temperature penalty factor: degrades score if temperature is elevated (> 60C)
        let temp_penalty = if metrics.nvme_temp > 60.0 {
            ((self.config.critical_temp - metrics.nvme_temp) / (self.config.critical_temp - 60.0))
                .clamp(0.0, 1.0)
        } else {
            1.0
        };

        // Migration penalty: Each active migration reduces the overall score by 15%
        // This prevents nodes from being overwhelmed by simultaneous migrations.
        let migration_penalty = (1.0 - (metrics.active_migrations as f64 * 0.15)).clamp(0.0, 1.0);

        // Combine scores with weights: Memory (40%), CPU (40%), Disk (20%)
        let raw_score = (mem_score * 0.4 + cpu_score * 0.4 + disk_score * 0.2)
            * temp_penalty
            * migration_penalty;

        // Map raw_score (0.0 to 1.0) to range 1 to 1000
        (1.0 + raw_score * 999.0) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_metrics() -> SystemMetrics {
        SystemMetrics {
            load_one: 0.5,
            load_five: 0.5,
            load_fifteen: 0.5,
            mem_total: 10 * 1024 * 1024 * 1024,      // 10 GB
            mem_available: 5 * 1024 * 1024 * 1024,   // 5 GB
            disk_total: 100 * 1024 * 1024 * 1024,    // 100 GB
            disk_available: 50 * 1024 * 1024 * 1024, // 50 GB
            nvme_temp: 35.0,
            cpu_cores: 4,
            active_migrations: 0,
        }
    }

    #[test]
    fn test_bidding_calculator_normal() {
        let bidder = Bidder::new(BidderConfig::default());
        let metrics = mock_metrics();

        let score =
            bidder.calculate_bid(&metrics, 2, 2 * 1024 * 1024 * 1024, 10 * 1024 * 1024 * 1024);
        assert!((1..=1000).contains(&score));
    }

    #[test]
    fn test_bidding_calculator_exceeded_memory() {
        let bidder = Bidder::new(BidderConfig::default());
        let metrics = mock_metrics();

        // Request 8GB memory when only 5GB is available
        let score =
            bidder.calculate_bid(&metrics, 2, 8 * 1024 * 1024 * 1024, 10 * 1024 * 1024 * 1024);
        assert_eq!(score, -1);
    }

    #[test]
    fn test_bidding_calculator_exceeded_disk() {
        let bidder = Bidder::new(BidderConfig::default());
        let metrics = mock_metrics();

        // Request 60GB disk when only 50GB is available
        let score = bidder.calculate_bid(&metrics, 2, 1024 * 1024 * 1024, 60 * 1024 * 1024 * 1024);
        assert_eq!(score, -1);
    }

    #[test]
    fn test_bidding_calculator_overheat() {
        let bidder = Bidder::new(BidderConfig::default());
        let mut metrics = mock_metrics();
        metrics.nvme_temp = 85.0; // critical

        let score = bidder.calculate_bid(&metrics, 2, 1024 * 1024 * 1024, 10 * 1024 * 1024 * 1024);
        assert_eq!(score, -1);
    }

    #[test]
    fn test_bidding_calculator_high_cpu_load() {
        let bidder = Bidder::new(BidderConfig::default());
        let mut metrics = mock_metrics();
        metrics.load_one = 9.0; // load ratio = 9.0 / 4 = 2.25 > 2.0

        let score = bidder.calculate_bid(&metrics, 2, 1024 * 1024 * 1024, 10 * 1024 * 1024 * 1024);
        assert_eq!(score, -1);
    }
}
