//! Config.yaml read helpers for the state crate.
//!
//! Fail-open by design: a missing/unparseable config or a malformed
//! `database:` section yields defaults (mirrors hermes_cli.config
//! load_config_readonly + cfg_get behavior surfaced in resolve_journal_mode /
//! apply_database_pragmas).

use std::collections::BTreeMap;
use std::path::Path;

/// Load config.yaml as a mapping (fail-open → empty mapping).
///
/// PARITY: hermes_cli.config.load_config_readonly (the observable slice the
/// state module consumes) — missing file → empty, unparseable YAML → empty,
/// non-dict root → empty.
pub fn load_config_value(path: &Path) -> Option<BTreeMap<String, serde_yaml::Value>> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text).ok()?;
    let mapping = value.as_mapping()?;
    let mut out = BTreeMap::new();
    for (k, v) in mapping {
        let key = match k.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        out.insert(key, v.clone());
    }
    Some(out)
}

/// `database.journal_mode` from config.yaml (fail-open → None).
///
/// PARITY: resolve_journal_mode's config read @ hermes_state.py 740–766.
pub fn raw_database_journal_mode(path: &Path) -> Option<String> {
    let cfg = load_config_value(path)?;
    let database = cfg.get("database")?.as_mapping()?;
    let raw = database.get(serde_yaml::Value::String("journal_mode".to_string()))?;
    raw.as_str().map(str::to_string)
}
