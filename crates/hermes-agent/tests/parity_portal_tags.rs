// Tier: unit — mirrors tests/agent/test_portal_tags.py for
// `agent/portal_tags.py`.
//
// The ambient conversation id is thread-local in the Rust port, and the
// libtest harness runs each `#[test]` on its own thread, so a test starts from
// the same clean context an isolated Python `Context` provides.

use hermes_agent::portal_tags::{
    conversation_tag, get_conversation_context, hermes_client_tag, hermes_version,
    nous_portal_tags, propagate_conversation_context_to_thread, reset_conversation_context,
    set_conversation_context, set_hermes_version, ConversationContext, HERMES_VERSION,
};
use serde_json::{Map, Value};

// Mirrors test_nous_portal_tags_contains_product_and_client: every Nous
// Portal request gets BOTH the product tag and the version tag.
#[test]
fn nous_portal_tags_contains_product_and_client() {
    let tags = nous_portal_tags(None);

    assert!(tags.iter().any(|tag| tag == "product=hermes-agent"));
    assert!(tags.iter().any(|tag| tag == &hermes_client_tag()));
    assert_eq!(tags.len(), 2);
}

#[test]
fn tag_shapes_are_pinned_to_the_b9aa928_version() {
    assert_eq!(HERMES_VERSION, "0.20.0");
    assert_eq!(hermes_version(), HERMES_VERSION);
    assert_eq!(hermes_client_tag(), "client=hermes-client-v0.20.0");
    assert_eq!(conversation_tag("abc"), "conversation=abc");
}

// Mirrors test_ambient_context_set_none_clears: publishing a falsy id
// publishes no tag, and `""` is coerced to no context.
#[test]
fn ambient_context_set_none_clears() {
    for empty in [None, Some("")] {
        let token = set_conversation_context(empty);
        assert_eq!(get_conversation_context(), None);
        assert_eq!(nous_portal_tags(None).len(), 2);
        reset_conversation_context(token);
    }
}

// Upstream's `effective = get_conversation_context() or session_id` treats a
// falsy argument as absent, so an empty session id appends no conversation tag.
#[test]
fn empty_session_id_appends_no_tag() {
    assert_eq!(
        nous_portal_tags(Some("")),
        vec![
            "product=hermes-agent".to_string(),
            "client=hermes-client-v0.20.0".to_string(),
        ]
    );
}

#[test]
fn ambient_context_wins_over_the_explicit_session_id() {
    let token = set_conversation_context(Some("root-conversation"));

    // Rotated segment id passed explicitly — the ambient root still wins.
    let tags = nous_portal_tags(Some("segment-after-compaction"));
    assert_eq!(tags.len(), 3);
    assert!(tags.contains(&conversation_tag("root-conversation")));
    assert!(!tags.contains(&conversation_tag("segment-after-compaction")));

    reset_conversation_context(token);
    // With the context cleared, the explicit id is the fallback again.
    assert_eq!(
        nous_portal_tags(Some("segment-after-compaction"))[2],
        conversation_tag("segment-after-compaction")
    );
}

// Upstream: "Always returns a fresh list so callers can mutate it freely."
#[test]
fn nous_portal_tags_returns_a_fresh_list() {
    let mut first = nous_portal_tags(None);
    first.push("mutated=by-caller".into());

    assert_eq!(first.len(), 3);
    assert_eq!(nous_portal_tags(None).len(), 2);
    assert!(!nous_portal_tags(None)
        .iter()
        .any(|tag| tag.starts_with("mutated")));
}

// Mirrors test_ambient_context_isolated_between_contexts: two concurrent
// agents must not see each other's conversation id, and the publishing thread
// stays clean.
#[test]
fn ambient_context_is_isolated_between_threads() {
    let agent_a = std::thread::spawn(|| {
        set_conversation_context(Some("agent-a"));
        nous_portal_tags(None)
    });
    let agent_b = std::thread::spawn(|| {
        set_conversation_context(Some("agent-b"));
        nous_portal_tags(None)
    });

    let tags_a = agent_a.join().expect("agent-a thread");
    let tags_b = agent_b.join().expect("agent-b thread");

    assert!(tags_a.contains(&conversation_tag("agent-a")));
    assert!(tags_b.contains(&conversation_tag("agent-b")));
    assert!(!tags_a
        .iter()
        .any(|tag| tag.starts_with("conversation=") && tag != &conversation_tag("agent-a")));
    assert!(!tags_a.contains(&conversation_tag("agent-b")));

    // The outer context stays clean: a fresh thread sees no ambient id.
    let outer = std::thread::spawn(|| nous_portal_tags(None))
        .join()
        .expect("outer thread");
    assert!(!outer.iter().any(|tag| tag.starts_with("conversation=")));
}

// A bare worker thread starts with no ambient id, matching upstream's bare
// `executor.submit(nous_portal_tags)` case.
#[test]
fn bare_thread_loses_the_ambient_context() {
    let token = set_conversation_context(Some("moa-root"));

    let plain = std::thread::spawn(|| nous_portal_tags(None))
        .join()
        .expect("worker thread");
    assert!(!plain.iter().any(|tag| tag.starts_with("conversation=")));

    // Resetting with the token from `set_conversation_context` restores the
    // value captured before the publish, which here is "no context".
    reset_conversation_context(token);
    assert_eq!(get_conversation_context(), None);
}

