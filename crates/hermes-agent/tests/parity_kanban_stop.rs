// Tier: unit — mirrors tests/agent/test_kanban_stop.py.
//
// The guard reads two environment variables, so every case here takes the same
// mutex and restores the previous values.

use hermes_agent::kanban_stop::{
    build_kanban_stop_nudge, kanban_stop_nudge_enabled, session_called_kanban_terminal,
    KanbanStopNudgeOptions, DEFAULT_MAX_ATTEMPTS,
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::ffi::OsString;

static KANBAN_ENV_MUTEX: Mutex<()> = Mutex::new(());

struct KanbanEnv {
    previous: Vec<(&'static str, Option<OsString>)>,
}

impl KanbanEnv {
    fn clear() -> Self {
        let previous = ["HERMES_KANBAN_TASK", "HERMES_KANBAN_STOP_NUDGE"]
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect();
        for key in ["HERMES_KANBAN_TASK", "HERMES_KANBAN_STOP_NUDGE"] {
            unsafe { std::env::remove_var(key) };
        }
        Self { previous }
    }

    fn set(&self, key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }
}

impl Drop for KanbanEnv {
    fn drop(&mut self) {
        for (key, value) in self.previous.iter() {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn messages() -> Vec<Value> {
    vec![
        json!({"role": "user", "content": "work kanban task"}),
        json!({
            "role": "assistant",
            "content": "Let me write the comprehensive recipe.",
            "tool_calls": [{
                "id": "1", "type": "function",
                "function": {"name": "kanban_heartbeat", "arguments": "{}"}
            }],
        }),
        json!({"role": "tool", "name": "kanban_heartbeat", "tool_call_id": "1", "content": "ok"}),
    ]
}

#[test]
fn env_can_disable_the_guard() {
    let _lock = KANBAN_ENV_MUTEX.lock();
    let env = KanbanEnv::clear();
    env.set("HERMES_KANBAN_TASK", "t_abc");
    env.set("HERMES_KANBAN_STOP_NUDGE", "0");

    assert!(!kanban_stop_nudge_enabled());
    assert_eq!(
        build_kanban_stop_nudge(KanbanStopNudgeOptions::new(Some(&messages()))),
        None
    );
}

#[test]
fn nudge_when_no_terminal_tool() {
    let _lock = KANBAN_ENV_MUTEX.lock();
    let env = KanbanEnv::clear();
    env.set("HERMES_KANBAN_TASK", "t_46be8aa5");

    let nudge =
        build_kanban_stop_nudge(KanbanStopNudgeOptions::new(Some(&messages())).with_attempts(0))
            .expect("nudge");

    assert!(nudge.contains("kanban_complete"));
    assert!(nudge.contains("kanban_block"));
    assert!(nudge.contains("t_46be8aa5"));
    assert!(nudge.contains("protocol violation"));
}

#[test]
fn no_nudge_after_a_terminal_tool() {
    let _lock = KANBAN_ENV_MUTEX.lock();
    let env = KanbanEnv::clear();
    env.set("HERMES_KANBAN_TASK", "t_abc");
    let done = vec![
        json!({
            "role": "assistant", "content": "",
            "tool_calls": [{
                "id": "1", "type": "function",
                "function": {"name": "kanban_complete", "arguments": "{}"}
            }],
        }),
        json!({"role": "tool", "name": "kanban_complete", "tool_call_id": "1", "content": "done"}),
    ];

    assert!(session_called_kanban_terminal(Some(&done)));
    assert_eq!(
        build_kanban_stop_nudge(KanbanStopNudgeOptions::new(Some(&done))),
        None
    );
}

#[test]
fn guard_is_off_without_a_kanban_task() {
    let _lock = KANBAN_ENV_MUTEX.lock();
    let env = KanbanEnv::clear();
    assert!(!kanban_stop_nudge_enabled());

    env.set("HERMES_KANBAN_TASK", "   ");
    assert!(!kanban_stop_nudge_enabled());

    env.set("HERMES_KANBAN_TASK", "t_abc");
    assert!(kanban_stop_nudge_enabled());
    // Only explicit disable spellings work; anything else stays on.
    for (spelling, enabled) in [
        ("", true),
        ("1", true),
        ("OFF", false),
        ("No", false),
        (" false ", false),
        ("yes", true),
    ] {
        env.set("HERMES_KANBAN_STOP_NUDGE", spelling);
        assert_eq!(kanban_stop_nudge_enabled(), enabled, "spell {spelling:?}");
    }
}

#[test]
fn nudge_budget_is_bounded_and_defaulted() {
    let _lock = KANBAN_ENV_MUTEX.lock();
    let env = KanbanEnv::clear();
    env.set("HERMES_KANBAN_TASK", "t_abc");
    assert_eq!(DEFAULT_MAX_ATTEMPTS, 2);

    assert!(build_kanban_stop_nudge(
        KanbanStopNudgeOptions::new(Some(&messages())).with_attempts(1)
    )
    .is_some());
    assert!(build_kanban_stop_nudge(
        KanbanStopNudgeOptions::new(Some(&messages())).with_attempts(2)
    )
    .is_none());
    assert!(build_kanban_stop_nudge(
        KanbanStopNudgeOptions::new(Some(&messages()))
            .with_attempts(2)
            .with_max_attempts(3)
    )
    .is_some());
}

#[test]
fn task_id_argument_wins_over_the_environment() {
    let _lock = KANBAN_ENV_MUTEX.lock();
    let env = KanbanEnv::clear();
    env.set("HERMES_KANBAN_TASK", "t_from_env");

    let nudge = build_kanban_stop_nudge(
        KanbanStopNudgeOptions::new(Some(&messages())).with_task_id(Some("t_explicit")),
    )
    .expect("nudge");
    assert!(nudge.contains("`t_explicit`"));
    assert!(!nudge.contains("t_from_env"));

    // An empty-string task id is falsy in the source `or` chain, so the
    // environment value wins.
    let nudge = build_kanban_stop_nudge(
        KanbanStopNudgeOptions::new(Some(&messages())).with_task_id(Some("")),
    )
    .expect("env task id");
    assert!(nudge.contains("`t_from_env`"), "{nudge}");

    // A whitespace-only task id is truthy in the `or` chain but empties out on
    // the final `.strip()`, which is the one reachable path to the literal.
    let nudge = build_kanban_stop_nudge(
        KanbanStopNudgeOptions::new(Some(&messages())).with_task_id(Some("   ")),
    )
    .expect("literal task id");
    assert!(nudge.contains("`this task`"), "{nudge}");
    // Clearing the task turns the whole guard off.
    env.set("HERMES_KANBAN_TASK", "");
    assert_eq!(
        build_kanban_stop_nudge(KanbanStopNudgeOptions::new(Some(&messages()))),
        None
    );
}

#[test]
fn terminal_detection_covers_the_message_shapes() {
    let _lock = KANBAN_ENV_MUTEX.lock();
    assert!(!session_called_kanban_terminal(None));
    assert!(!session_called_kanban_terminal(Some(&[])));
    // Non-assistant/tool roles never satisfy the guard.
    assert!(!session_called_kanban_terminal(Some(&[json!({
        "role": "user", "name": "kanban_block", "tool_calls": [{ "name": "kanban_block" }]
    })])));
    // A tool-role message whose name is a terminal tool counts.
    assert!(session_called_kanban_terminal(Some(&[json!({
        "role": "tool", "name": "kanban_block", "content": "ok"
    })])));
    // Assistant tool calls accept both the nested function shape and a bare
    // name, and a non-string name is stringified like Python's str().
    assert!(session_called_kanban_terminal(Some(&[json!({
        "role": "assistant", "tool_calls": [{ "name": "kanban_complete" }]
    })])));
    assert!(session_called_kanban_terminal(Some(&[json!({
        "role": "assistant", "tool_calls": [{ "function": { "name": "kanban_block" } }]
    })])));
    // Malformed entries are skipped instead of aborting the scan.
    assert!(!session_called_kanban_terminal(Some(&[
        json!("not-a-message"),
        json!({"role": "assistant", "tool_calls": json!("oops")}),
        json!({"role": "assistant"}),
    ])));
    // A missing/blank name never matches.
    assert!(!session_called_kanban_terminal(Some(&[json!({
        "role": "assistant", "tool_calls": [{"function": {}}]
    })])));
}
