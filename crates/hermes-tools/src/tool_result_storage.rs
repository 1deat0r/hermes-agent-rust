//! Tool result persistence -- preserves large outputs instead of truncating.
//!
//! PARITY: `tools/tool_result_storage.py` @ 5d59366 (whole module, 321
//! lines). Layers against context overflow: (1) per-tool caps inside
//! each tool; (2) `maybe_persist_tool_result` — output over the tool's
//! threshold is persisted and replaced by a preview + path; canonical
//! home is ALWAYS host-side `$HERMES_HOME/cache/spillover/{id}.txt`,
//! remote backends get the translated in-sandbox path (probed for
//! readability) else a copy in the sandbox temp dir; (3)
//! `enforce_turn_budget`.
//!
//! The sandbox write seam (`SandboxExecutor`) replaces Python's
//! `env.execute`: callers (the agent environment layer) supply an
//! executor; the storage module never shells out itself. Host-side
//! paths resolve through `hermes_constants::get_hermes_home` (the
//! `get_spillover_dir` contract); the sandbox-visible translation and
//! readability probe arrive via the [`SandboxEnv`] seam.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use sha2::{Digest, Sha256};

use crate::budget_config::{BudgetConfig, BudgetThreshold};

pub const PERSISTED_OUTPUT_TAG: &str = "<persisted-output>";
pub const PERSISTED_OUTPUT_CLOSING_TAG: &str = "</persisted-output>";
pub const STORAGE_DIR: &str = "/tmp/hermes-results";
pub const SPILLOVER_SUBDIR: &str = "cache/spillover";
pub const SPILLOVER_MAX_AGE_HOURS: i64 = 24;
const BUDGET_TOOL_NAME: &str = "__budget_enforcement__";
const MAX_RESULT_FILENAME_STEM: usize = 120;

/// The env.execute seam: runs a command in the sandbox with stdin data.
/// Returns (returncode, stdout).
pub trait SandboxExecutor {
    fn execute(&self, cmd: &str, timeout_secs: u64, stdin_data: &str) -> SandboxResult;
}

pub struct SandboxResult {
    pub returncode: i32,
    pub output: String,
}

impl SandboxResult {
    pub fn ok() -> Self {
        SandboxResult {
            returncode: 0,
            output: String::new(),
        }
    }

    pub fn ok_with_output(output: &str) -> Self {
        SandboxResult {
            returncode: 0,
            output: output.to_string(),
        }
    }
}

/// Remote-backend surface the spillover layer needs: temp dir, sync,
/// readability probe, stdin mode. Local backends never implement this
/// (host-side writes cover them).
pub trait SandboxEnv {
    /// Sandbox temp dir for fallback writes (`_resolve_storage_dir`).
    fn temp_dir(&self) -> Option<String> {
        None
    }
    /// Force a sync for synced backends (best-effort).
    fn sync(&self) {}
    /// Probe readability of a translated path (the `test -r` probe).
    fn probe_readable(&self, translated_path: &str) -> bool {
        let _ = translated_path;
        false
    }
    /// `"heredoc"` when stdin appends exactly one trailing newline.
    fn stdin_mode(&self) -> &str {
        "pipe"
    }
    /// Translate a host path into the sandbox view (the image tools'
    /// helper); None when untranslatable.
    fn translate_host_path(&self, host_path: &str) -> Option<String> {
        let _ = host_path;
        None
    }
    /// Execute a command, returning (returncode, stdout).
    fn execute(&self, cmd: &str, timeout_secs: u64, stdin_data: &str) -> (i32, String);
}

/// Return $HERMES_HOME/cache/spillover (not created).
///
/// PARITY: `get_spillover_dir` (upstream lines 32-35).
pub fn spillover_dir() -> std::path::PathBuf {
    hermes_constants::get_hermes_home().join(SPILLOVER_SUBDIR)
}

/// Delete spillover files older than `max_age_hours`; returns count
/// removed (same contract as the other `cleanup_*_cache` helpers the
/// gateway housekeeping loop runs hourly).
///
/// PARITY: `cleanup_spillover_cache` (upstream lines 38-54).
pub fn cleanup_spillover_cache(max_age_hours: i64) -> usize {
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        - max_age_hours * 3600;
    let entries = match std::fs::read_dir(spillover_dir()) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let old = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(i64::MAX)
            })
            .unwrap_or(i64::MAX)
            < cutoff;
        if !old {
            continue;
        }
        if std::fs::metadata(&path)
            .map(|m| m.is_file())
            .unwrap_or(false)
            && std::fs::remove_file(&path).is_ok()
        {
            removed += 1;
        }
    }
    removed
}

