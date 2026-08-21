//! URL hostname helpers and model capability detection.
//!
//! PARITY: utils.py lines 590–666 (`base_url_hostname`,
//! `model_forces_max_completion_tokens`, `base_url_host_matches`).

use url::Url;

/// Return the lowercased hostname for a base URL, or `""` if absent.
///
/// Use exact-hostname comparisons against known provider hosts instead of
/// substring matches on the raw URL (which false-positive on
/// `https://api.openai.com.example/v1` or `https://proxy.test/api.openai.com/v1`).
///
/// PARITY: utils.py `base_url_hostname` (593–607).
pub fn base_url_hostname(base_url: &str) -> String {
    let raw = base_url.trim();
    if raw.is_empty() {
        return String::new();
    }
    // Python's urlparse handles protocol-relative input ("//host/path");
    // the url crate needs a scheme, so synthesize one for scheme-less input.
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{}", raw)
    };
    match Url::parse(&with_scheme) {
        Ok(u) => u.host_str().unwrap_or("").to_lowercase().trim_end_matches('.').to_string(),
        Err(_) => String::new(),
    }
}

/// Return True when the base URL's hostname is `domain` or a subdomain.
///
/// PARITY: utils.py `base_url_host_matches` (648–666).
pub fn base_url_host_matches(base_url: &str, domain: &str) -> bool {
    let hostname = base_url_hostname(base_url);
    if hostname.is_empty() {
        return false;
    }
    let domain = domain.trim().to_lowercase().trim_end_matches('.').to_string();
    if domain.is_empty() {
        return false;
    }
    hostname == domain || hostname.ends_with(&format!(".{}", domain))
}

/// Return True for model families that require `max_completion_tokens`.
///
/// OpenAI's newer families reject `max_tokens` on `/v1/chat/completions`.
/// Handles vendor prefixes by stripping to the tail after the last `/`.
///
/// PARITY: utils.py `model_forces_max_completion_tokens` (613–645).
pub fn model_forces_max_completion_tokens(model: &str) -> bool {
    let mut m = model.trim().to_lowercase();
    if m.is_empty() {
        return false;
    }
    if let Some(idx) = m.rfind('/') {
        m = m[idx + 1..].to_string();
    }
    m.starts_with("gpt-4o")
        || m.starts_with("gpt-4.1")
        || m.starts_with("gpt-5")
        || m.starts_with("o1")
        || m.starts_with("o3")
        || m.starts_with("o4")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_examples() {
        assert_eq!(base_url_hostname("https://api.openai.com/v1"), "api.openai.com");
        assert_eq!(base_url_hostname("https://api.openai.com.example/v1"), "api.openai.com.example");
        assert_eq!(base_url_hostname("api.moonshot.ai/v1"), "api.moonshot.ai");
        assert_eq!(base_url_hostname(""), "");
        assert_eq!(base_url_hostname("   "), "");
    }

    #[test]
    fn host_matches() {
        assert!(base_url_host_matches("https://api.moonshot.ai/v1", "moonshot.ai"));
        assert!(base_url_host_matches("https://moonshot.ai", "moonshot.ai"));
        assert!(!base_url_host_matches("https://evil.com/moonshot.ai/v1", "moonshot.ai"));
        assert!(!base_url_host_matches("https://moonshot.ai.evil/v1", "moonshot.ai"));
        assert!(!base_url_host_matches("", "moonshot.ai"));
    }

    #[test]
    fn max_completion_token_families() {
        for m in [
            "gpt-4o", "gpt-4o-mini", "gpt-4.1", "gpt-4.1-nano", "gpt-5", "gpt-5.4",
            "o1", "o1-preview", "o3", "o3-mini", "o4-mini", "openai/gpt-5.4",
        ] {
            assert!(model_forces_max_completion_tokens(m), "{}", m);
        }
        for m in ["gpt-3.5-turbo", "gpt-4", "claude-opus-4.5", "", "anthropic/claude-sonnet-4-5"] {
            assert!(!model_forces_max_completion_tokens(m), "{}", m);
        }
        // Upstream uses startswith, so "o1x" DOES match (Python str.startswith).
        assert!(model_forces_max_completion_tokens("o1x"), "upstream startswith semantics");
    }
}
