// Enforce JSF rules and safety lints
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use std::collections::HashMap;

/// Helper to parse the slot number from the node ID (format: "blade-XX").
pub fn parse_slot_number(node_id: &str) -> Result<u32, String> {
    if !node_id.starts_with("blade-") {
        return Err(format!("Invalid node ID prefix: {}", node_id));
    }
    let num_str = &node_id["blade-".len()..];
    let slot = num_str
        .parse::<u32>()
        .map_err(|e| format!("Failed to parse slot number from '{}': {}", node_id, e))?;
    if slot == 0 {
        return Err(format!("Invalid slot number 0 in node ID: {}", node_id));
    }
    Ok(slot)
}

/// Helper to compute the total active VMs in adjacent slots.
fn get_adjacent_density(slot: u32, chassis_active_vms: &HashMap<u32, u32>) -> u32 {
    let prev = if slot > 1 {
        *chassis_active_vms.get(&(slot - 1)).unwrap_or(&0)
    } else {
        0
    };
    let next = *chassis_active_vms.get(&(slot + 1)).unwrap_or(&0);
    prev + next
}

/// Information needed for a candidate node in tie-breaking resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct TieBreakerCandidate {
    pub node_id: String,
    pub ssd_wear: f64,
}

/// Resolves a tie among multiple candidates deterministically.
/// Returns the winning candidate.
pub fn resolve_tie(
    candidates: &[TieBreakerCandidate],
    chassis_active_vms: &HashMap<u32, u32>,
) -> Result<TieBreakerCandidate, String> {
    if candidates.is_empty() {
        return Err("No candidates provided for tie-breaker".to_string());
    }
    if candidates.len() == 1 {
        let winner = candidates.first().ok_or("No candidate at index 0")?;
        return Ok(winner.clone());
    }

    // Pre-parse slot numbers and calculate densities to avoid error handling inside the sort closure.
    let mut parsed_candidates = Vec::with_capacity(candidates.len());
    for c in candidates {
        let slot = parse_slot_number(&c.node_id)?;
        let adjacent_density = get_adjacent_density(slot, chassis_active_vms);
        parsed_candidates.push((c, slot, adjacent_density));
    }

    // Sort according to the multi-tier tie-breaker logic.
    // Order of preference (ascending/descending):
    // 1. Adjacent slot density (lower is better)
    // 2. SSD wear percentage (lower is better)
    // 3. Physical chassis slot number (lower is better)
    parsed_candidates.sort_by(|a, b| {
        // Compare adjacent densities (lower is better)
        let density_cmp = a.2.cmp(&b.2);
        if density_cmp != std::cmp::Ordering::Equal {
            return density_cmp;
        }

        // Compare SSD wear (lower is better).
        // Since SSD wear is f64, use partial_cmp with a fallback to Equal if NaN.
        let wear_cmp = a.0.ssd_wear.partial_cmp(&b.0.ssd_wear)
            .unwrap_or(std::cmp::Ordering::Equal);
        if wear_cmp != std::cmp::Ordering::Equal {
            return wear_cmp;
        }

        // Compare physical slot numbers (lower is better)
        a.1.cmp(&b.1)
    });

    let (best_candidate, _, _) = parsed_candidates.first().ok_or("Failed to retrieve best candidate")?;
    Ok((*best_candidate).clone())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slot_number_valid() {
        assert_eq!(parse_slot_number("blade-1").unwrap(), 1);
        assert_eq!(parse_slot_number("blade-01").unwrap(), 1);
        assert_eq!(parse_slot_number("blade-16").unwrap(), 16);
    }

    #[test]
    fn test_parse_slot_number_invalid() {
        assert!(parse_slot_number("blade-0").is_err());
        assert!(parse_slot_number("blade-abc").is_err());
        assert!(parse_slot_number("node-01").is_err());
    }

    #[test]
    fn test_resolve_tie_by_adjacent_density() {
        let candidates = vec![
            TieBreakerCandidate {
                node_id: "blade-01".to_string(),
                ssd_wear: 0.10,
            },
            TieBreakerCandidate {
                node_id: "blade-02".to_string(),
                ssd_wear: 0.10,
            },
        ];

        // Slot 1 adjacent slots: 2. Slot 2 active VMs = 5. Total density = 5.
        // Slot 2 adjacent slots: 1, 3. Slot 1 = 0, Slot 3 = 1. Total density = 1.
        let mut chassis_active_vms = HashMap::new();
        chassis_active_vms.insert(2, 5);
        chassis_active_vms.insert(3, 1);

        let winner = resolve_tie(&candidates, &chassis_active_vms).unwrap();
        // blade-02 should win because its adjacent density is 1 vs blade-01's 5
        assert_eq!(winner.node_id, "blade-02");
    }

    #[test]
    fn test_resolve_tie_by_ssd_wear() {
        let candidates = vec![
            TieBreakerCandidate {
                node_id: "blade-01".to_string(),
                ssd_wear: 0.15,
            },
            TieBreakerCandidate {
                node_id: "blade-02".to_string(),
                ssd_wear: 0.05,
            },
        ];

        // Equal adjacent densities (0 active VMs adjacent to both)
        let chassis_active_vms = HashMap::new();

        let winner = resolve_tie(&candidates, &chassis_active_vms).unwrap();
        // blade-02 should win because its SSD wear (0.05) is lower than blade-01's (0.15)
        assert_eq!(winner.node_id, "blade-02");
    }

    #[test]
    fn test_resolve_tie_by_slot_number_fallback() {
        let candidates = vec![
            TieBreakerCandidate {
                node_id: "blade-05".to_string(),
                ssd_wear: 0.10,
            },
            TieBreakerCandidate {
                node_id: "blade-02".to_string(),
                ssd_wear: 0.10,
            },
        ];

        // Equal adjacent densities and equal SSD wear
        let chassis_active_vms = HashMap::new();

        let winner = resolve_tie(&candidates, &chassis_active_vms).unwrap();
        // blade-02 should win because slot 2 < slot 5
        assert_eq!(winner.node_id, "blade-02");
    }
}
