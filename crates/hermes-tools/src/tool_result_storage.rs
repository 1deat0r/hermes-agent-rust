//! Tool result persistence — preserves large outputs instead of truncating.
//!
//! PARITY: tools/tool_result_storage.py @ b9aa928 (254 LOC, ported 1:1).
//!
//! The sandbox write seam (`SandboxExecutor`) replaces Python's `env.execute`:
//! callers (the agent environment layer) supply an executor; the storage
//! module never shells out itself.

use crate::budget_config::{BudgetConfig, BudgetThreshold};

pub const PERSISTED_OUTPUT_TAG: &str = "<persisted-output>";
pub const PERSISTED_OUTPUT_CLOSING_TAG: &str = "</persisted-output>";
pub const STORAGE_DIR: &str = "/tmp/hermes-results";
pub const HEREDOC_MARKER: &str = "HERMES_PERSIST_EOF";
const BUDGET_TOOL_NAME: &str = "__budget_enforcement__";
const MAX_RESULT_FILENAME_STEM: usize = 120;

/// The env.execute seam: writes content into the sandbox.
pub trait SandboxExecutor {
    fn execute(&self, cmd: &str, timeout_secs: u64, stdin_data: &str) -> SandboxResult;
}

pub struct SandboxResult {
    pub returncode: i32,
}

impl SandboxResult {
    pub fn ok() -> Self {
        SandboxResult { returncode: 0 }
    }
}

/// Resolve the storage dir for this environment (env temp dir override).
fn resolve_storage_dir(executor: Option<&dyn SandboxExecutor>, temp_dir: Option<&str>) -> String {
    if let Some(temp_dir) = temp_dir {
        let temp_dir = temp_dir.trim_end_matches(['/', '\\']);
        if !temp_dir.is_empty() {
            let base = if temp_dir == "/" { String::new() } else { temp_dir.to_string() };
            return format!("{base}/hermes-results");
        }
    }
    let _ = executor;
    STORAGE_DIR.to_string()
}

/// A single safe filename for a tool result id.
pub fn safe_result_filename(tool_use_id: &str) -> String {
    let raw_id = if tool_use_id.is_empty() { "tool_result" } else { tool_use_id };
    let safe_stem: String = raw_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe_stem = safe_stem.trim_matches(|c: char| c == '.' || c == '_' || c == '-').to_string();
    let changed = safe_stem != raw_id;

    let changed = if safe_stem.is_empty() {
        true
    } else {
        changed
    };
    let mut safe_stem = if safe_stem.is_empty() {
        "tool_result".to_string()
    } else {
        safe_stem
    };

    if changed || safe_stem.chars().count() > MAX_RESULT_FILENAME_STEM {
        let digest = short_sha256(raw_id);
        let cut: String = safe_stem.chars().take(MAX_RESULT_FILENAME_STEM).collect();
        let cut = cut.trim_end_matches(['.', '_', '-']).to_string();
        safe_stem = format!("{}_{}", if cut.is_empty() { "tool_result" } else { &cut }, digest);
    }
    format!("{safe_stem}.txt")
}

fn short_sha256(input: &str) -> String {
    // Minimal FNV-1a 12-hex digest (upstream uses SHA-256[:12]; FNV is
    // deterministic and avoids adding the sha2 crate for filename disambiguation).
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:012x}")[..12].to_string()
}

/// Truncate at last newline within max_chars. Returns (preview, has_more).
pub fn generate_preview(content: &str, max_chars: usize) -> (String, bool) {
    if content.chars().count() <= max_chars {
        return (content.to_string(), false);
    }
    let mut truncated: String = content.chars().take(max_chars).collect();
    if let Some(last_nl) = truncated.rfind('\n') {
        if last_nl > max_chars / 2 {
            truncated = truncated[..last_nl + 1].to_string();
        }
    }
    (truncated, true)
}


fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn write_to_sandbox(content: &str, remote_path: &str, executor: &dyn SandboxExecutor) -> bool {
    let storage_dir = remote_path.rsplit('/').nth(1).unwrap_or("/tmp");
    let cmd = format!(
        "mkdir -p {} && cat > {}",
        shell_quote(storage_dir),
        shell_quote(remote_path)
    );
    let result = executor.execute(&cmd, 30, content);
    result.returncode == 0
}

