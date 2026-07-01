// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use aetherd::bidder::{Bidder, BidderConfig};
use aetherd::telemetry::SystemMetrics;

fn test_metrics() -> SystemMetrics {
    SystemMetrics {
        load_one: 0.5,
        load_five: 0.5,
        load_fifteen: 0.5,
        mem_total: 16 * 1024 * 1024 * 1024,       // 16 GB
        mem_available: 8 * 1024 * 1024 * 1024,    // 8 GB
        disk_total: 200 * 1024 * 1024 * 1024,     // 200 GB
        disk_available: 100 * 1024 * 1024 * 1024, // 100 GB
        nvme_temp: 35.0,
        cpu_cores: 4,
        active_migrations: 0,
    }
}

#[test]
fn test_bidding_resource_thresholds_evaluation() -> Result<(), Box<dyn std::error::Error>> {
    let bidder = Bidder::new(BidderConfig::default());
    let metrics = test_metrics();

    // 1. Normal bidding: requested resources fit well
    let normal_score =
        bidder.calculate_bid(&metrics, 2, 4 * 1024 * 1024 * 1024, 20 * 1024 * 1024 * 1024);
    assert!((1..=1000).contains(&normal_score));

    // 2. Memory overrun: request 12GB memory when only 8GB is free
    let mem_overrun_score = bidder.calculate_bid(
        &metrics,
        2,
        12 * 1024 * 1024 * 1024,
        20 * 1024 * 1024 * 1024,
    );
    assert_eq!(mem_overrun_score, -1);

    // 3. Disk overrun: request 120GB disk when only 100GB is free
    let disk_overrun_score = bidder.calculate_bid(
        &metrics,
        2,
        4 * 1024 * 1024 * 1024,
        120 * 1024 * 1024 * 1024,
    );
    assert_eq!(disk_overrun_score, -1);

    // 4. Overheated NVMe: temp is 82C (critical is 80C)
    let mut hot_metrics = metrics.clone();
    hot_metrics.nvme_temp = 82.0;
    let hot_score = bidder.calculate_bid(
        &hot_metrics,
        2,
        4 * 1024 * 1024 * 1024,
        20 * 1024 * 1024 * 1024,
    );
    assert_eq!(hot_score, -1);

    // 5. Heavy CPU load: 1-minute load avg is 9.0 (ratio 2.25 > 2.0 max limit)
    let mut heavy_cpu_metrics = metrics.clone();
    heavy_cpu_metrics.load_one = 9.0;
    let heavy_cpu_score = bidder.calculate_bid(
        &heavy_cpu_metrics,
        2,
        4 * 1024 * 1024 * 1024,
        20 * 1024 * 1024 * 1024,
    );
    assert_eq!(heavy_cpu_score, -1);

    Ok(())
}