// Mirrors test_nous_sticky_key_matches_conversation_tag at the layer that
// owns the ambient context in Rust: the agent publishes it, and the profile
// body is built from the same effective conversation key.
#[test]
fn nous_sticky_key_matches_the_conversation_tag() {
    use hermes_providers::registry::get_provider_profile;

    let profile = get_provider_profile("nous").expect("nous profile registered");
    let token = set_conversation_context(Some("root-conversation"));
    let ambient = get_conversation_context();
    let mut context = Map::new();
    if let Some(ambient) = &ambient {
        context.insert(
            "conversation_context".into(),
            Value::String(ambient.clone()),
        );
    }

    // Rotated segment id passed explicitly — root still wins, both places.
    let body = profile.build_extra_body(Some("segment-after-compaction"), &context);
    assert_eq!(
        body.get("session_id"),
        Some(&Value::String("root-conversation".into()))
    );
    assert!(body
        .get("tags")
        .and_then(Value::as_array)
        .expect("tags")
        .contains(&Value::String(conversation_tag("root-conversation"))));

    // Auxiliary call sites pass no session id but inherit the context.
    let aux = profile.build_extra_body(None, &context);
    assert_eq!(
        aux.get("session_id"),
        Some(&Value::String("root-conversation".into()))
    );

    reset_conversation_context(token);
}

// Mirrors the propagated half of
// test_ambient_context_propagates_via_thread_context_helper: the wrapper is
// built on the parent thread and the worker observes the ambient id.
#[test]
fn propagated_worker_keeps_the_ambient_context() {
    let token = set_conversation_context(Some("moa-root"));

    let plain = || nous_portal_tags(None);
    let propagated = propagate_conversation_context_to_thread(plain);

    let bare = std::thread::spawn(plain).join().expect("bare worker");
    assert!(!bare.iter().any(|tag| tag.starts_with("conversation=")));

    let worker = std::thread::spawn(propagated).join().expect("worker");
    assert!(worker.contains(&conversation_tag("moa-root")));

    reset_conversation_context(token);
    // A wrapper captured after the turn ends carries no ambient id, and the
    // worker's own thread-local is restored when a run finishes.
    let after = propagate_conversation_context_to_thread(get_conversation_context);
    let seen = std::thread::spawn(after).join().expect("restored");
    assert_eq!(seen, None);
}

// A captured context behaves like a reused Python `Context`: it carries the
// id it captured, not whatever the publishing thread does afterwards, and
// values a run writes stay in that context for its next run.
#[test]
fn captured_context_is_stable_and_reusable() {
    let token = set_conversation_context(Some("turn-root"));
    let context = ConversationContext::capture();
    reset_conversation_context(token);

    // The publisher moved on; the captured context still says `turn-root`.
    let seen = context.run(get_conversation_context);
    assert_eq!(seen.as_deref(), Some("turn-root"));
    // ... and the calling thread's own ambient id is restored afterwards.
    assert_eq!(get_conversation_context(), None);

    // A write inside the run persists in that context only.
    context.run(|| {
        set_conversation_context(Some("rotated-segment"));
    });
    assert_eq!(get_conversation_context(), None);
    assert_eq!(
        context.run(get_conversation_context).as_deref(),
        Some("rotated-segment")
    );

    // Nesting one context inside its own run is a Python RuntimeError; the Rust
    // port returns the ambient id for the outer frame and leaves the context
    // untouched, which no upstream test relies on.
    let outer = context.run(|| context.run(get_conversation_context));
    assert_eq!(outer.as_deref(), Some("rotated-segment"));

    // Fresh captures see the current ambient id again.
    let token = set_conversation_context(Some("next-turn"));
    assert_eq!(
        ConversationContext::capture()
            .run(get_conversation_context)
            .as_deref(),
        Some("next-turn")
    );
    reset_conversation_context(token);
}

// The agent loop turn wrapper pairs capture with restore, mirroring
// `set_conversation_context` on entry / `reset_conversation_context` on exit.
#[test]
fn turn_publish_and_restore_round_trip() {
    let mut previous = set_conversation_context(Some("turn-1"));
    assert_eq!(get_conversation_context().as_deref(), Some("turn-1"));
    previous = {
        reset_conversation_context(previous.take());
        set_conversation_context(Some("turn-2"))
    };
    assert_eq!(get_conversation_context().as_deref(), Some("turn-2"));
    reset_conversation_context(previous);
    assert_eq!(get_conversation_context(), None);
}

// The source computes the client tag on every call so a bumped
// `hermes_cli.__version__` is picked up without restarting a long-running
// gateway (`agent/portal_tags.py` lines 28-31 and 85-103).
#[test]
fn a_published_version_changes_the_tag_without_a_restart() {
    let previous = set_hermes_version(Some("0.21.0"));
    assert_eq!(hermes_version(), "0.21.0");
    assert_eq!(nous_portal_tags(None)[1], "client=hermes-client-v0.21.0");

    // Whitespace-only input is falsy-equivalent and restores the fallback.
    set_hermes_version(Some("   "));
    assert_eq!(hermes_version(), HERMES_VERSION);

    set_hermes_version(Some("9.9.9"));
    assert_eq!(hermes_client_tag(), "client=hermes-client-v9.9.9");
    set_hermes_version(previous.as_deref());
    assert_eq!(hermes_client_tag(), "client=hermes-client-v0.20.0");
}
