// Tier: unit — mirrors tests/agent/test_message_content.py.

use hermes_agent::message_content::flatten_message_text;
use serde_json::json;

#[test]
fn accepts_chat_and_responses_text_parts() {
    let content = json!([
        {"type": "text", "text": "chat text"},
        {"type": "input_text", "text": "user text"},
        {"type": "output_text", "text": "assistant text"},
        {"type": "summary_text", "text": "summary text"},
    ]);
    assert_eq!(
        flatten_message_text(Some(&content), "\n"),
        "chat text\nuser text\nassistant text\nsummary text"
    );
}

#[test]
fn accepts_object_and_legacy_content_parts() {
    // `SimpleNamespace(type="output_text", text="object text")` and
    // `{"content": "legacy content"}` are both mapped-shaped parts in Rust.
    let content = json!([
        {"type": "output_text", "text": "object text"},
        {"content": "legacy content"},
    ]);
    assert_eq!(
        flatten_message_text(Some(&content), "\n"),
        "object text\nlegacy content"
    );
}

// Source-derived cases for the branches the upstream oracle does not touch.
#[test]
fn scalar_and_empty_shapes() {
    assert_eq!(flatten_message_text(None, "\n"), "");
    let plain = json!("hello");
    assert_eq!(flatten_message_text(Some(&plain), "\n"), "hello");
    let empty: Vec<serde_json::Value> = Vec::new();
    let empty = json!(empty);
    assert_eq!(flatten_message_text(Some(&empty), "\n"), "");
    // Blank parts are filtered out of the join.
    let content = json!([{"type": "text", "text": ""}, {"type": "text", "text": "b"}]);
    assert_eq!(flatten_message_text(Some(&content), " | "), "b");
    assert_eq!(flatten_message_text(Some(&content), "\n"), "b");
}

#[test]
fn non_text_part_types_are_skipped() {
    let content = json!([
        {"type": "image_url", "image_url": {"url": "https://example.invalid/a.png"}},
        {"type": "input_audio", "input_audio": {"data": "..."}},
        {"type": "text", "text": "kept"},
    ]);
    assert_eq!(flatten_message_text(Some(&content), "\n"), "kept");
    // An `audio`/`input_image` part with a text key still loses to its type.
    let sneaky = json!([{"type": "audio", "text": "transcript"}]);
    assert_eq!(flatten_message_text(Some(&sneaky), "\n"), "");
}

#[test]
fn type_is_compared_case_insensitively_and_trimmed() {
    let content = json!([{"type": "  IMAGE  ", "text": "hidden"}]);
    assert_eq!(flatten_message_text(Some(&content), "\n"), "");
}

#[test]
fn unknown_single_part_falls_back_to_its_string_form() {
    // `str(content)` on a mapping yields the source's fallback text; the Rust
    // port renders the JSON document instead, which is the observable shape of
    // an unmapped payload.
    let number = json!(7);
    assert_eq!(flatten_message_text(Some(&number), "\n"), "7");
    let object = json!({"unexpected": true});
    assert_eq!(
        flatten_message_text(Some(&object), "\n"),
        "{\"unexpected\":true}"
    );
}
