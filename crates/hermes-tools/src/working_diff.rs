//! Working-tree git diff collection shared by the CLI and gateway `/diff`.
//!
//! PARITY: tools/working_diff.py @ b9aa928 (130 LOC, ported 1:1).
//!
//! Modes
//! -----
//! - `working` (default): unstaged changes plus untracked files — what
//!   you'd lose with `git checkout . && git clean -fd`.
//! - `staged`: changes already staged for commit (`git diff --cached`).
//! - `all`: everything since HEAD (staged + unstaged) plus untracked
//!   files.
//!
//! Untracked files are folded in via `git diff --no-index /dev/null
//! <file>` so brand-new files show up as additions rather than being
//! silently invisible (mirrors Codex CLI's `/diff` behaviour).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const GIT_TIMEOUT: u64 = 15;
pub const MAX_UNTRACKED_FILES: usize = 50;

pub const VALID_MODES: [&str; 3] = ["working", "staged", "all"];

fn os_devnull() -> &'static str {
    if cfg!(windows) {
        "nul"
    } else {
        "/dev/null"
    }
}

/// `shutil.which`-equivalent: first executable in PATH, or None.
fn which(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let cand = dir.join(binary);
        if cand.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let exec = fs::metadata(&cand)
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
                if exec {
                    return Some(cand);
                }
            }
            #[cfg(windows)]
            {
                let ext = cand
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let exec = matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com");
                if exec {
                    return Some(cand);
                }
            }
        }
    }
    None
}

#[derive(Debug)]
enum RunError {
    Timeout,
    Io(std::io::Error),
}

/// Run git, returning `(returncode, stdout)`. Mirrors upstream `_run`:
/// `-c core.quotePath=false` is prepended, output is captured (not
/// temp-file captured — that #67964 workaround was specific to env_probe's
/// launcher-shaped children), and Timeout/Io failures surface to the
/// caller as errors rather than a bogus success tuple.
fn _run(args: &[&str], cwd: &Path, timeout: Duration) -> Result<(i32, String), RunError> {
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("core.quotePath=false")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn().map_err(RunError::Io)?;
    // Bound the whole call with a polling wait, killing the child on
    // timeout (equivalent to Python's `timeout=`).
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RunError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Io(e));
            }
        }
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    if let Some(mut out) = child.stdout.take() {
        use std::io::Read as _;
        let _ = out.read_to_string(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        use std::io::Read as _;
        let _ = err.read_to_string(&mut stderr);
    }
    let _ = stderr; // upstream captures but discards stderr
    Ok((status.code().unwrap_or(-1), stdout))
}

