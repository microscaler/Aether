// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use aetherd::telemetry::{TelemetryCollector, TelemetryConfig};

#[test]
fn test_host_system_metrics_query() -> Result<(), Box<dyn std::error::Error>> {
    // Instantiate collector with default configuration
    let collector = TelemetryCollector::new(TelemetryConfig::default());

    // Trigger metrics collection
    let metrics = collector.collect();

    // Verify CPU load values (loads must be non-negative)
    assert!(metrics.load_one >= 0.0);
    assert!(metrics.load_five >= 0.0);
    assert!(metrics.load_fifteen >= 0.0);
    assert!(metrics.cpu_cores > 0);

    // Verify Memory Info (capacity should be non-zero)
    assert!(metrics.mem_total > 0);
    assert!(metrics.mem_available > 0);
    assert!(metrics.mem_available <= metrics.mem_total);

    // Verify Disk Info (total should be non-zero)
    assert!(metrics.disk_total > 0);
    assert!(metrics.disk_available <= metrics.disk_total);

    // Verify NVMe temperature
    assert!(metrics.nvme_temp >= 0.0);

    Ok(())
}
