//! Parity oracles for `hermes-utils` vs upstream `utils.py` (@ b9aa928).
//! Golden fixture: `upstream/golden_utils.json` (generated from the real
//! upstream module).

use hermes_utils::{
    base_url_host_matches, base_url_hostname, is_truthy,
    model_forces_max_completion_tokens, normalize_proxy_url, TruthyValue,
};
use serde_json::Value;

fn load_golden() -> Value {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("upstream/golden_utils.json");
    let text = std::fs::read_to_string(&path).expect("golden fixture missing");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn is_truthy_value_matches_upstream_golden() {
    let g = load_golden();
    let tc = &g["is_truthy_value"];
    let expect = |key: &str, value: TruthyValue, default: bool| {
        let got = is_truthy(&value, default);
        let want = tc[key].as_bool().unwrap();
        assert_eq!(got, want, "key {}: got {} want {}", key, got, want);
    };
    for (key_str, v) in [
        ("True", TruthyValue::Bool(true)),
        ("False", TruthyValue::Bool(false)),
        ("'1'", TruthyValue::Str("1")),
        ("'true'", TruthyValue::Str("true")),
        ("'TRUE'", TruthyValue::Str("TRUE")),
        ("'Yes'", TruthyValue::Str("Yes")),
        ("'on'", TruthyValue::Str("on")),
        ("'0'", TruthyValue::Str("0")),
        ("'false'", TruthyValue::Str("false")),
        ("'no'", TruthyValue::Str("no")),
        ("'off'", TruthyValue::Str("off")),
        ("'yes sir'", TruthyValue::Str("yes sir")),
        ("''", TruthyValue::Str("")),
        ("'  on  '", TruthyValue::Str("  on  ")),
    ] {
        expect(key_str, v, false);
    }
}

#[test]
fn model_forces_max_completion_tokens_matches_upstream_golden() {
    let g = load_golden();
    let mc = &g["model_forces_max_completion_tokens"];
    for (model, want) in mc.as_object().unwrap() {
        let got = model_forces_max_completion_tokens(model);
        let want = want.as_bool().unwrap();
        assert_eq!(got, want, "model {:?}: got {} want {}", model, got, want);
    }
}

#[test]
fn base_url_hostname_matches_upstream_golden() {
    let g = load_golden();
    let hh = &g["base_url_hostname"];
    for (url, want) in hh.as_object().unwrap() {
        let got = base_url_hostname(url);
        let want = want.as_str().unwrap();
        assert_eq!(got, want, "url {:?}: got {:?} want {:?}", url, got, want);
    }
}

#[test]
fn base_url_host_matches_upstream_golden() {
    let g = load_golden();
    let hm = &g["base_url_host_matches"];
    for (record, want) in hm.as_object().unwrap() {
        let (url, domain) = record.split_once(" || ").unwrap();
        let got = base_url_host_matches(url, domain);
        let want = want.as_bool().unwrap();
        assert_eq!(got, want, "record {}", record);
    }
}

#[test]
fn normalize_proxy_url_matches_upstream_golden() {
    let g = load_golden();
    let np = &g["normalize_proxy_url"];
    for (rec, want) in np.as_object().unwrap() {
        // Golden keys are repr() of the Python input; map back:
        let input: Option<&str> = match rec.as_str() {
            "None" => None,
            "'  '" => Some("  "),
            other => Some(other.trim_matches('\'')),
        };
        let got = normalize_proxy_url(input);
        match want {
            Value::Null => assert!(got.is_none(), "record {}: expected None, got {:?}", rec, got),
            Value::String(s) => assert_eq!(got.as_deref(), Some(s.as_str()), "record {}", rec),
            other => panic!("unexpected golden shape for {}: {}", rec, other),
        }
    }
}
