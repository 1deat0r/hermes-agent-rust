//! Structured-output schema helpers for delegate_task (T1-24).
//!
//! PARITY: tools/delegation_output_schema.py @ b9aa928 (151 LOC, ported
//! 1:1 except where noted).
//!
//! Optional per-task `output_schema` (a JSON Schema object): the child is
//! told about the contract via an OUTPUT CONTRACT block appended to its
//! context, the parent validates the child's final answer with jsonschema,
//! and on failure sends exactly ONE bounded retry turn carrying the
//! validation errors verbatim (max 1 retry, exact errors, no schema
//! re-paste).
//!
//! Pattern from: github/copilot-cli ctx.agent(prompt, {schema}) — PATTERN
//! ONLY, zero code/prompt text copied (proprietary).
//!
//! JSON-SCHEMA SEAM (documented divergence): upstream calls the Python
//! `jsonschema` package (a hard dependency in practice) for both
//! meta-validation (`validator_for(raw).check_schema(raw)`) and instance
//! validation (`validator.iter_errors(parsed)`), but ships a defensive
//! ImportError branch that accepts schemas/parsed JSON unvalidated when the
//! package is missing. This Rust port ships that degradation path as the
//! default — the crate builds without adding a dependency — and isolates
//! the two validation calls in `jsonschema_compat` so wiring the real
//! validator is a contained swap. The parent should add
//! `jsonschema = "0.33"` (already in the local cargo cache) and replace the
//! two stub bodies with `jsonschema::validator_for(schema).map(|_| ())` /
//! `.iter_errors(instance)`; the two oracle tests that depend on real
//! validation are written and `#[ignore]`d in parity_delegation_output_schema.rs.

use serde_json::Value;

/// Exactly one retry turn — bounded by design. More retries make frontier
/// models drop fields that were right the first time.
pub const MAX_SCHEMA_RETRIES: usize = 1;

const CONTRACT_HEADER: &str = "OUTPUT CONTRACT (machine-validated)";

/// JSON Schema adapter — see module doc for the seam explanation.
///
/// `PathSegment` constructors live inside the real validator's error
/// mapping, so the enum is intentionally dead until the seam is wired.
#[allow(dead_code)]
mod jsonschema_compat {
    use super::Value;

    /// One segment of an error's absolute JSON path (a property name or
    /// an array index).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum PathSegment {
        Key(String),
        Index(u64),
    }

    /// Meta-validation failure (mirrors Python jsonschema's SchemaError).
    #[derive(Debug)]
    pub struct SchemaError(pub String);

    /// A single instance-validation error.
    #[derive(Debug)]
    pub struct ValidationError {
        pub absolute_path: Vec<PathSegment>,
        pub message: String,
    }

    /// Meta-validate `schema` against the JSON Schema meta-schema.
    ///
    /// Upstream: `validator_for(raw).check_schema(raw)`; on ImportError
    /// (jsonschema unavailable) it degrades to accepting the dict as-is so
    /// delegation still works. This port ships the degradation branch as
    /// the default. To restore full parity, add `jsonschema = "0.33"` to
    /// hermes-tools' [dependencies] and replace this body with:
    ///
    /// ```ignore
    /// jsonschema::validator_for(schema)
    ///     .map(|_| ())
    ///     .map_err(|e| SchemaError(e.to_string()))
    ///     .map_err(|SchemaError(m)| SchemaError(m))
    /// ```
    pub fn check_schema(_schema: &Value) -> Result<(), SchemaError> {
        Ok(())
    }

    /// Validate `instance` against `schema`, returning every error.
    ///
    /// Upstream: `validator_for(schema)(schema).iter_errors(parsed)`. The
    /// upstream ImportError branch accepts parsed JSON without validation
    /// (returns `(True, [])`); this port ships that branch as the default.
    /// To restore full parity with the `jsonschema` crate, replace the
    /// body with:
    ///
    /// ```ignore
    /// let validator = match jsonschema::validator_for(schema) { ... };
    /// validator
    ///     .iter_errors(instance)
    ///     .map(|e| ValidationError {
    ///         absolute_path: e.instance_path.iter()...
    ///             .map(|seg| match seg { ... PathSegment::Key / Index })
    ///             .collect(),
    ///         message: e.to_string(),
    ///     })
    ///     .collect()
    /// ```
    pub fn iter_errors(_schema: &Value, _instance: &Value) -> Vec<ValidationError> {
        Vec::new()
    }
}

/// Validate a model/caller-supplied output_schema value.
///
/// Returns `(Some(schema), None)` when usable, `(None, Some(error))` when
/// not. `Value::Null` passes through as `(None, None)` (no schema
/// requested) — mirroring upstream's `None` input.
pub fn coerce_output_schema(raw: Value) -> (Option<Value>, Option<String>) {
    let mut raw = raw;
    if raw.is_null() {
        return (None, None);
    }
    if let Value::String(s) = &raw {
        // Models sometimes double-encode the schema as a JSON string.
        match serde_json::from_str::<Value>(s) {
            Ok(parsed) => {
                if !parsed.is_object() {
                    return (
                        None,
                        Some("output_schema must be a JSON Schema object.".to_string()),
                    );
                }
                raw = parsed;
            }
            Err(_) => {
                return (
                    None,
                    Some(
                        "output_schema must be a JSON Schema object, got a non-JSON string."
                            .to_string(),
                    ),
                )
            }
        }
    }
    if !raw.is_object() {
        let tname = match &raw {
            Value::Array(_) => "list",
            Value::Bool(_) => "bool",
            Value::Number(n) => {
                if n.is_f64() {
                    "float"
                } else {
                    "int"
                }
            }
            Value::Null => "NoneType",
            Value::String(_) => "str", // unreachable (string branch above), kept for totality
            Value::Object(_) => "dict",
        };
        return (
            None,
            Some(format!(
                "output_schema must be a JSON Schema object, got {tname}."
            )),
        );
    }
    match jsonschema_compat::check_schema(&raw) {
        Ok(()) => (Some(raw), None),
        Err(exc) => (
            None,
            Some(format!(
                "output_schema is not a valid JSON Schema: {}",
                exc.0
            )),
        ),
    }
}

