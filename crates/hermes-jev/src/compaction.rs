//! Scored verbatim compaction prune.
//!
//! Ports live `agent/compression_scored_prune.py` (fast-jev-compaction):
//! every old-region assistant tool call is paired with its `tool` result
//! by `tool_call_id`, and Jev answers two `noul` questions per pair —
//! keep the call? keep the result verbatim? Winners stay byte-identical,
//! middles keep a result head plus a note, stale calls disappear with
//! their results. Keyless hosts run the offline heuristic (errors stay,
//! huge outputs truncate first) with zero network. Rebuild invariants:
//! no result survives without its call, pinned pairs are never touched,
//! role alternation is preserved (emptied assistant rows become stubs).

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::guardrail::scrub_text;
use crate::questions::{validate_noul_answer, NoulQuestion, Question};
use crate::transport::{post_system_one, JEV_MODEL};

/// First pair + recent floor are never candidates (jev_compact_codex defaults).
pub const PIN_FIRST_PAIRS: usize = 1;
pub const PRESERVE_RECENT_PAIRS: usize = 6;
/// Request envelope overhead around state + questions.
pub const REQUEST_OVERHEAD_TOKENS: usize = 20;
/// Characters of result head kept when only the result is dropped.
pub const DEFAULT_TRUNCATE_HEAD_CHARS: usize = 300;
/// Default keep threshold (live `ScoredPruneConfig.keep_threshold`).
pub const DEFAULT_KEEP_THRESHOLD: f64 = 0.2;
/// Default state / request token budgets.
pub const DEFAULT_MAX_STATE_TOKENS: usize = 25000;
pub const DEFAULT_MAX_REQUEST_TOKENS: usize = 30000;
/// Placeholder the legacy pass writes; scored pairs never re-score it.
pub const PRUNED_PLACEHOLDER: &str = "[Old tool output cleared to save context space]";

/// Compaction context header for the Jev state.
pub const STATE_CONTEXT: &str = "A coding assistant conversation is being compacted to free context. `history` is the whole conversation so far, oldest first; tool outputs are replaced by a short result note and long texts may be abridged. Each question asks whether one tool call, or the full output of that call, still needs to stay in the history verbatim. Whatever is not kept is deleted permanently, but the assistant can always re-run a tool.";

/// Knobs for the scored prune pass. `enabled` defaults OFF.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredPruneConfig {
    pub enabled: bool,
    pub keep_threshold: f64,
    pub max_state_tokens: usize,
    pub max_request_tokens: usize,
    pub truncate_head_chars: usize,
}

impl Default for ScoredPruneConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            keep_threshold: DEFAULT_KEEP_THRESHOLD,
            max_state_tokens: DEFAULT_MAX_STATE_TOKENS,
            max_request_tokens: DEFAULT_MAX_REQUEST_TOKENS,
            truncate_head_chars: DEFAULT_TRUNCATE_HEAD_CHARS,
        }
    }
}

/// One assistant tool call paired with its `tool` result.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallPair {
    /// Short scorer id: t1, t2, ...
    pub id: String,
    pub tool_call_id: String,
    pub tool: String,
    pub args: String,
    pub call_index: usize,
    pub result_index: usize,
    pub result_chars: usize,
    pub is_error: bool,
    pub pinned: bool,
}

/// Threshold verdict per pair.
#[derive(Debug, Clone, PartialEq)]
pub struct CallDecision {
    pub id: String,
    pub tool: String,
    /// keep | drop_result | drop_call
    pub action: String,
    /// pinned | kept | result_dropped | call_dropped
    pub reason: String,
    pub keep_call: f64,
    pub keep_result: f64,
}

/// Keep probabilities for one pair.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeepScores {
    pub keep_call: f64,
    pub keep_result: f64,
}

/// Tokenizer-free estimate ported from fast-jev-compaction (errs high).
///
/// A word costs one token per six letters, a digit half a token, any
/// other symbol nine tenths.
pub fn estimate_tokens(text: &str) -> usize {
    let mut tokens = 0.0_f64;
    let mut chars = text.chars().peekable();
    while let Some(first) = chars.next() {
        if first.is_ascii_alphabetic() {
            let mut len = 1_usize;
            while chars.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                chars.next();
                len += 1;
            }
            tokens += 1.0 + ((len - 1) / 6) as f64;
        } else if first.is_ascii_digit() {
            let mut len = 1_usize;
            while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                chars.next();
                len += 1;
            }
            tokens += len as f64 / 2.0;
        } else if first.is_whitespace() {
            continue;
        } else {
            // One symbol = one 0.9 piece, exactly like the live
            // `[^\sA-Za-z\d]` regex arm (verified bit-equal on probes).
            tokens += 0.9;
        }
    }
    tokens.ceil() as usize
}

