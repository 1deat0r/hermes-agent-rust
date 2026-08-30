//! Parent-death watchdog for a stdio MCP subprocess.
//!
//! PARITY: `tools/mcp_stdio_watchdog.py` `__main__` @ b9aa928 — the
//! `python3 -m tools.mcp_stdio_watchdog --ppid <pid> -- <command>...`
//! invocation as a native binary.
//!
//! All logic lives in `hermes_tools::mcp_stdio_watchdog` for testability.

use std::io::Write;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // `parser.add_argument("--ppid", type=int, required=True)` followed by
    // `nargs=argparse.REMAINDER` — find --ppid and treat the rest as the
    // command.
    let mut original_ppid: Option<i32> = None;
    let mut command: Vec<String> = Vec::new();
    let mut iter = argv.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--ppid" {
            match iter.peek().and_then(|v| v.parse::<i32>().ok()) {
                Some(value) => {
                    original_ppid = Some(value);
                    iter.next();
                }
                None => {
                    let mut err = std::io::stderr();
                    let _ = writeln!(
                        err,
                        "{}",
                        "mcp_stdio_watchdog: error: argument --ppid: expected one argument"
                    );
                    std::process::exit(2);
                }
            }
        } else if let Some(rest) = arg.strip_prefix("--ppid=") {
            match rest.parse::<i32>() {
                Ok(value) => original_ppid = Some(value),
                Err(_) => std::process::exit(2),
            }
        } else {
            command.push(arg.clone());
            command.extend(iter.cloned());
            break;
        }
    }
    let Some(original_ppid) = original_ppid else {
        let mut err = std::io::stderr();
        let _ = writeln!(
            err,
            "{}",
            "mcp_stdio_watchdog: error: the following arguments are required: --ppid"
        );
        std::process::exit(2);
    };

    let code = hermes_tools::mcp_stdio_watchdog::run(original_ppid, &command);
    std::process::exit(code);
}
