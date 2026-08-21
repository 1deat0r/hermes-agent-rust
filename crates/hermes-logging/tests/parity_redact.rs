//! Parity oracle: byte-for-byte equivalence with the real upstream
//! agent/redact.py functions @ b9aa928 over a crafted corpus.
//! Golden: upstream/golden_redact.json (generated from the actual Python).

use hermes_logging::{
    is_env_dump_command, mask_secret, redact_cdp_url, redact_sensitive_text,
    redact_terminal_output,
};

fn golden() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../upstream/golden_redact.json")).expect("golden")
}

#[test]
fn redact_sensitive_text_matches_upstream_corpus() {
    let g = golden();
    let samples: Vec<&str> = g["samples"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    let expected: Vec<&str> = g["redact"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    for (i, s) in samples.iter().enumerate() {
        let out = redact_sensitive_text(s, false, false, false, false);
        assert_eq!(out, expected[i], "redact_sensitive_text case {i}: {:?}", s);
    }
}

#[test]
fn redact_cdp_url_matches_upstream_corpus() {
    let g = golden();
    let samples: Vec<&str> = g["samples"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    let expected: Vec<&str> = g["cdp"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    for (i, s) in samples.iter().take(expected.len()).enumerate() {
        let out = redact_cdp_url(s);
        assert_eq!(out, expected[i], "redact_cdp_url case {i}");
    }
}

#[test]
fn mask_secret_matches_upstream_corpus() {
    let g = golden();
    let expected: Vec<&str> = g["mask_secret"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(mask_secret("sk-proj-abcdef1234567890", 4, 4, 12, "***", ""), expected[0]);
    assert_eq!(mask_secret("short", 4, 4, 12, "***", ""), expected[1]);
    assert_eq!(mask_secret("", 4, 4, 12, "***", ""), expected[2]);
    assert_eq!(mask_secret(&"a".repeat(40), 8, 2, 30, "***", ""), expected[3]);
}

#[test]
fn redact_terminal_output_matches_upstream_corpus() {
    let g = golden();
    let cases: Vec<(String, Option<&str>)> = vec![
        ("MAX_TOKENS=100".to_string(), Some("cat main.rs")),
        ("MAX_TOKENS=100".to_string(), Some("env")),
        ("MY_SERVICE_TOKEN=abc123randomstring".to_string(), Some("printenv")),
        ("postgresql://user:{pass}@host/db".to_string(), Some("cat app.py")),
        ("cat .env".to_string(), Some("cat .env")),
    ];
    let expected: Vec<&str> = g["term_output"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    for (i, (output, command)) in cases.into_iter().enumerate() {
        let out = redact_terminal_output(&output, command, false);
        assert_eq!(out, expected[i], "terminal case {i}: {:?} {:?}", output, command);
    }
}

#[test]
fn is_env_dump_command_matches_upstream_corpus() {
    let g = golden();
    let cases = ["env", "echo x | printenv", "cat .env", "ls -la"];
    let expected: Vec<bool> = g["is_env_dump"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(is_env_dump_command(Some(c)), expected[i], "isdump case {i}: {c}");
    }
}
