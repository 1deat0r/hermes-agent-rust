//! Toolset distributions — probability-weighted toolset sampling for data
//! generation runs.
//!
//! PARITY: toolset_distributions.py @ 5d59366 (whole module) —
//!   DISTRIBUTIONS                          (18–46, data.rs)
//!   get_distribution                       (49–51)
//!   list_distributions                     (54–55)
//!   validate_distribution                  (58–59)
//!   _entry_members                         (62–64)
//!   sample_toolsets_from_distribution      (67–93)
//!   print_distribution_info                (96–106)
//!
//! PORT SEAMS (documented divergences):
//! - `inject_distribution` / `remove_distribution` are test-support analogs
//!   of the upstream oracle tests doing `monkeypatch.setitem(DISTRIBUTIONS,
//!   ...)` (test_toolset_distributions.py lines 83–99 / 95–99); production
//!   upstream has no insert API — its module dict is simply mutable.
//! - `sample_toolsets_from_distribution_verbose` returns the warning lines
//!   the upstream `sample` prints to stdout (capsys-checked in the oracle);
//!   `sample_toolsets_from_distribution` prints them and returns only the
//!   selection, matching upstream's observable behavior.
//!
//! Grouped "+" entries (upstream lines 62–93, #64503): a key like
//! `browser+search` rolls once and selects (or skips) every member together.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::data::{DistributionDef, DISTRIBUTIONS};
use crate::toolsets::validate_toolset;

/// Runtime distribution overlay (test-support; see PORT SEAMS).
type DistOverlay = HashMap<String, (String, Vec<(String, u32)>)>;

fn overlay() -> &'static Mutex<DistOverlay> {
    static OVERLAY: OnceLock<Mutex<DistOverlay>> = OnceLock::new();
    OVERLAY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Test-support: insert/replace a distribution (upstream tests
/// `monkeypatch.setitem(DISTRIBUTIONS, ...)`).
pub fn inject_distribution(name: &str, description: &str, entries: Vec<(String, u32)>) {
    overlay()
        .lock()
        .expect("dist overlay")
        .insert(name.to_string(), (description.to_string(), entries));
}

/// Test-support: remove an injected distribution (test cleanup — the
/// upstream `monkeypatch` fixture undoes itself).
pub fn remove_distribution(name: &str) {
    overlay().lock().expect("dist overlay").remove(name);
}

/// One distribution as (description, ordered toolset entries).
fn dist_entries(name: &str) -> Option<(String, Vec<(String, u32)>)> {
    if let Some(entry) = overlay().lock().expect("dist overlay").get(name) {
        return Some(entry.clone());
    }
    let def: &DistributionDef = DISTRIBUTIONS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, d)| d)?;
    Some((
        def.description.to_string(),
        def.toolsets
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect(),
    ))
}

fn entries_to_json(entries: &[(String, u32)]) -> Value {
    let mut map = serde_json::Map::new();
    for (k, v) in entries {
        map.insert(k.clone(), serde_json::json!(v));
    }
    Value::Object(map)
}

use serde_json::Value;

/// Get a toolset distribution by name (None when not found) — upstream
/// `get_distribution` (lines 49–51).
pub fn get_distribution(name: &str) -> Option<Value> {
    let (description, entries) = dist_entries(name)?;
    Some(serde_json::json!({
        "description": description,
        "toolsets": entries_to_json(&entries),
    }))
}

/// List all available distributions (a fresh copy each call) — upstream
/// `list_distributions` (lines 54–55).
pub fn list_distributions() -> Value {
    let mut out = serde_json::Map::new();
    for (name, def) in DISTRIBUTIONS {
        let entries: Vec<(String, u32)> = def
            .toolsets
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect();
        out.insert(
            name.to_string(),
            serde_json::json!({
                "description": def.description,
                "toolsets": entries_to_json(&entries),
            }),
        );
    }
    for (name, (description, entries)) in overlay().lock().expect("dist overlay").iter() {
        out.insert(
            name.clone(),
            serde_json::json!({
                "description": description,
                "toolsets": entries_to_json(entries),
            }),
        );
    }
    Value::Object(out)
}

/// Check if a distribution name is valid — upstream
/// `validate_distribution` (lines 58–59).
pub fn validate_distribution(distribution_name: &str) -> bool {
    dist_entries(distribution_name).is_some()
}

