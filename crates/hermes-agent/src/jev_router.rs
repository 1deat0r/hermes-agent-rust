//! Jev fast/full routing seam for auxiliary side tasks.
//!
//! Live Hermes (`agent/auxiliary_client.py` lines 794-945, newer than pin
//! `5d59366`) lets `auxiliary.<task>.prefer_jev_routing` opt three tasks
//! (`approval`, `compression`, `title_generation`) into a Jev Choice
//! (FAST vs FULL) over a scrubbed size-bucket state. This module is the
//! dependency-safe seam: task eligibility + size buckets live here, the
//! Choice round trip lives in `hermes_jev::router`. Default OFF, fails
//! safe to the legacy route, never throws into the hot path.

use serde_json::Value;

/// Tasks eligible for Jev-assisted fast/full routing (live `_JEV_ROUTED_TASKS`).
pub const JEV_ROUTED_TASKS: &[&str] = &["approval", "compression", "title_generation"];

/// FAST/Full choice ids (live `_validate_jev_route_answer`).
pub const FAST_OPTION: &str = "FAST";
pub const FULL_OPTION: &str = "FULL";

/// Question id for the routing decision.
pub const ROUTE_QUESTION_ID: &str = "route";

/// Routing instructions (live `_ask_jev_route` verbatim intent).
pub const ROUTE_INSTRUCTIONS: &str = "This auxiliary side task can run on the provider's cheap FAST model or must stay on the FULL main model. FAST is for short, tolerant side work (titling, classification, cheap rewrites); FULL is for quality-sensitive work where the fast tier would degrade the result. Choose FAST only when the fast tier is very likely good enough.";

/// Whether `task` opts into Jev routing via its config map.
///
/// PARITY: live `_task_prefers_jev_routing` — eligible task AND truthy
/// `prefer_jev_routing` in `auxiliary.<task>`. Anything malformed is OFF.
pub fn task_prefers_jev_routing(task: &str, task_config: &Value) -> bool {
    if !JEV_ROUTED_TASKS.contains(&task) {
        return false;
    }
    let Some(map) = task_config.as_object() else {
        return false;
    };
    match map.get("prefer_jev_routing") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|v| v != 0.0),
        Some(Value::String(s)) => {
            matches!(
                s.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        }
        _ => false,
    }
}

/// Rough input-size bucket for the route state (live `_jev_route_size_bucket`).
pub fn route_size_bucket(input_chars: Option<i64>) -> &'static str {
    let n = match input_chars {
        Some(n) => n,
        None => return "m",
    };
    if n < 0 {
        return "m";
    }
    if n < 2000 {
        return "s";
    }
    if n < 20000 {
        return "m";
    }
    "l"
}

/// Ask Jev FAST-vs-FULL for one aux task; `None` keeps legacy behaviour.
///
/// Never throws: keyless, transport, and invalid answers all yield `None`.
pub fn ask_jev_route(
    task: &str,
    provider: &str,
    model: &str,
    input_chars: Option<i64>,
) -> Option<String> {
    if !JEV_ROUTED_TASKS.contains(&task) {
        return None;
    }
    let options = [
        hermes_jev::router::RouteOption {
            id: FAST_OPTION.to_string(),
            description: Some("cheap FAST model is very likely good enough".to_string()),
        },
        hermes_jev::router::RouteOption {
            id: FULL_OPTION.to_string(),
            description: Some("quality-sensitive work must stay on FULL".to_string()),
        },
    ];
    let state = serde_json::json!({
        "task": hermes_jev::guardrail::scrub_text(task),
        "input_size": route_size_bucket(input_chars),
        "provider": hermes_jev::guardrail::scrub_text(provider),
        "model": hermes_jev::guardrail::scrub_text(model),
    });
    let mut questions = std::collections::BTreeMap::new();
    questions.insert(
        ROUTE_QUESTION_ID.to_string(),
        hermes_jev::Question::Choice(hermes_jev::ChoiceQuestion {
            instructions: Value::String(ROUTE_INSTRUCTIONS.to_string()),
            criteria: options
                .iter()
                .map(|o| (o.id.clone(), o.description.clone()))
                .collect(),
        }),
    );
    let answers =
        hermes_jev::transport::post_system_one(&state, &questions, hermes_jev::JEV_MODEL).ok()?;
    let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    let parsed =
        hermes_jev::questions::validate_choice_answer(&ids, answers.get(ROUTE_QUESTION_ID)?)?;
    Some(parsed.choice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn routing_defaults_off() {
        assert!(!task_prefers_jev_routing("approval", &json!({})));
        assert!(!task_prefers_jev_routing(
            "approval",
            &json!({"prefer_jev_routing": false})
        ));
        assert!(!task_prefers_jev_routing(
            "unknown_task",
            &json!({"prefer_jev_routing": true})
        ));
    }

    #[test]
    fn routing_opts_in_per_task() {
        assert!(task_prefers_jev_routing(
            "title_generation",
            &json!({"prefer_jev_routing": true})
        ));
        assert!(!task_prefers_jev_routing(
            "title_generation",
            &json!({"prefer_jev_routing": 0})
        ));
    }

    #[test]
    fn size_buckets_match_source() {
        assert_eq!(route_size_bucket(Some(100)), "s");
        assert_eq!(route_size_bucket(Some(5000)), "m");
        assert_eq!(route_size_bucket(Some(50000)), "l");
        assert_eq!(route_size_bucket(None), "m");
        assert_eq!(route_size_bucket(Some(-1)), "m");
    }
}