/// `git ls-files --others --exclude-standard`.
///
/// The caller owns the outer fail-open/error mapping, matching upstream's
/// `collect_working_diff` try/except around this probe.
fn _untracked_files(cwd: &Path) -> Result<Vec<String>, RunError> {
    match _run(
        &["ls-files", "--others", "--exclude-standard"],
        cwd,
        Duration::from_secs(GIT_TIMEOUT),
    ) {
        Ok((0, out)) => Ok(out
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.to_string())
            .collect()),
        Ok((_code, _out)) => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

/// Render untracked files as new-file diffs via `git diff --no-index`.
fn _untracked_diff(cwd: &Path, files: &[String]) -> String {
    let mut chunks: Vec<String> = Vec::new();
    for rel in files.iter().take(MAX_UNTRACKED_FILES) {
        // --no-index exits 1 when the files differ — that's the success
        // path here, so ignore the return code and keep the output.
        match _run(
            &["diff", "--no-index", "--", os_devnull(), rel],
            cwd,
            Duration::from_secs(GIT_TIMEOUT),
        ) {
            Ok((_code, out)) => {
                if !out.trim().is_empty() {
                    chunks.push(out.trim_end_matches('\n').to_string());
                }
            }
            Err(_) => continue,
        }
    }
    if files.len() > MAX_UNTRACKED_FILES {
        chunks.push(format!(
            "... ({} more untracked files not shown)",
            files.len() - MAX_UNTRACKED_FILES
        ));
    }
    chunks.join("\n")
}

/// Result of [`collect_working_diff`].
///
/// Mirrors the upstream dict shape: `success` is always present; on
/// failure `error` carries the reason; on success `stat` / `diff` /
/// `untracked` are present and `empty` is present (true) only when all
/// three are empty. (A `serde::Serialize` derive was omitted because
/// `serde` is not a direct dependency; add it when a wire consumer lands.)
#[derive(Debug, Clone, PartialEq)]
pub struct WorkingDiffResult {
    pub success: bool,
    pub error: Option<String>,
    pub stat: Option<String>,
    pub diff: Option<String>,
    pub untracked: Option<Vec<String>>,
    pub empty: Option<bool>,
}

impl WorkingDiffResult {
    fn failure(error: String) -> Self {
        WorkingDiffResult {
            success: false,
            error: Some(error),
            stat: None,
            diff: None,
            untracked: None,
            empty: None,
        }
    }
}

/// Collect a git diff of the working directory.
///
/// Returns a success-shaped result on success or a failure-shaped result
/// when git is unavailable / not a repo / mode is unknown. `paths`
/// optionally restricts the diff to specific pathspecs (passed through
/// to git verbatim, so quoted paths with spaces survive).
pub fn collect_working_diff(cwd: &Path, mode: &str, paths: Option<&[String]>) -> WorkingDiffResult {
    if !VALID_MODES.contains(&mode) {
        return WorkingDiffResult::failure(format!(
            "Unknown mode '{mode}'. Use: {}",
            VALID_MODES.join(", ")
        ));
    }

    if which("git").is_none() {
        return WorkingDiffResult::failure("git is not installed or not on PATH.".to_string());
    }

    match _run(
        &["rev-parse", "--is-inside-work-tree"],
        cwd,
        Duration::from_secs(5),
    ) {
        Err(RunError::Timeout) => {
            return WorkingDiffResult::failure("git failed: command timed out".to_string())
        }
        Err(RunError::Io(e)) => return WorkingDiffResult::failure(format!("git failed: {e}")),
        Ok((0, _)) => {}
        Ok((_code, _out)) => {
            return WorkingDiffResult::failure("Not a git repository.".to_string())
        }
    }

    let base_args: Vec<&str> = match mode {
        "staged" => vec!["diff", "--cached"],
        "all" => vec!["diff", "HEAD"],
        _ => vec!["diff"], // working
    };
    let pathspec: Vec<&str> = match paths {
        Some(ps) if !ps.is_empty() => {
            let mut v = vec!["--"];
            v.extend(ps.iter().map(|s| s.as_str()));
            v
        }
        _ => Vec::new(),
    };

    let mut stat_args = base_args.clone();
    stat_args.push("--stat");
    stat_args.extend(pathspec.iter().copied());
    let stat_result = _run(&stat_args, cwd, Duration::from_secs(GIT_TIMEOUT));

    let mut diff_args = base_args.clone();
    diff_args.extend(pathspec.iter().copied());
    let diff_result = _run(&diff_args, cwd, Duration::from_secs(GIT_TIMEOUT * 2));

    let (stat_out, diff_out) = match (stat_result, diff_result) {
        (Ok((_code, stat_out)), Ok((_code2, diff_out))) => (stat_out, diff_out),
        (Err(RunError::Timeout), _) | (_, Err(RunError::Timeout)) => {
            return WorkingDiffResult::failure("git diff timed out.".to_string())
        }
        (Err(RunError::Io(e)), _) => return WorkingDiffResult::failure(format!("git failed: {e}")),
        (_, Err(RunError::Io(e))) => return WorkingDiffResult::failure(format!("git failed: {e}")),
    };

    let mut untracked: Vec<String> = Vec::new();
    let mut untracked_diff = String::new();
    if (mode == "working" || mode == "all")
        && paths.map(|pathspecs| pathspecs.is_empty()).unwrap_or(true)
    {
        match _untracked_files(cwd) {
            Ok(files) => {
                untracked = files;
                if !untracked.is_empty() {
                    untracked_diff = _untracked_diff(cwd, &untracked);
                }
            }
            Err(RunError::Timeout) => {
                return WorkingDiffResult::failure("git diff timed out.".to_string())
            }
            Err(RunError::Io(e)) => return WorkingDiffResult::failure(format!("git failed: {e}")),
        }
    }

    let stat = stat_out.trim().to_string();
    let mut diff = diff_out.trim().to_string();
    if !untracked_diff.is_empty() {
        diff = format!("{diff}\n{untracked_diff}").trim().to_string();
    }

    let empty = stat.is_empty() && diff.is_empty() && untracked.is_empty();
    WorkingDiffResult {
        success: true,
        error: None,
        stat: Some(stat),
        diff: Some(diff),
        untracked: Some(untracked),
        empty: empty.then_some(true),
    }
}
