// Tier: unit — mirrors tests/agent/test_portal_tags.py for
// `agent/portal_tags.py`.
//
// The ambient conversation id is thread-local in the Rust port, and the
// libtest harness runs each `#[test]` on its own thread, so a test starts from
// the same clean context an isolated Python `Context` provides.

use hermes_agent::portal_tags::{
    conversation_tag, get_conversation_context, hermes_client_tag, nous_portal_tags,
    reset_conversation_context, set_conversation_context, HERMES_VERSION,
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
// `executor.submit(nous_portal_tags)` case. The propagated-worker half of
// `test_ambient_context_propagates_via_thread_context_helper` is still a
// pending seam: `agent.portal_tags` is not yet wired into
// `hermes_tools::thread_context`'s snapshot factory (see PLAN §7).
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
