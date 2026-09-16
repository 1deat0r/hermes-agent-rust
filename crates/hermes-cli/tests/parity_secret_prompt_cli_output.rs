//! Parity tests for `hermes_cli/secret_prompt.py` and
//! `hermes_cli/cli_output.py` @ b9aa928. Upstream has no dedicated test
//! files (missing-test gap, noted in the ledger); cases derive from the
//! upstream code as oracle over the injected read/write seams.

use std::cell::RefCell;
use std::rc::Rc;

use hermes_cli::cli_output::{prompt_from, prompt_yes_no_with};
use hermes_cli::secret_prompt::{collect_masked_input, MaskedInputError};

/// Scripted character source: pops from a queue, returning None (EOF-style
/// read failure) when exhausted.
fn scripted(chars: &[Option<char>]) -> impl FnMut() -> Option<char> + '_ {
    let queue: RefCell<VecDeque<Option<char>>> = RefCell::new(chars.iter().copied().collect());
    move || queue.borrow_mut().pop_front().flatten()
}

use std::collections::VecDeque;

#[test]
fn typed_characters_collect_with_mask_feedback() {
    let chars = vec![
        Some('s'),
        Some('e'),
        Some('c'),
        Some('r'),
        Some('e'),
        Some('t'),
        Some('\n'),
    ];
    let output: Rc<RefCell<String>> = Rc::default();
    let out_cb = Rc::clone(&output);
    let mut write = move |text: &str| out_cb.borrow_mut().push_str(text);

    let value = collect_masked_input(&mut scripted(&chars), &mut write, "pass: ", "*").unwrap();
    assert_eq!(value, "secret");
    let out = output.borrow().clone();
    assert_eq!(out, "pass: ******\r\n");
}

#[test]
fn enter_commits_and_writes_crlf() {
    let chars = vec![Some('\n')];
    let output: Rc<RefCell<String>> = Rc::default();
    let out_cb = Rc::clone(&output);
    let mut write = move |text: &str| out_cb.borrow_mut().push_str(text);
    let value = collect_masked_input(&mut scripted(&chars), &mut write, "p: ", "*").unwrap();
    assert_eq!(value, "");
    assert!(output.borrow().ends_with("\r\n"));
}

#[test]
fn backspace_erases_only_when_buffer_nonempty() {
    // 'a', backspace, backspace (second is a no-op), 'b', enter.
    let chars = vec![
        Some('a'),
        Some('\u{7f}'),
        Some('\u{8}'),
        Some('b'),
        Some('\r'),
    ];
    let output: Rc<RefCell<String>> = Rc::default();
    let out_cb = Rc::clone(&output);
    let mut write = move |text: &str| out_cb.borrow_mut().push_str(text);
    let value = collect_masked_input(&mut scripted(&chars), &mut write, "p: ", "*").unwrap();
    assert_eq!(value, "b");
    // One erase sequence for the one real erase.
    assert_eq!(output.borrow().matches("\u{8} \u{8}").count(), 1);
}

#[test]
fn ctrl_c_raises_interrupt_and_ctrl_d_z_raise_eof() {
    for (ch, expected) in [
        ('\u{3}', MaskedInputError::Interrupted),
        ('\u{4}', MaskedInputError::Eof),
        ('\u{1a}', MaskedInputError::Eof),
    ] {
        let chars = vec![Some(ch)];
        let mut write = |_text: &str| {};
        let result =
            collect_masked_input(&mut scripted(&chars), &mut write, "p: ", "*").unwrap_err();
        assert_eq!(result, expected);
    }
}

#[test]
fn reader_exhaustion_is_eof() {
    let chars: Vec<Option<char>> = vec![Some('a'), None];
    let mut write = |_text: &str| {};
    let result = collect_masked_input(&mut scripted(&chars), &mut write, "p: ", "*").unwrap_err();
    assert_eq!(result, MaskedInputError::Eof);
}

#[test]
fn escape_is_ignored_so_navigation_sequences_stay_out() {
    // ESC, 'a', enter: only the 'a' lands.
    let chars = vec![Some('\u{1b}'), Some('a'), Some('\n')];
    let mut write = |_text: &str| {};
    let value = collect_masked_input(&mut scripted(&chars), &mut write, "p: ", "*").unwrap();
    assert_eq!(value, "a");
}