fn build_persisted_message(preview: &str, has_more: bool, original_size: usize, file_path: &str) -> String {
    let size_kb = original_size as f64 / 1024.0;
    let size_str = if size_kb >= 1024.0 {
        format!("{:.1} MB", size_kb / 1024.0)
    } else {
        format!("{size_kb:.1} KB")
    };
    let mut msg = String::new();
    msg.push_str(PERSISTED_OUTPUT_TAG);
    msg.push('\n');
    msg.push_str(&format!(
        "This tool result was too large ({} characters, {}).\n",
        group_thousands(original_size),
        size_str
    ));
    msg.push_str(&format!("Full output saved to: {file_path}\n"));
    msg.push_str("Use the read_file tool with offset and limit to access specific sections of this output.\n\n");
    msg.push_str(&format!("Preview (first {} chars):\n", preview.chars().count()));
    msg.push_str(preview);
    if has_more {
        msg.push_str("\n...");
    }
    msg.push('\n');
    msg.push_str(PERSISTED_OUTPUT_CLOSING_TAG);
    msg
}

/// Layer 2: persist oversized result into the sandbox, return preview + path.
#[allow(clippy::too_many_arguments)]
pub fn maybe_persist_tool_result(
    content: &str,
    tool_name: &str,
    tool_use_id: &str,
    executor: Option<&dyn SandboxExecutor>,
    temp_dir: Option<&str>,
    config: &BudgetConfig,
    threshold: Option<BudgetThreshold>,
) -> String {
    let effective_threshold = match threshold {
        Some(t) => t,
        None => config.resolve_threshold(tool_name),
    };
    if effective_threshold.is_infinite() {
        return content.to_string();
    }
    let threshold_chars = match effective_threshold {
        BudgetThreshold::Chars(c) => c,
        BudgetThreshold::Infinite => return content.to_string(),
    };
    let len = content.chars().count();
    if len <= threshold_chars {
        return content.to_string();
    }

    let storage_dir = resolve_storage_dir(executor, temp_dir);
    let remote_path = format!("{storage_dir}/{}", safe_result_filename(tool_use_id));
    let (preview, has_more) = generate_preview(content, config.preview_size);

    if let Some(executor) = executor {
        if write_to_sandbox(content, &remote_path, executor) {
            return build_persisted_message(&preview, has_more, len, &remote_path);
        }
    }

    format!(
        "{}\n\n[Truncated: tool response was {} chars. Full output could not be saved to sandbox.]",
        preview,
        group_thousands(len)
    )
}

/// Layer 3: enforce aggregate budget across all tool results in a turn.
pub fn enforce_turn_budget(
    tool_messages: &mut [serde_json::Value],
    executor: Option<&dyn SandboxExecutor>,
    temp_dir: Option<&str>,
    config: &BudgetConfig,
) {
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    let mut total_size: usize = 0;
    for (i, msg) in tool_messages.iter().enumerate() {
        let content = msg.get("content").and_then(serde_json::Value::as_str).unwrap_or("");
        let size = content.chars().count();
        total_size += size;
        if !content.contains(PERSISTED_OUTPUT_TAG) {
            candidates.push((i, size));
        }
    }
    if total_size <= config.turn_budget {
        return;
    }
    candidates.sort_by_key(|(_, size)| std::cmp::Reverse(*size));

    for (idx, size) in candidates {
        if total_size <= config.turn_budget {
            break;
        }
        let content = tool_messages[idx]
            .get("content")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        let tool_use_id = tool_messages[idx]
            .get("tool_call_id")
            .and_then(serde_json::Value::as_str)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("budget_{idx}"));
        let replacement = maybe_persist_tool_result(
            &content,
            BUDGET_TOOL_NAME,
            &tool_use_id,
            executor,
            temp_dir,
            config,
            Some(BudgetThreshold::Chars(0)),
        );
        if replacement != content {
            total_size = total_size.saturating_sub(size) + replacement.chars().count();
            if let Some(obj) = tool_messages[idx].as_object_mut() {
                obj.insert("content".to_string(), serde_json::Value::String(replacement));
            }
        }
    }
}

/// Comma-group a usize for display (Python's `:,` formatting).
fn group_thousands(n: usize) -> String {
    let digits = n.to_string();
    let bytes = digits.as_bytes();
    let mut out = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}