/// Toolsets named by a distribution entry: a bare name or a "+"-grouped
/// compound — upstream `_entry_members` (lines 62–64).
fn entry_members(entry: &str) -> Vec<String> {
    entry
        .split('+')
        .map(|name| name.trim().to_string())
        .collect()
}

fn default_rng() -> f64 {
    // Deterministic-enough xorshift fallback; callers inject their own rng
    // for exact rolls (the Python module uses stdlib random).
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0x9E3779B97F4A7C15) };
    }
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        s.set(x);
        (x as f64) / (u64::MAX as f64)
    })
}

/// Sample + warnings — the upstream body (lines 67–93) with `print`
/// captured into the warning list instead of stdout (see PORT SEAMS).
fn sample_inner(
    distribution_name: &str,
    mut rng: Option<&mut dyn FnMut() -> f64>,
) -> Result<(Vec<String>, Vec<String>), String> {
    let (_description, entries) = dist_entries(distribution_name)
        .ok_or_else(|| format!("Unknown distribution: {distribution_name}"))?;
    let mut default = default_rng;
    let roll: &mut dyn FnMut() -> f64 = match rng.as_mut() {
        Some(f) => *f,
        None => &mut default,
    };

    let mut selected: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for (entry, probability) in &entries {
        let members = entry_members(entry);
        let invalid: Vec<&String> = members
            .iter()
            .filter(|name| !validate_toolset(name))
            .collect();
        if !invalid.is_empty() {
            // PARITY: upstream prints only the INVALID members, joined
            // with '+' (line 85), and skips the whole entry (elif chain).
            let joined: Vec<&str> = invalid.iter().map(|s| s.as_str()).collect();
            warnings.push(format!(
                "⚠️  Warning: Toolset '{}' in distribution '{}' is not valid",
                joined.join("+"),
                distribution_name
            ));
        } else if roll() * 100.0 < (*probability as f64) {
            selected.extend(members);
        }
    }
    // PARITY: fallback picks the FIRST maximal-probability entry over ALL
    // entries (upstream `max(..., key=...)`, line 89) and extends only when
    // every member validates (silently — no warning in this arm).
    if selected.is_empty() && !entries.is_empty() {
        let mut best: Option<&(String, u32)> = None;
        for candidate in &entries {
            match best {
                None => best = Some(candidate),
                Some(b) if candidate.1 > b.1 => best = Some(candidate),
                _ => {}
            }
        }
        if let Some((entry, _)) = best {
            let members = entry_members(entry);
            if members.iter().all(|m| validate_toolset(m)) {
                selected.extend(members);
            }
        }
    }
    Ok((selected, warnings))
}

/// Sample toolset names, each entry included independently with its %
/// probability — upstream `sample_toolsets_from_distribution` (lines 67–93).
///
/// A "+"-grouped compound rolls once for all members; invalid members skip
/// the whole entry (warning printed, matching upstream stdout); when
/// nothing was rolled the highest-probability entry is forced in if all
/// its members validate. Raises the upstream `ValueError` text for an
/// unknown distribution.
pub fn sample_toolsets_from_distribution(
    distribution_name: &str,
    rng: Option<&mut dyn FnMut() -> f64>,
) -> Result<Vec<String>, String> {
    let (selected, warnings) = sample_inner(distribution_name, rng)?;
    for warning in warnings {
        println!("{warning}");
    }
    Ok(selected)
}

/// Like [`sample_toolsets_from_distribution`] but returns the warning lines
/// instead of printing them (test seam for the oracle's capsys check — see
/// PORT SEAMS).
pub fn sample_toolsets_from_distribution_verbose(
    distribution_name: &str,
    rng: Option<&mut dyn FnMut() -> f64>,
) -> Result<(Vec<String>, Vec<String>), String> {
    sample_inner(distribution_name, rng)
}

/// Print a distribution's description and toolset probabilities (highest
/// first) — upstream `print_distribution_info` (lines 96–106).
pub fn print_distribution_info(distribution_name: &str) {
    let Some((description, entries)) = dist_entries(distribution_name) else {
        println!("❌ Unknown distribution: {distribution_name}");
        return;
    };
    println!("\n📊 Distribution: {distribution_name}");
    println!("   Description: {description}");
    println!("   Toolsets:");
    let mut sorted = entries;
    sorted.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    for (toolset, prob) in sorted {
        println!("     • {toolset:15} : {prob:3}% chance");
    }
}
