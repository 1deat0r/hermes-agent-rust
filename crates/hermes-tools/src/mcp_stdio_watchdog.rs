//! Parent-death watchdog supervisor for stdio MCP subprocesses.
//!
//! PARITY: `tools/mcp_stdio_watchdog.py` @ b9aa928 (whole module; the
//! executable entry lives in `src/bin/mcp_stdio_watchdog.rs`).
//!
//! A stdio MCP server (e.g. `npx -y mcp-remote <url>`) is spawned as a
//! direct child of the Hermes process. Hermes's own teardown path reaps it
//! cleanly on a *graceful* exit — but if the spawning process dies hard
//! (`kill -9`, crash, force-quit), that teardown never runs and the child
//! (plus its descendants) is orphaned, racing to hold the same upstream SSE
//! session. See the upstream module docstring for the full #18451-style
//! failure analysis.
//!
//! Fix: spawn this supervisor instead, which
//!   1. execs the real command as its own child in its own process group
//!      (`start_new_session`, so we can killpg it cleanly);
//!   2. transparently passes stdin/stdout/stderr through — the MCP stdio
//!      protocol talks directly over those pipes, so the supervisor is a
//!      no-op relay, not a bytes-in-the-middle proxy;
//!   3. runs a background thread polling the direct POSIX parent identity:
//!      current `getppid()` vs the parent PID recorded at creation;
//!   4. the instant the original parent is gone, terminates the child's
//!      process group (SIGTERM, grace period, then SIGKILL) and exits.
//!
//! Intentionally thin and standard-library-only so it starts fast and can't
//! itself become a resource leak.
//!
//! Usage:
//!
//! ```text
//! mcp_stdio_watchdog --ppid <original_parent_pid> -- <real_command> <args>...
//! ```

use std::io::Write;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

/// PARITY: `_POLL_INTERVAL_S` (upstream line 26).
pub const POLL_INTERVAL_S: f64 = 2.0;

/// PARITY: `_TERM_GRACE_S` (upstream line 27).
pub const TERM_GRACE_S: f64 = 3.0;

/// The real child's pid, shared with the signal handler (the Rust
/// equivalent of the upstream closure's `proc` capture).
pub static CHILD_PID: AtomicI32 = AtomicI32::new(0);

/// Return whether this process no longer has its original POSIX parent.
///
/// PARITY: `_is_orphaned` (upstream lines 30-32) with the `getppid`
/// injection the upstream default-argument makes testable.
pub fn is_orphaned(original_ppid: i32, current_ppid: i32) -> bool {
    current_ppid != original_ppid
}

/// PARITY: the `real_argv` post-processing in `main` (upstream lines
/// 88-91) — a leading `"--"` separator is dropped.
pub fn strip_separator(args: &[String]) -> Vec<String> {
    let mut real = args.to_vec();
    if real.first().map(String::as_str) == Some("--") {
        real.remove(0);
    }
    real
}

/// The stderr message and exit code for an empty command list.
///
/// PARITY: the `if not real_argv:` arm (upstream lines 92-94) — message on
/// stderr, exit code 2.
pub fn no_command_message() -> &'static str {
    "mcp_stdio_watchdog: no command given after '--'"
}

