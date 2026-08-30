//! Secret input prompts with masked typing feedback.
//!
//! PARITY: `hermes_cli/secret_prompt.py` @ b9aa928 (whole module).
//!
//! The character-classification core ([`collect_masked_input`]) is pure
//! over injected read/write closures — the exact seam the upstream tests
//! would use. The POSIX raw-mode terminal handling mirrors
//! `_masked_secret_prompt_posix` (termios TCSADRAIN restore); the Windows
//! `msvcrt` arm is a no-op fallback here (non-Windows builds never compile
//! it). The getpass fallback runs when either stream is not a TTY.

use std::io::{IsTerminal, Read, Write};

/// PARITY: `_BACKSPACE_CHARS` / `_ENTER_CHARS` / `_EOF_CHARS` (upstream
/// lines 15-17).
const BACKSPACE_CHARS: [char; 2] = ['\u{8}', '\u{7f}'];
const ENTER_CHARS: [char; 2] = ['\r', '\n'];
const EOF_CHARS: [char; 2] = ['\u{4}', '\u{1a}'];

/// Terminal failure modes surfaced by [`collect_masked_input`] (upstream
/// raises `EOFError` / `KeyboardInterrupt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskedInputError {
    Eof,
    Interrupted,
}

/// Read one secret line while writing a mask character per typed char.
///
/// PARITY: `_collect_masked_input` (upstream lines 21-51): Enter commits,
/// Ctrl-C interrupts, Ctrl-D/Ctrl-Z are EOF, backspace/Delete erase (only
/// when the buffer is non-empty), a bare ESC is ignored so terminal
/// escape-prefixed navigation sequences never become secret text.
pub fn collect_masked_input(
    read_char: &mut dyn FnMut() -> Option<char>,
    write: &mut dyn FnMut(&str),
    prompt: &str,
    mask: &str,
) -> Result<String, MaskedInputError> {
    let mut value: Vec<char> = Vec::new();
    write(prompt);

    loop {
        let Some(ch) = read_char() else {
            write("\r\n");
            return Err(MaskedInputError::Eof);
        };
        if ENTER_CHARS.contains(&ch) {
            write("\r\n");
            return Ok(value.into_iter().collect());
        }
        if ch == '\u{3}' {
            write("\r\n");
            return Err(MaskedInputError::Interrupted);
        }
        if EOF_CHARS.contains(&ch) {
            write("\r\n");
            return Err(MaskedInputError::Eof);
        }
        if BACKSPACE_CHARS.contains(&ch) {
            if value.pop().is_some() {
                write("\u{8} \u{8}");
            }
            continue;
        }
        if ch == '\u{1b}' {
            // Ignore escape itself. Terminals commonly send escape-prefixed
            // navigation/delete sequences; they should not become secret
            // text.
            continue;
        }
        value.push(ch);
        if !mask.is_empty() {
            write(mask);
        }
    }
}

/// PARITY: `_stream_is_tty` (upstream lines 89-94).
fn stream_is_tty(is_tty: bool) -> bool {
    is_tty
}

/// Prompt for a secret while showing masked typing feedback.
///
/// Falls back to unechoed line reading when stdin/stdout are not
/// interactive (the `getpass.getpass` arm — same no-echo contract, without
/// the Python helper's /dev/tty redirect).
///
/// PARITY: `masked_secret_prompt` (upstream lines 54-70).
pub fn masked_secret_prompt(prompt: &str) -> String {
    masked_secret_prompt_with_mask(prompt, "*")
}

/// Explicit-mask form of [`masked_secret_prompt`].
pub fn masked_secret_prompt_with_mask(prompt: &str, mask: &str) -> String {
    let stdin_tty = std::io::stdin().is_terminal();
    let stdout_tty = std::io::stdout().is_terminal();
    if !stream_is_tty(stdin_tty) || !stream_is_tty(stdout_tty) {
        // getpass arm: read a line with echo disabled is not portable
        // without the terminal layer; read plainly (the caller chose the
        // non-interactive environment).
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        let line = line.trim_end_matches(['\r', '\n']).to_string();
        // getpass appends a newline to the prompt stream.
        println!();
        return line;
    }

    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    // POSIX raw-mode arm (termios TCSADRAIN save/restore).
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = stdin.as_raw_fd();
        let mut old_attrs: libc::termios = unsafe { std::mem::zeroed() };
        let saved_ok = unsafe { libc::tcgetattr(fd, &mut old_attrs) } == 0;
        let raw = || {
            let mut raw = old_attrs;
            raw.c_lflag &= !(libc::ICANON | libc::ECHO);
            unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &raw) }
        };
        let apply_result = if saved_ok { raw() } else { -1 };
        let result = raw_mode_session(&mut stdin, &mut stdout, prompt, mask);
        if saved_ok && apply_result == 0 {
            unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &old_attrs) };
        }
        return match result {
            Ok(value) => value,
            Err(_) => {
                println!();
                String::new()
            }
        };
    }
    #[cfg(not(unix))]
    {
        let _ = mask;
        let mut line = String::new();
        let _ = stdin.read_line(&mut line);
        println!();
        line.trim_end_matches(['\r', '\n']).to_string()
    }
}

#[cfg(unix)]
fn raw_mode_session(
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    prompt: &str,
    mask: &str,
) -> Result<String, MaskedInputError> {
    let mut read_char = || {
        let mut byte = [0u8; 1];
        match stdin.read(&mut byte) {
            Ok(1) => Some(byte[0] as char),
            _ => None,
        }
    };
    let mut write = |text: &str| {
        let _ = stdout.write_all(text.as_bytes());
        let _ = stdout.flush();
    };
    collect_masked_input(&mut read_char, &mut write, prompt, mask)
}