/// Pair every assistant tool call with its `tool` result by id.
///
/// Calls without a result are not candidates. Pinned (never pruned):
/// the first pair, the `preserve_recent` newest pairs, and — when a
/// prune `boundary` is passed — any pair touching the protected tail.
pub fn collect_pairs(
    messages: &[Value],
    preserve_recent: usize,
    boundary: Option<usize>,
) -> Vec<ToolCallPair> {
    let mut results: BTreeMap<String, usize> = BTreeMap::new();
    for (idx, msg) in messages.iter().enumerate() {
        if msg.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }
        let cid = msg
            .get("tool_call_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !cid.is_empty() && !results.contains_key(cid) {
            results.insert(cid.to_string(), idx);
        }
    }
    let mut raw: Vec<ToolCallPair> = Vec::new();
    for (call_idx, msg) in messages.iter().enumerate() {
        if msg.get("role").and_then(Value::as_str) != Some("assistant") {
            continue;
        }
        let tcs = msg.get("tool_calls").and_then(Value::as_array);
        let Some(tcs) = tcs else { continue };
        for tc in tcs {
            let Some(tc_obj) = tc.as_object() else {
                continue;
            };
            let cid = tc_obj.get("id").and_then(Value::as_str).unwrap_or("");
            if cid.is_empty() || !results.contains_key(cid) {
                continue;
            }
            let result_idx = results[cid];
            let func = tc_obj.get("function");
            let name = func
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let args = func
                .and_then(|f| f.get("arguments"))
                .map(|a| {
                    if let Some(s) = a.as_str() {
                        s.to_string()
                    } else {
                        a.to_string()
                    }
                })
                .unwrap_or_default();
            let text = messages[result_idx]
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("");
            raw.push(ToolCallPair {
                id: format!("t{}", raw.len() + 1),
                tool_call_id: cid.to_string(),
                tool: name.to_string(),
                args,
                call_index: call_idx,
                result_index: result_idx,
                result_chars: text.len(),
                is_error: messages[result_idx]
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                pinned: false,
            });
        }
    }
    let floor = preserve_recent;
    let len = raw.len();
    for (pos, pair) in raw.iter_mut().enumerate() {
        if pos < PIN_FIRST_PAIRS || pos + floor >= len {
            pair.pinned = true;
        } else if let Some(b) = boundary {
            if pair.call_index >= b || pair.result_index >= b {
                pair.pinned = true;
            }
        }
    }
    raw
}

/// First real user turn; skips harness `<...>` boilerplate preambles.
pub fn first_goal(messages: &[Value]) -> String {
    let mut fallback = String::new();
    for msg in messages {
        if msg.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        let content = msg.get("content").and_then(Value::as_str).unwrap_or("");
        if content.trim().is_empty() {
            continue;
        }
        if fallback.is_empty() {
            fallback = content.trim().chars().take(500).collect();
        }
        if !content.trim().starts_with('<') {
            return content.trim().chars().take(500).collect();
        }
    }
    fallback
}

/// Two `noul` questions per candidate, exactly as compact.ts specifies.
pub fn build_questions(candidates: &[ToolCallPair]) -> BTreeMap<String, Question> {
    let mut questions = BTreeMap::new();
    for pair in candidates {
        questions.insert(
            format!("call_{}", pair.id),
            Question::Noul(NoulQuestion {
                instructions: Value::String(format!(
                    "Tool call {} ({}) should stay in the history: knowing this call was made, with its input, still matters for what the assistant does next",
                    pair.id, pair.tool
                )),
                criteria_true: None,
                criteria_false: None,
            }),
        );
        questions.insert(
            format!("result_{}", pair.id),
            Question::Noul(NoulQuestion {
                instructions: Value::String(format!(
                    "The full output of tool call {} ({}, {} chars) should stay in the history verbatim: the assistant still needs its contents and re-running the tool would not do",
                    pair.id, pair.tool, pair.result_chars
                )),
                criteria_true: None,
                criteria_false: None,
            }),
        );
    }
    questions
}