#[test]
fn empty_mask_collects_without_feedback() {
    let chars = vec![Some('x'), Some('\n')];
    let output: Rc<RefCell<String>> = Rc::default();
    let out_cb = Rc::clone(&output);
    let mut write = move |text: &str| out_cb.borrow_mut().push_str(text);
    let value = collect_masked_input(&mut scripted(&chars), &mut write, "p: ", "").unwrap();
    assert_eq!(value, "x");
    // `if mask:` — no mask characters written.
    assert_eq!(output.borrow().clone(), "p: \r\n");
}

// ── cli_output ───────────────────────────────────────────────────────────

#[test]
fn prompt_strips_and_takes_default_on_empty_input() {
    let inputs = vec![Some("  padded  \n".to_string())];
    let mut queue = inputs.into_iter();
    let mut read_line = move || queue.next().flatten();
    assert_eq!(prompt_from("q", None, false, &mut read_line), "padded");

    let inputs = vec![Some("\n".to_string())];
    let mut queue = inputs.into_iter();
    let mut read_line = move || queue.next().flatten();
    assert_eq!(
        prompt_from("q", Some("fallback"), false, &mut read_line),
        "fallback"
    );
}

#[test]
fn prompt_eof_returns_empty_string() {
    let mut read_line = || None;
    assert_eq!(prompt_from("q", Some("d"), false, &mut read_line), "");
    // Upstream returns "" (not the default) on EOF/Interrupt — verified
    // against `return value if value else (default or "")` reached only on
    // real input; the except arm is a bare `return ""`.
    let inputs = vec![Some("x\n".to_string())];
    let mut queue = inputs.into_iter();
    let mut read_line = move || queue.next().flatten();
    assert_eq!(prompt_from("q", Some("d"), false, &mut read_line), "x");
}

#[test]
fn prompt_yes_no_hint_and_default_semantics() {
    // Default true: hint Y/n; empty answer takes the default.
    let inputs = vec![Some("\n".to_string())];
    let mut queue = inputs.into_iter();
    let mut read_line = move || queue.next().flatten();
    assert!(prompt_yes_no_with("ok?", true, &mut read_line));

    // Default false with empty answer.
    let inputs = vec![Some("\n".to_string())];
    let mut queue = inputs.into_iter();
    let mut read_line = move || queue.next().flatten();
    assert!(!prompt_yes_no_with("ok?", false, &mut read_line));

    // Any y-prefixed answer is true; anything else false.
    let inputs = vec![Some("YES\n".to_string())];
    let mut queue = inputs.into_iter();
    let mut read_line = move || queue.next().flatten();
    assert!(prompt_yes_no_with("ok?", false, &mut read_line));

    let inputs = vec![Some("nope\n".to_string())];
    let mut queue = inputs.into_iter();
    let mut read_line = move || queue.next().flatten();
    assert!(!prompt_yes_no_with("ok?", true, &mut read_line));
}

// ── line_input ─────────────────────────────────────────────────────────────
// PARITY @ 5d59366: TTY-gated fancy reader with triple fallback (non-TTY,
// missing prompt_toolkit, runtime failure → plain reader; KI/EOF re-raise).

use hermes_cli::cli_output::{line_input_from, LineInputFailure};

#[test]
fn line_input_non_tty_uses_plain_reader() {
    let out = line_input_from(
        "name: ",
        false,
        &mut || Err(LineInputFailure::Unavailable),
        &mut || Some("plain".to_string()),
    );
    assert_eq!(out, Some("plain".to_string()));
}

#[test]
fn line_input_fancy_success_wins_on_tty() {
    let out = line_input_from(
        "name: ",
        true,
        &mut || Ok(Some("fancy".to_string())),
        &mut || Some("plain".to_string()),
    );
    assert_eq!(out, Some("fancy".to_string()));
}

#[test]
fn line_input_fancy_runtime_failure_falls_back() {
    let out = line_input_from(
        "name: ",
        true,
        &mut || Err(LineInputFailure::Runtime("EINVAL".to_string())),
        &mut || Some("fallback".to_string()),
    );
    assert_eq!(out, Some("fallback".to_string()));
}

#[test]
fn line_input_interrupt_and_eof_propagate() {
    for failure in [LineInputFailure::Interrupted, LineInputFailure::Eof] {
        let out = line_input_from(
            "name: ",
            true,
            &mut || Err(failure.clone()),
            &mut || Some("must-not-use".to_string()),
        );
        assert_eq!(out, None);
    }
}