static PRUNE_LOCK: Mutex<()> = Mutex::new(());
static PRUNED_HOMES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn pruned_homes() -> &'static Mutex<HashSet<String>> {
    PRUNED_HOMES.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Best-effort prune, at most once per process PER PROFILE HOME
/// (CLI-only installs never run housekeeping; a multiplexed gateway
/// must sweep every profile's cache/spillover).
///
/// PARITY: `_prune_spillover_once` (upstream lines 57-71).
/// `home_key` is `hermes_home_key()` (caller-wired until that helper
/// lands in hermes-constants).
pub fn prune_spillover_once(home_key: &str) {
    {
        let mut pruned = pruned_homes().lock().unwrap_or_else(|e| e.into_inner());
        let _guard = PRUNE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if !pruned.insert(home_key.to_string()) {
            return;
        }
    }
    let removed = cleanup_spillover_cache(SPILLOVER_MAX_AGE_HOURS);
    if removed > 0 {
        log::debug!("Pruned {removed} expired spillover file(s)");
    }
}

/// Canonical prune entry: sweeps the current home and records it.
pub fn prune_spillover_for_current_home(home_key: &str) {
    let removed = cleanup_spillover_cache(SPILLOVER_MAX_AGE_HOURS);
    if removed > 0 {
        log::debug!("Pruned {removed} expired spillover file(s)");
    }
    let _ = home_key;
}

/// Write host-side to $HERMES_HOME/cache/spillover; returns path str or
/// None.
///
/// The write is size-verified before the caller tells the model "Full
/// output saved": a partially-flushed file (quota, ENOSPC race) fails
/// closed to the bounded inline truncation instead of referencing an
/// archive that silently lost bytes.
///
/// PARITY: `_write_to_spillover` (upstream lines 86-115).
pub fn write_to_spillover(content: &str, filename: &str) -> Option<String> {
    write_to_spillover_with_mode(content, filename, "pipe")
}

/// Test seam: `"heredoc"` mode accepts exactly one extra trailing byte
/// (mirrors the sandbox heredoc rule for the host path).
pub fn write_to_spillover_with_mode(
    content: &str,
    filename: &str,
    stdin_mode: &str,
) -> Option<String> {
    let data = content.as_bytes();
    let dir = spillover_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        log::warn!("Spillover write failed for {filename}: cannot create dir");
        return None;
    }
    let path = dir.join(filename);
    if std::fs::write(&path, data).is_err() {
        log::warn!("Spillover write failed for {filename}: write error");
        return None;
    }
    let persisted_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let expected = data.len() as u64;
    let lossless =
        persisted_size == expected || (stdin_mode == "heredoc" && persisted_size == expected + 1);
    if !lossless {
        log::warn!(
            "Spillover write for {filename} is not lossless ({persisted_size} bytes on disk, \
             expected {expected}) — discarding archive",
        );
        let _ = std::fs::remove_file(&path);
        return None;
    }
    prune_spillover_for_current_home(&hermes_home_key_fallback());
    Some(path.to_string_lossy().into_owned())
}

fn hermes_home_key_fallback() -> String {
    hermes_constants::get_hermes_home()
        .to_string_lossy()
        .into_owned()
}

/// Path where a remote backend can read `host_path`, or None.
/// Translates via the sandbox seam, forces a sync, then PROBES
/// readability — a persistent container created before spillover
/// joined the mount list lacks the bind mount and must fall back to
/// the in-sandbox write.
///
/// PARITY: `_sandbox_visible_spillover_path` (upstream lines 118-139).
pub fn sandbox_visible_spillover_path(
    host_path: Option<&str>,
    env: &dyn SandboxEnv,
) -> Option<String> {
    let host_path = host_path?;
    let visible = env.translate_host_path(host_path)?;
    env.sync();
    if env.probe_readable(&visible) {
        Some(visible)
    } else {
        None
    }
}

/// Return the best temp-backed storage dir for this environment.
///
/// PARITY: `_resolve_storage_dir` (upstream lines 142-152).
pub fn resolve_storage_dir(env: Option<&dyn SandboxEnv>) -> String {
    if let Some(env) = env {
        if let Some(temp_dir) = env.temp_dir() {
            let trimmed = temp_dir.trim_end_matches('/').to_string();
            if !trimmed.is_empty() {
                return format!("{trimmed}/hermes-results");
            }
            return "/hermes-results".to_string();
        }
    }
    STORAGE_DIR.to_string()
}