/// Offline keep probabilities: errors stay, huge outputs go first.
pub fn heuristic_answers(candidates: &[ToolCallPair]) -> BTreeMap<String, KeepScores> {
    let mut answers = BTreeMap::new();
    for pair in candidates {
        let (keep_call, keep_result) = if pair.is_error {
            (0.9, 0.85)
        } else if pair.result_chars > 8000 {
            (0.6, 0.15)
        } else if pair.result_chars > 2000 {
            (0.65, 0.35)
        } else {
            (0.7, 0.6)
        };
        answers.insert(
            pair.id.clone(),
            KeepScores {
                keep_call,
                keep_result,
            },
        );
    }
    answers
}

/// One history entry set as the Jev state JSON (full stage).
pub fn build_state_json(messages: &[Value], pairs: &[ToolCallPair], goal: &str) -> Value {
    let goal_text = if goal.is_empty() {
        first_goal(messages)
    } else {
        goal.to_string()
    };
    let by_call: BTreeMap<usize, Vec<&ToolCallPair>> = {
        let mut map: BTreeMap<usize, Vec<&ToolCallPair>> = BTreeMap::new();
        for pair in pairs {
            map.entry(pair.call_index).or_default().push(pair);
        }
        map
    };
    let mut note_by_result: BTreeMap<usize, &ToolCallPair> = BTreeMap::new();
    for pair in pairs {
        note_by_result.insert(pair.result_index, pair);
    }
    let mut history = Vec::new();
    for (idx, msg) in messages.iter().enumerate() {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
        let text = if let (Some(pair), "tool") = (note_by_result.get(&idx), role) {
            format!(
                "{}, {} chars (omitted)",
                if pair.is_error { "error" } else { "ok" },
                pair.result_chars
            )
        } else {
            msg.get("content")
                .and_then(Value::as_str)
                .map(scrub_text)
                .unwrap_or_default()
        };
        let mut entry = json!({"i": idx, "role": role, "text": text});
        if let Some(calls) = by_call.get(&idx) {
            entry["tool_calls"] = Value::Array(
                calls
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "tool": p.tool,
                            "input": scrub_text(&p.args.chars().take(1000).collect::<String>()),
                            "result": format!("{}, {} chars (omitted)", if p.is_error { "error" } else { "ok" }, p.result_chars),
                        })
                    })
                    .collect(),
            );
        }
        if !text.trim().is_empty() || entry.get("tool_calls").is_some() {
            history.push(entry);
        }
    }
    json!({"context": STATE_CONTEXT, "goal": goal_text, "history": history})
}

/// Validate one batch's `noul` answers; `None` on anything invalid.
pub fn parse_noul_batch(
    batch_ids: &[String],
    answers: &serde_json::Map<String, Value>,
) -> Option<BTreeMap<String, KeepScores>> {
    let mut merged: BTreeMap<String, KeepScores> = BTreeMap::new();
    for qid in batch_ids {
        let (prefix, short) = qid.split_once('_')?;
        let answer = answers.get(qid)?;
        let noul = validate_noul_answer(answer)?.noul;
        let slot = merged.entry(short.to_string()).or_default();
        if prefix == "call" {
            slot.keep_call = noul;
        } else if prefix == "result" {
            slot.keep_result = noul;
        } else {
            return None;
        }
    }
    Some(merged)
}

/// Ask Jev for keep probabilities over every candidate (one request).
/// `None` on keyless/transport/invalid: caller runs the heuristic.
pub fn ask_jev_scores(
    state: &Value,
    candidates: &[ToolCallPair],
) -> Option<BTreeMap<String, KeepScores>> {
    ask_jev_scores_with_model(state, candidates, JEV_MODEL)
}

/// Same as [`ask_jev_scores`] with an explicit model id.
pub fn ask_jev_scores_with_model(
    state: &Value,
    candidates: &[ToolCallPair],
    model: &str,
) -> Option<BTreeMap<String, KeepScores>> {
    if candidates.is_empty() {
        return Some(BTreeMap::new());
    }
    let questions = build_questions(candidates);
    let answers = post_system_one(state, &questions, model).ok()?;
    let ids: Vec<String> = questions.keys().cloned().collect();
    parse_noul_batch(&ids, &answers)
}