/// PARITY: `_terminate_process_group` (upstream lines 37-68) — best-effort
/// SIGTERM-then-SIGKILL of the child's process group. SIGKILL only fires
/// after the grace period expires; a clean exit inside the grace window
/// returns immediately. ProcessLookupError / PermissionError degrade to a
/// no-op return.
pub fn terminate_process_group(child_pid: i32) {
    // `os.getpgid` — the child may already be gone.
    let pgid = unsafe { libc::getpgid(child_pid) };
    if pgid < 0 {
        return;
    }
    for sig in [libc::SIGTERM, libc::SIGKILL] {
        if unsafe { libc::killpg(pgid, sig) } != 0 {
            // ProcessLookupError / PermissionError / OSError: give up.
            return;
        }
        // `proc.wait(timeout=_TERM_GRACE_S)` — poll the child's liveness.
        let deadline = Instant::now() + Duration::from_secs_f64(TERM_GRACE_S);
        loop {
            if unsafe { libc::waitpid(child_pid, std::ptr::null_mut(), libc::WNOHANG) } != 0 {
                return;
            }
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// PARITY: `_watchdog_loop` (upstream lines 71-76) — poll the child and the
/// parent identity; the instant the original parent is gone, terminate the
/// child's group and return. `should_exit` is the child's own `poll()`
/// result: `None` while running.
pub fn watchdog_loop(child_pid: i32, original_ppid: i32) {
    loop {
        // `proc.poll() is None` — the child is still running.
        let running = unsafe { libc::waitpid(child_pid, std::ptr::null_mut(), libc::WNOHANG) } == 0;
        if !running {
            return;
        }
        let current_ppid = unsafe { libc::getppid() };
        if is_orphaned(original_ppid, current_ppid) {
            terminate_process_group(child_pid);
            return;
        }
        std::thread::sleep(Duration::from_secs_f64(POLL_INTERVAL_S));
    }
}

/// Reap the child, translating the exit status to a process exit code.
///
/// PARITY: `return proc.wait()` (upstream line 130).
pub fn reap_child(child_pid: i32) -> i32 {
    let mut status = 0;
    loop {
        let rc = unsafe { libc::waitpid(child_pid, &mut status, 0) };
        if rc == child_pid {
            break;
        }
        if rc < 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            return -1;
        }
    }
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        0
    }
}

/// Install SIGTERM/SIGINT forwarding to the child's process group.
///
/// PARITY: the `_forward_shutdown` handler (upstream lines 108-118) — a
/// graceful-shutdown `killpg` of the parent's group no longer reaches the
/// child (it lives in its own group), so the signal must be forwarded;
/// otherwise the watchdog wrap would invert the bug it fixes. Exits with
/// `128 + signum`, async-signal-safely.
pub fn install_signal_forwarding() {
    extern "C" fn handler(signum: i32) {
        let child = CHILD_PID.load(Ordering::SeqCst);
        if child > 0 {
            let pgid = unsafe { libc::getpgid(child) };
            if pgid > 0 {
                unsafe { libc::killpg(pgid, signum) };
            }
        }
        unsafe { libc::_exit(128 + signum) };
    }
    for sig in [libc::SIGTERM, libc::SIGINT] {
        unsafe {
            libc::signal(sig, handler as extern "C" fn(i32) as usize);
        }
    }
}

/// Run the supervisor: spawn `command` in its own session, start the
/// watchdog thread, and block on the child.
///
/// PARITY: `main` (upstream lines 79-133). `stdin`/`stdout`/`stderr` are
/// inherited (transparent pass-through). Returns the child's exit code, or
/// 2 (with [`no_command_message`] printed to stderr) when no command was
/// given.
pub fn run(original_ppid: i32, command: &[String]) -> i32 {
    let real_argv = strip_separator(command);
    if real_argv.is_empty() {
        let mut err = std::io::stderr();
        let _ = writeln!(err, "{}", no_command_message());
        return 2;
    }

    // New process group (`start_new_session=True`) so we can killpg() the
    // whole tree the real command may spawn, without touching our own
    // group or the original parent's.
    let mut spawned = std::process::Command::new(&real_argv[0]);
    spawned.args(&real_argv[1..]);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        spawned.process_group(0);
    }
    let child = match spawned.spawn() {
        Ok(child) => child,
        Err(e) => {
            let mut err = std::io::stderr();
            let _ = writeln!(err, "mcp_stdio_watchdog: failed to spawn: {e}");
            return 127;
        }
    };
    let child_pid = child.id() as i32;
    CHILD_PID.store(child_pid, Ordering::SeqCst);

    install_signal_forwarding();

    // Daemon watchdog thread.
    std::thread::spawn(move || watchdog_loop(child_pid, original_ppid));

    reap_child(child_pid)
}
