//! Output-pattern failure hints for the terminal tool.
//!
//! PARITY: tools/terminal_hints.py @ b9aa928 (170 LOC, ported 1:1).
//!
//! When a command exits non-zero, the raw stderr often confuses models into
//! wasted diagnostic turns (e.g. retrying `python` when only `python3` exists,
//! or re-sending a gh field list that the installed gh doesn't support).
//!
//! This module extends the exit-code semantics table in `terminal_tool` with
//! an *output-pattern* tier: a bounded scan of the command output that maps
//! well-known failure shapes to one short, actionable recovery hint.
//!
//! Design rules (kept from upstream):
//! * Only fires on non-zero exit codes — never annotate success.
//! * At most ONE hint per result, first match wins; patterns are ordered by
//!   observed frequency in production trajectories (state.db mining, Aug 2026).
//! * Scans only the first `SCAN_CHARS` characters of output.
//! * Hints state the *next action*, not a diagnosis essay.
//! * Pure function, no I/O, no config reads — trivially unit-testable.
//!
//! UPSTREAM TEST GAP: `TestTerminalIntegration` exercises
//! `terminal_tool._interpret_exit_code` wiring; that module is not ported yet,
//! so the two wiring tests are deferred (see parity_terminal_hints.rs).
//! `test_hint_functions_cannot_crash_annotate` patches `_OUTPUT_HINTS` with a
//! raising lambda — Rust has no monkey-patching; the hint fns are pure and
//! cannot raise, so the try/except guard is structurally unnecessary (the
//! dispatcher still calls each hint independently and moves on).

use once_cell::sync::Lazy;
use regex::{Regex, RegexBuilder};

/// Bounded scan window: error headers appear early; deep output is noise.
///
/// PARITY: `_SCAN_CHARS` (4000).
pub const SCAN_CHARS: usize = 4000;

type HintFn = fn(&str, &str) -> Option<String>;

/// ~9,175x: gh CLI version drift — model asks for fields the installed
/// gh doesn't know. gh already prints the valid field list.
fn hint_gh_unknown_json_field(_command: &str, output: &str) -> Option<String> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r#"Unknown JSON field: "?(\w+)"#).expect("terminal_hints regex")
    });
    let m = RE.captures(output)?;
    let field = m.get(1)?.as_str();
    Some(format!(
        "The installed gh does not support the JSON field '{field}'. The valid field list is printed in the output above — retry using only fields from that list."
    ))
}

/// ~1,010x generic; 837x of them are bare `python` on python3-only distros.
fn hint_command_not_found(_command: &str, output: &str) -> Option<String> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:bash: line \d+: |bash: |sh: \d*:? ?)?([\w.+-]+): command not found")
            .expect("terminal_hints regex")
    });
    let m = RE.captures(output)?;
    let missing = m.get(1)?.as_str();
    if missing == "python" {
        return Some(
            "This system has no bare `python` — use `python3`, or the project venv's interpreter (e.g. .venv/bin/python)."
                .to_string(),
        );
    }
    if missing == "pip" {
        return Some(
            "This system has no bare `pip` — use `pip3`, `python3 -m pip`, or the project venv's pip (e.g. .venv/bin/pip)."
                .to_string(),
        );
    }
    Some(format!(
        "`{missing}` is not installed or not on PATH. Verify with `which {missing}`; install it or use an absolute path instead of retrying the same command."
    ))
}

/// ~739x: almost always a venv-activation slip, not a missing dependency.
fn hint_module_not_found(_command: &str, output: &str) -> Option<String> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:ModuleNotFoundError|ImportError): No module named '?([\w.]+)")
            .expect("terminal_hints regex")
    });
    let m = RE.captures(output)?;
    let name = m.get(1)?.as_str();
    Some(format!(
        "Python cannot import '{name}'. Most often the wrong interpreter is running: activate the project venv (e.g. `source .venv/bin/activate`) or invoke its python directly. Only pip install if the package is genuinely absent from that venv."
    ))
}

/// ~1,172x: models sometimes re-run the failing merge/rebase verbatim.
fn hint_merge_conflict(_command: &str, output: &str) -> Option<String> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        // Upstream compiles with re.M — `^CONFLICT ` anchors at line starts.
        RegexBuilder::new(r"^CONFLICT |Automatic merge failed|needs merge")
            .multi_line(true)
            .build()
            .expect("terminal_hints regex")
    });
    if !RE.is_match(output) {
        return None;
    }
    Some(
        "Git merge conflict. Do not retry this command. Resolve the conflicted files listed above (edit, then `git add`), then continue (`git rebase --continue` / commit the merge) — or abort with `--abort`."
            .to_string(),
    )
}

