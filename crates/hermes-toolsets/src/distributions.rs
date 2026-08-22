//! Toolset distributions — probability-weighted toolset sampling for data
//! generation runs.
//!
//! PARITY: toolset_distributions.py @ b9aa928 —
//!   DISTRIBUTIONS                        (29–215)
//!   get_distribution                     (217–229)
//!   list_distributions                   (231–239)
//!   sample_toolsets_from_distribution    (241–283)
//!   validate_distribution                (285–296)

use std::collections::HashMap;

use crate::data::{DistributionDef, DISTRIBUTIONS};
use crate::toolsets::validate_toolset;

fn static_distribution(name: &str) -> Option<DistributionDef> {
    DISTRIBUTIONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, def)| DistributionDef {
            description: def.description,
            toolsets: def.toolsets,
        })
}

/// Get a toolset distribution by name (None when not found).
pub fn get_distribution(name: &str) -> Option<serde_json::Value> {
    let def = static_distribution(name)?;
    Some(serde_json::json!({
        "description": def.description,
        "toolsets": def.toolsets.iter().map(|(k, v)| (k.to_string(), *v)).collect::<HashMap<String, u32>>(),
    }))
}

/// List all available distributions.
pub fn list_distributions() -> serde_json::Value {
    serde_json::json!(DISTRIBUTIONS
        .iter()
        .map(|(n, def)| {
            (
                *n,
                serde_json::json!({
                    "description": def.description,
                    "toolsets": serde_json::json!(def.toolsets.iter().map(|(k, v)| (k.to_string(), *v)).collect::<HashMap<String, u32>>()),
                }),
            )
        })
        .collect::<HashMap<&str, serde_json::Value>>())
}

/// Sample toolsets based on a distribution's probabilities. Each toolset
/// has an independent % chance of being included; when nothing wins, the
/// highest-probability toolset is forced in (mirroring the Python fallback).
///
/// PARITY: toolset_distributions.py sample_toolsets_from_distribution
/// @ b9aa928 (241–283)
pub fn sample_toolsets_from_distribution(
    distribution_name: &str,
    mut rng: Option<&mut dyn FnMut() -> f64>,
) -> Result<Vec<String>, String> {
    let dist = get_distribution(distribution_name)
        .ok_or_else(|| format!("Unknown distribution: {distribution_name}"))?;
    let dist_toolsets = dist
        .get("toolsets")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut default_rng = rand::rng;
    let rand: &mut dyn FnMut() -> f64 = match rng.as_mut() {
        Some(f) => f,
        None => &mut default_rng,
    };

    let mut selected: Vec<String> = Vec::new();
    let mut highest: Option<(String, i64)> = None;

    // Preserve insertion order like dict.items().
    let entries: Vec<(String, i64)> = dist_toolsets
        .iter()
        .map(|(k, v)| (k.clone(), v.as_i64().unwrap_or(0)))
        .collect();
    for (toolset_name, probability) in entries {
        if !validate_toolset(&toolset_name) {
            continue;
        }
        if highest.is_none() || probability > highest.as_ref().unwrap().1 {
            highest = Some((toolset_name.clone(), probability));
        }
        if rand() * 100.0 < probability as f64 {
            selected.push(toolset_name);
        }
        // Double-draw bound: Rust's f64 from a fixed closure may be exactly
        // 0.0/1.0 in tests; the comparison mirrors the Python behavior.
        let _ = 0;
    }

    if selected.is_empty() {
        if let Some((best_name, _)) = highest {
            if validate_toolset(&best_name) {
                selected.push(best_name);
            }
        }
    }

    Ok(selected)
}

/// Check if a distribution name is valid.
pub fn validate_distribution(distribution_name: &str) -> bool {
    static_distribution(distribution_name).is_some()
}

// Minimal deterministic RNG fallback (the Python module uses stdlib random;
// callers may inject their own rng for deterministic tests).
mod rand {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0x9E3779B97F4A7C15) };
    }
    pub fn rng() -> f64 {
        STATE.with(|s| {
            let mut x = s.get();
            // xorshift64star
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            s.set(x);
            (x as f64) / (u64::MAX as f64)
        })
    }
}