/// A single safe filename for a tool result id.
///
/// PARITY: `_safe_result_filename` (upstream lines 154-164).
pub fn safe_result_filename(tool_use_id: &str) -> String {
    let raw_id = if tool_use_id.is_empty() {
        "tool_result"
    } else {
        tool_use_id
    };
    let mut safe_stem: String = raw_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    safe_stem = safe_stem
        .trim_matches(|c: char| c == '.' || c == '_' || c == '-')
        .to_string();
    let changed = safe_stem != raw_id;

    let changed = if safe_stem.is_empty() { true } else { changed };
    let mut safe_stem = if safe_stem.is_empty() {
        "tool_result".to_string()
    } else {
        safe_stem
    };

    if changed || safe_stem.chars().count() > MAX_RESULT_FILENAME_STEM {
        let digest = short_sha256(raw_id);
        let cut: String = safe_stem.chars().take(MAX_RESULT_FILENAME_STEM).collect();
        let cut = cut.trim_end_matches(['.', '_', '-']).to_string();
        safe_stem = format!(
            "{}_{}",
            if cut.is_empty() { "tool_result" } else { &cut },
            digest
        );
    }
    format!("{safe_stem}.txt")
}

fn short_sha256(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex[..12].to_string()
}

/// Truncate at last newline within max_chars. Returns (preview, has_more).
///
/// PARITY: `generate_preview` (upstream lines 167-172). Python slices
/// bytes; on ASCII the boundary is identical. Multibyte content may
/// split a character in Python — the char-boundary version here is
/// strictly safer and byte-identical on ASCII.
pub fn generate_preview(content: &str, max_chars: usize) -> (String, bool) {
    if content.chars().count() <= max_chars {
        return (content.to_string(), false);
    }
    // Byte-wise rfind over the first max_chars chars (upstream cuts
    // bytes; the newline sought is ASCII so the index is a char
    // boundary either way).
    let head: String = content.chars().take(max_chars).collect();
    let last_nl = head.rfind('\n');
    let cut_at = match last_nl {
        Some(idx) if idx > max_chars / 2 => idx + 1,
        _ => head.len(),
    };
    (head[..cut_at].to_string(), true)
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Write content into the sandbox; True on success (round-trip
/// verified). Content goes through stdin, not the command string:
/// Linux MAX_ARG_STRLEN caps one argv element at 128 KB.
///
/// The write is round-trip verified with `wc -c`: a zero exit from
/// `cat` does not prove the bytes landed. A measured mismatch removes
/// the archive and fails closed; an unprobeable backend stays
/// best-effort success. Heredoc-mode backends append exactly one
/// trailing newline, so one extra byte is accepted there.
///
/// PARITY: `_write_to_sandbox` (upstream lines 175-215).
pub fn write_to_sandbox(
    content: &str,
    remote_path: &str,
    executor: &dyn SandboxExecutor,
    stdin_mode: &str,
) -> bool {
    // Full parent directory (upstream os.path.dirname), not just the
    // last segment.
    let storage_dir = remote_path
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("/tmp");
    let cmd = format!(
        "mkdir -p {} && cat > {}",
        shell_quote(storage_dir),
        shell_quote(remote_path)
    );
    let result = executor.execute(&cmd, 30, content);
    if result.returncode != 0 {
        return false;
    }
    let expected = content.as_bytes().len();
    let probe = executor.execute(&format!("wc -c < {}", shell_quote(remote_path)), 15, "");
    if probe.returncode != 0 {
        return true;
    }
    let raw = probe.output.split_whitespace().collect::<Vec<_>>();
    let Some(last) = raw.last() else {
        return true;
    };
    let Ok(persisted_size) = last.parse::<usize>() else {
        return true;
    };
    if persisted_size == expected {
        return true;
    }
    // Only heredoc mode may be +1; payload backends deliver stdin
    // verbatim, so any drift there is a real loss.
    if persisted_size == expected + 1 && stdin_mode == "heredoc" {
        return true;
    }
    log::warn!(
        "Sandbox spill for {remote_path} is not lossless ({persisted_size} bytes in sandbox, \
         expected {expected}) — discarding archive",
    );
    let _ = executor.execute(&format!("rm -f {}", shell_quote(remote_path)), 15, "");
    false
}

/// Build the <persisted-output> replacement block.
///
/// PARITY: `_build_persisted_message` (upstream lines 218-233).
fn build_persisted_message(
    preview: &str,
    has_more: bool,
    original_size: usize,
    file_path: &str,
) -> String {
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
    msg.push_str(
        "Use the read_file tool with offset and limit to access specific sections of this output.\n",
    );
    msg.push_str(
        "Recovery: page through the saved file with read_file (offset/limit) or process it \
         with execute_code — do NOT re-request the same data from the remote API; the full \
         result is already on disk.\n\n",
    );
    msg.push_str(&format!(
        "Preview (first {} chars):\n",
        preview.chars().count()
    ));
    msg.push_str(preview);
    if has_more {
        msg.push_str("\n...");
    }
    msg.push('\n');
    msg.push_str(PERSISTED_OUTPUT_CLOSING_TAG);
    msg
}

/// File path from a <persisted-output> block, or None.
///
/// PARITY: `extract_persisted_path` (upstream lines 236-244).
pub fn extract_persisted_path(content: &str) -> Option<String> {
    if !content.contains(PERSISTED_OUTPUT_TAG) {
        return None;
    }
    for line in content.lines() {
        if let Some(path) = line.strip_prefix("Full output saved to: ") {
            let path = path.trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }
    None
}

/// Layer 2: persist an oversized result, return preview + path.
/// `threshold` overrides `config.resolve_threshold(tool_name)`; falls
/// back to inline truncation when no write location succeeds.
///
/// Always persists host-side first (the canonical home); remote
/// backends reference the mounted path when readable, else write into
/// the sandbox temp dir.
///
/// PARITY: `maybe_persist_tool_result` (upstream lines 247-285).
#[allow(clippy::too_many_arguments)]
pub fn maybe_persist_tool_result(
    content: &str,
    tool_name: &str,
    tool_use_id: &str,
    executor: Option<&dyn SandboxExecutor>,
    sandbox_env: Option<&dyn SandboxEnv>,
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

    let filename = safe_result_filename(tool_use_id);
    let (preview, has_more) = generate_preview(content, config.preview_size);
    let build = |path: &str| build_persisted_message(&preview, has_more, len, path);

    // Always persist host-side first: cache/spillover is the single
    // canonical home.
    let host_path = write_to_spillover(content, &filename);
    let host_side = sandbox_env.is_none();
    if host_side {
        if let Some(path) = host_path {
            log::info!(
                "Persisted large tool result: {tool_name} ({tool_use_id}, {len} chars -> {path})"
            );
            return build(&path);
        }
    } else if let Some(env) = sandbox_env {
        // Remote backend: reference the mounted path when readable, else
        // the sandbox temp dir.
        if let Some(visible) = sandbox_visible_spillover_path(host_path.as_deref(), env) {
            let host_suffix = host_path
                .map(|p| format!(" [host: {p}]"))
                .unwrap_or_default();
            log::info!(
                "Persisted large tool result: {tool_name} ({tool_use_id}, {len} chars -> \
                 {visible}{host_suffix})"
            );
            return build_persisted_message(&preview, has_more, len, &visible);
        }
        let remote_path = format!("{}/{}", resolve_storage_dir(Some(env)), filename);
        if let Some(executor) = executor {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                write_to_sandbox(content, &remote_path, executor, env.stdin_mode())
            })) {
                Ok(true) => {
                    log::info!(
                        "Persisted large tool result: {tool_name} ({tool_use_id}, {len} chars -> \
                         {remote_path})"
                    );
                    return build(&remote_path);
                }
                _ => {
                    log::warn!("Sandbox write failed for {tool_use_id}");
                }
            }
        }
    }
    log::info!("Inline-truncating large tool result: {tool_name} ({len} chars, no sandbox write)");
    format!(
        "{}\n\n[Truncated: tool response was {} chars. Full output could not be saved to sandbox.]",
        preview,
        group_thousands(len)
    )
}

/// Layer 3: enforce aggregate budget across all tool results in a turn.
/// Mutates the list in-place and returns it.
///
/// PARITY: `enforce_turn_budget` (upstream lines 288-311).
pub fn enforce_turn_budget(
    tool_messages: &mut [serde_json::Value],
    executor: Option<&dyn SandboxExecutor>,
    sandbox_env: Option<&dyn SandboxEnv>,
    temp_dir: Option<&str>,
    config: &BudgetConfig,
) {
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    let mut total_size: usize = 0;
    for (i, msg) in tool_messages.iter().enumerate() {
        let content = msg
            .get("content")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
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
            sandbox_env,
            temp_dir,
            config,
            Some(BudgetThreshold::Chars(0)),
        );
        if replacement != content {
            total_size = total_size.saturating_sub(size) + replacement.chars().count();
            if let Some(obj) = tool_messages[idx].as_object_mut() {
                obj.insert(
                    "content".to_string(),
                    serde_json::Value::String(replacement),
                );
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