/// Threshold keep probabilities into keep / drop_result / drop_call.
///
/// Pinned pairs always keep; missing answers default to keep (1.0).
pub fn decide(
    pairs: &[ToolCallPair],
    answers: &BTreeMap<String, KeepScores>,
    keep_threshold: f64,
) -> Vec<CallDecision> {
    pairs
        .iter()
        .map(|pair| {
            let scores = answers.get(&pair.id);
            let keep_call = scores.map(|s| s.keep_call).unwrap_or(1.0);
            let keep_result = scores.map(|s| s.keep_result).unwrap_or(1.0);
            let (action, reason) = if pair.pinned {
                ("keep", "pinned")
            } else if keep_result >= keep_threshold {
                ("keep", "kept")
            } else if keep_call >= keep_threshold {
                ("drop_result", "result_dropped")
            } else {
                ("drop_call", "call_dropped")
            };
            CallDecision {
                id: pair.id.clone(),
                tool: pair.tool.clone(),
                action: action.to_string(),
                reason: reason.to_string(),
                keep_call,
                keep_result,
            }
        })
        .collect()
}

/// Truncate a dropped result to its head plus a re-run note.
pub fn truncated_result(text: &str, head_chars: usize) -> String {
    if text.len() <= head_chars + 120 {
        return text.to_string();
    }
    let head = if head_chars > 0 {
        format!("{}\n", text.chars().take(head_chars).collect::<String>())
    } else {
        String::new()
    };
    format!(
        "{head}[jev-compaction truncated {} chars; re-run the tool if needed]",
        text.len() - head_chars
    )
}

/// Rebuild the message list from decisions, preserving invariants.
pub fn apply_decisions(
    messages: &[Value],
    pairs: &[ToolCallPair],
    decisions: &[CallDecision],
    truncate_head_chars: usize,
) -> Vec<Value> {
    let by_id: BTreeMap<&str, &ToolCallPair> = pairs.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut dropped_call_ids: Vec<&str> = Vec::new();
    let mut truncated_ids: Vec<&str> = Vec::new();
    for decision in decisions {
        let Some(pair) = by_id.get(decision.id.as_str()) else {
            continue;
        };
        if decision.action == "drop_call" {
            dropped_call_ids.push(pair.tool_call_id.as_str());
        } else if decision.action == "drop_result" {
            truncated_ids.push(pair.tool_call_id.as_str());
        }
    }
    let mut kept = Vec::new();
    for msg in messages {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
        if role == "assistant" && msg.get("tool_calls").and_then(Value::as_array).is_some() {
            let tcs = msg["tool_calls"].as_array().expect("checked");
            let remaining: Vec<Value> = tcs
                .iter()
                .filter(|tc| {
                    let id = tc.get("id").and_then(Value::as_str).unwrap_or("");
                    !dropped_call_ids.contains(&id)
                })
                .cloned()
                .collect();
            if remaining.len() == tcs.len() {
                kept.push(msg.clone());
                continue;
            }
            let mut new_msg = msg.clone();
            new_msg["tool_calls"] = Value::Array(remaining.clone());
            if remaining.is_empty()
                && msg
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .is_empty()
            {
                new_msg["content"] =
                    Value::String("[Pruned tool call(s) to save context space]".to_string());
            }
            kept.push(new_msg);
        } else if role == "tool" {
            let cid = msg
                .get("tool_call_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            if dropped_call_ids.contains(&cid) {
                continue;
            }
            if truncated_ids.contains(&cid) && msg.get("content").and_then(Value::as_str).is_some()
            {
                let mut new_msg = msg.clone();
                let content = msg["content"].as_str().expect("checked");
                new_msg["content"] = Value::String(truncated_result(content, truncate_head_chars));
                kept.push(new_msg);
            } else {
                kept.push(msg.clone());
            }
        } else {
            kept.push(msg.clone());
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_errors_stay() {
        let pair = ToolCallPair {
            id: "t1".to_string(),
            tool_call_id: "c1".to_string(),
            tool: "terminal".to_string(),
            args: String::new(),
            call_index: 1,
            result_index: 2,
            result_chars: 9000,
            is_error: true,
            pinned: false,
        };
        let answers = heuristic_answers(&[pair]);
        assert_eq!(answers["t1"].keep_call, 0.9);
    }
}
