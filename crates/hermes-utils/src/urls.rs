//! URL hostname / origin helpers and model capability detection.
//!
//! PARITY: utils.py @ 5d59366 — `_parse_base_url` (557–560),
//! `_hostname_of` (563–564), `base_url_hostname` (567–574),
//! `model_forces_max_completion_tokens` (577–580),
//! `base_url_origin` (583–600), `base_url_host_matches` (603–611).

use url::Url;

/// `_parse_base_url`: `urlparse` that tolerates a bare `host[:port][/path]`
/// (no scheme). Scheme-less inputs are parsed through a placeholder scheme
/// that carries NO known default port, so explicit ports survive
/// normalization — the placeholder's flag is returned so `base_url_origin`
/// can report the empty scheme upstream's `urlparse("//…")` yields.
fn parse_base_url(base_url: &str) -> Option<(bool, Url)> {
    let raw = base_url.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.contains("://") {
        Url::parse(raw).ok().map(|u| (true, u))
    } else {
        // `hermes-bare` has no default port in the url crate's known-scheme
        // table, so `//h:80` keeps its explicit port (urlparse semantics).
        Url::parse(&format!("hermes-bare://{raw}"))
            .ok()
            .map(|u| (false, u))
    }
}

/// `_hostname_of`: lowercased hostname, trailing dots stripped.
fn hostname_of(parsed: Option<&Url>) -> String {
    parsed
        .and_then(|u| u.host_str())
        .unwrap_or("")
        .to_lowercase()
        .trim_end_matches('.')
        .to_string()
}

/// Return the lowercased hostname for a base URL, or `""` if absent.
///
/// Use exact-hostname comparisons against known provider hosts instead of
/// substring matches on the raw URL (which false-positive on
/// `https://api.openai.com.example/v1` or `https://proxy.test/api.openai.com/v1`).
///
/// PARITY: `base_url_hostname` (567–574).
pub fn base_url_hostname(base_url: &str) -> String {
    let Some((_had_scheme, parsed)) = parse_base_url(base_url) else {
        return String::new();
    };
    hostname_of(Some(&parsed))
}

/// `(scheme, hostname, effective_port)` for a base URL;
/// `("", "", 0)` on no host / bad port.
///
/// Origin, not just host: `https://h` vs `http://h` and two ports on one
/// host are different trust boundaries, so handing a bearer secret to a new
/// URL must compare all three — hostname alone would authorise an
/// HTTPS→HTTP downgrade. Port defaults to 443/80 so `https://h` equals
/// `https://h:443`.
///
/// PARITY: `base_url_origin` (583–600).
pub fn base_url_origin(base_url: &str) -> (String, String, u16) {
    let Some((had_scheme, parsed)) = parse_base_url(base_url) else {
        return (String::new(), String::new(), 0);
    };
    let hostname = hostname_of(Some(&parsed));
    if hostname.is_empty() {
        return (String::new(), String::new(), 0);
    }
    let scheme = if had_scheme {
        parsed.scheme().to_lowercase()
    } else {
        String::new()
    };
    // Out-of-range ports make Url::parse fail above (upstream ValueError →
    // ("", "", 0)). `parsed.port()` is the explicit port (known-default
    // ports are normalized away by the url crate — the default fill below
    // restores them, matching urlparse's `port` property).
    let port = match parsed.port_or_known_default() {
        Some(p) => p,
        None => match scheme.as_str() {
            "https" => 443,
            "http" => 80,
            _ => 0,
        },
    };
    let effective = parsed.port().unwrap_or(port);
    (scheme, hostname, effective)
}

/// Return True when the base URL's hostname is `domain` or a subdomain.
///
/// PARITY: `base_url_host_matches` (603–611).
pub fn base_url_host_matches(base_url: &str, domain: &str) -> bool {
    let hostname = base_url_hostname(base_url);
    if hostname.is_empty() {
        return false;
    }
    let domain = domain
        .trim()
        .to_lowercase()
        .trim_end_matches('.')
        .to_string();
    if domain.is_empty() {
        return false;
    }
    hostname == domain || hostname.ends_with(&format!(".{domain}"))
}

/// Return True for model families that require `max_completion_tokens`.
///
/// OpenAI's newer families reject `max_tokens` on `/v1/chat/completions`.
/// Handles vendor prefixes by stripping to the tail after the last `/`.
///
/// PARITY: `model_forces_max_completion_tokens` (577–580).
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
        assert_eq!(
            base_url_hostname("https://api.openai.com/v1"),
            "api.openai.com"
        );
        assert_eq!(
            base_url_hostname("https://api.openai.com.example/v1"),
            "api.openai.com.example"
        );
        assert_eq!(base_url_hostname("api.moonshot.ai/v1"), "api.moonshot.ai");
        assert_eq!(base_url_hostname(""), "");
        assert_eq!(base_url_hostname("   "), "");
    }

    #[test]
    fn host_matches() {
        assert!(base_url_host_matches(
            "https://api.moonshot.ai/v1",
            "moonshot.ai"
        ));
        assert!(base_url_host_matches("https://moonshot.ai", "moonshot.ai"));
        assert!(!base_url_host_matches(
            "https://evil.com/moonshot.ai/v1",
            "moonshot.ai"
        ));
        assert!(!base_url_host_matches(
            "https://moonshot.ai.evil/v1",
            "moonshot.ai"
        ));
        assert!(!base_url_host_matches("", "moonshot.ai"));
    }

    #[test]
    fn max_completion_token_families() {
        for m in [
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4.1",
            "gpt-4.1-nano",
            "gpt-5",
            "gpt-5.4",
            "o1",
            "o1-preview",
            "o3",
            "o3-mini",
            "o4-mini",
            "openai/gpt-5.4",
        ] {
            assert!(model_forces_max_completion_tokens(m), "{}", m);
        }
        for m in [
            "gpt-3.5-turbo",
            "gpt-4",
            "claude-opus-4.5",
            "",
            "anthropic/claude-sonnet-4-5",
        ] {
            assert!(!model_forces_max_completion_tokens(m), "{}", m);
        }
        // Upstream uses startswith, so "o1x" DOES match (Python str.startswith).
        assert!(
            model_forces_max_completion_tokens("o1x"),
            "upstream startswith semantics"
        );
    }
}