/// ~633x: branch/dir/file already exists → retrying unchanged always fails.
fn hint_already_exists(_command: &str, output: &str) -> Option<String> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?:fatal|error):.*?'([^']+)' already exists").expect("terminal_hints regex")
    });
    let m = RE.captures(output)?;
    let name = m.get(1)?.as_str();
    Some(format!(
        "'{name}' already exists — retrying unchanged will keep failing. Reuse it, choose another name, or delete it first if it is genuinely stale."
    ))
}

/// ~133x: immediate retries burn turns; the limit is time-based.
fn hint_gh_rate_limit(_command: &str, output: &str) -> Option<String> {
    if !output.contains("API rate limit") && !output.contains("was submitted too quickly") {
        return None;
    }
    Some(
        "GitHub API rate limit hit — immediate retries will keep failing. Continue with other work and retry this operation later."
            .to_string(),
    )
}

fn hint_permission_denied(_command: &str, output: &str) -> Option<String> {
    if !output.contains("Permission denied") && !output.contains("EACCES") {
        return None;
    }
    Some(
        "Permission denied. Check ownership/mode of the target path (`ls -la`); prefer a user-writable location. Only escalate to sudo if the task genuinely requires it."
            .to_string(),
    )
}

/// Ordered by production frequency — first match wins.
///
/// PARITY: `_OUTPUT_HINTS` @ tools/terminal_hints.py (order is part of the
/// contract: upstream orders by observed frequency in production trajectories).
const OUTPUT_HINTS: [HintFn; 7] = [
    hint_gh_unknown_json_field,
    hint_merge_conflict,
    hint_command_not_found,
    hint_module_not_found,
    hint_already_exists,
    hint_gh_rate_limit,
    hint_permission_denied,
];

/// Exit-code-only hints for codes the semantics table in terminal_tool does
/// not cover per-command. Checked after output patterns.
///
/// PARITY: `_EXIT_CODE_HINTS`.
fn exit_code_hint(exit_code: i32) -> Option<&'static str> {
    match exit_code {
        126 => Some(
            "Exit 126: the file was found but is not executable — `chmod +x` it or invoke it via its interpreter (e.g. `bash script.sh`).",
        ),
        137 => Some(
            "Exit 137: the process was SIGKILLed — usually out-of-memory or an external kill. Reduce memory use or check `dmesg | tail` before retrying.",
        ),
        124 => Some(
            "Exit 124: the command hit its timeout. Raise timeout= (foreground max 600s) or run it with background=true and notify_on_complete=true.",
        ),
        _ => None,
    }
}

/// Return one short recovery hint for a failed command, or None.
///
/// Args mirror upstream `annotate_failure`:
/// - `command`: The command string that ran.
/// - `exit_code`: Its exit code (non-zero for failures).
/// - `output`: Combined stdout/stderr as returned to the model.
///
/// Only the first `SCAN_CHARS` characters of output are examined and at
/// most one hint is returned. Returns None for exit_code == 0.
pub fn annotate_failure(command: &str, exit_code: i32, output: &str) -> Option<String> {
    if exit_code == 0 {
        return None;
    }
    let window: String = output.chars().take(SCAN_CHARS).collect();
    if !window.is_empty() {
        for hint in OUTPUT_HINTS {
            if let Some(hint) = hint(command, &window) {
                return Some(hint);
            }
        }
    }
    exit_code_hint(exit_code).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_conflict_regex_is_multiline() {
        // Upstream compiles this pattern with re.M; the `^CONFLICT ` branch must
        // anchor at line starts, not just the string start.
        let out = "Auto-merging a.py\nCONFLICT (content): Merge conflict in a.py\nAutomatic merge failed; fix conflicts and then commit the result.";
        assert!(hint_merge_conflict("git merge feature", out)
            .unwrap()
            .contains("Do not retry"));
    }

    #[test]
    fn output_hints_ordering_first_match_wins() {
        let out = "CONFLICT (content): Merge conflict in a.py\npython: command not found";
        // merge_conflict is listed before command_not_found upstream.
        let hint = annotate_failure("git merge x && python t.py", 1, out).unwrap();
        assert!(hint.to_lowercase().contains("conflict"));
        assert!(!hint.contains("python3"));
    }
}