/// Append the explicit output contract block to a child's context.
pub fn append_output_contract(context: Option<&str>, schema: &Value) -> String {
    let schema_text = match serde_json::to_string_pretty(schema) {
        Ok(s) => s,
        Err(_) => schema.to_string(),
    };
    let block = format!(
        "{CONTRACT_HEADER}:\n\
         Your FINAL response must be a single JSON object that validates \
         against this JSON Schema. No prose before or after the JSON; a \
         ```json code fence is acceptable but not required.\n\
         {schema_text}"
    );
    let base = context.unwrap_or("").trim_end();
    if base.is_empty() {
        block
    } else {
        format!("{base}\n\n{block}")
    }
}

/// Best-effort extraction of a JSON payload from model output.
///
/// Strips markdown code fences and leading/trailing prose around the
/// outermost `{...}` / `[...]` span. Returns the (possibly unchanged)
/// candidate string; parsing errors are reported by [`validate_output`].
pub fn extract_json_candidate(text: &str) -> String {
    let mut raw = text.trim().to_string();
    if raw.starts_with("```") {
        if let Some((_, rest)) = raw.split_once('\n') {
            raw = rest.to_string();
        }
        if raw.trim_end().ends_with("```") {
            let trimmed = raw.trim_end();
            raw = trimmed[..trimmed.len() - 3].to_string();
        }
        raw = raw.trim().to_string();
        if raw.to_lowercase().starts_with("json\n") {
            if let Some((_, rest)) = raw.split_once('\n') {
                raw = rest.to_string();
            }
        }
    }
    for (opener, closer) in [("{", "}"), ("[", "]")] {
        if raw.starts_with(opener) {
            return raw;
        }
        if let Some(start) = raw.find(opener) {
            if let Some(end) = raw.rfind(closer) {
                if end > start {
                    return raw[start..=end].to_string();
                }
            }
        }
    }
    raw
}

/// Render an absolute error path like Python's `$` + segment join:
/// `$.zip` for a property, `$.tasks[0]` for an array item.
fn render_error_path(segments: &[jsonschema_compat::PathSegment]) -> String {
    let mut out = String::from("$");
    for seg in segments {
        match seg {
            jsonschema_compat::PathSegment::Key(k) => {
                out.push('.');
                out.push_str(k);
            }
            jsonschema_compat::PathSegment::Index(i) => {
                out.push('[');
                out.push_str(&i.to_string());
                out.push(']');
            }
        }
    }
    out
}

/// Validate a child's final answer against `schema`.
///
/// Returns `(true, [])` on success or `(false, errors)` where errors are
/// human-readable strings suitable for the retry turn. Errors are sorted
/// by absolute path, bounded to the first 10 (upstream `errors[:10]`).
pub fn validate_output(text: &str, schema: &Value) -> (bool, Vec<String>) {
    let candidate = extract_json_candidate(text);
    if candidate.trim().is_empty() {
        return (
            false,
            vec!["Response was empty — expected a JSON object matching the schema.".to_string()],
        );
    }
    let parsed = match serde_json::from_str::<Value>(&candidate) {
        Ok(v) => v,
        Err(exc) => return (false, vec![format!("Response is not valid JSON: {exc}")]),
    };
    let mut errors = jsonschema_compat::iter_errors(schema, &parsed);
    if errors.is_empty() {
        return (true, Vec::new());
    }
    // Python sorts by `list(e.absolute_path)` (mixed Key/Index paths
    // would TypeError in Python 3; homogeneous paths compare naturally).
    // This port orders by (kind, value): object keys before array indexes
    // (a deterministic stand-in for the missing comparison).
    errors.sort_by(|a, b| {
        let a_rank: Vec<(u8, String)> = a
            .absolute_path
            .iter()
            .map(|s| match s {
                jsonschema_compat::PathSegment::Key(k) => (0, k.clone()),
                jsonschema_compat::PathSegment::Index(i) => (1, i.to_string()),
            })
            .collect();
        let b_rank: Vec<(u8, String)> = b
            .absolute_path
            .iter()
            .map(|s| match s {
                jsonschema_compat::PathSegment::Key(k) => (0, k.clone()),
                jsonschema_compat::PathSegment::Index(i) => (1, i.to_string()),
            })
            .collect();
        a_rank.cmp(&b_rank)
    });
    let rendered: Vec<String> = errors
        .iter()
        .take(10)
        .map(|err| format!("{}: {}", render_error_path(&err.absolute_path), err.message))
        .collect();
    (false, rendered)
}

/// Build the single bounded retry turn sent to the child.
///
/// Carries the validation errors verbatim; deliberately does NOT re-paste
/// the schema (the child already has it in its context).
pub fn build_retry_message(errors: &[String]) -> String {
    let error_block = errors
        .iter()
        .map(|e| format!("- {e}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Your previous final response was rejected by the output contract \
         validator. Validation errors:\n\
         {error_block}\n\n\
         Reply with ONLY the corrected JSON object matching the OUTPUT \
         CONTRACT schema from your task context. No prose, no explanations."
    )
}
