//! Server-rendered /login page decision shapes.
//!
//! PARITY: `hermes_cli/dashboard_auth/login_page.py` @ 5d59366. The CSS
//! templates (`_LOGIN_HTML_TEMPLATE`, `_EMPTY_HTML`,
//! `_PASSWORD_FORM_SCRIPT`) ride with the web-server surface that serves
//! them; what ports here is the XSS-escaping contract and the render
//! decisions (empty roster → empty page, password-vs-OAuth branch,
//! `next` query construction), each pinned by tests.
//!
//! Escaping rule (upstream `render_login_html`): every interpolated
//! value is `html.escape`d — provider names/queries with `quote=True`
//! (attribute context), display names without (element context).
//! `next_path` is URL-encoded THEN HTML-escaped, matching the gate's
//! `_safe_next_target` shape so a round-tripped value is byte-identical.

use super::middleware::percent_encode_all;

/// HTML-escape for element vs attribute contexts (Python `html.escape`
/// with `quote=False` / `quote=True`).
pub fn html_escape(text: &str, quote: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if quote => out.push_str("&quot;"),
            '\'' if quote => out.push_str("&#x27;"),
            c => out.push(c),
        }
    }
    out
}

/// The `next` query fragment for provider buttons: URL-encoded then
/// HTML-escaped (attribute context), byte-identical to the gate's
/// `_safe_next_target` shape on round trip.
pub fn next_query_fragment(next_path: &str) -> String {
    if next_path.is_empty() {
        return String::new();
    }
    format!(
        "&next={}",
        html_escape(&percent_encode_all(next_path), true)
    )
}

/// One provider's login control: OAuth anchor or password form marker.
/// The web surface expands the marker with the (static) form template.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderControl {
    OAuth { href: String, label: String },
    Password { provider: String },
}

/// Build the login controls for the session-provider roster.
/// Empty roster → no controls (the web surface renders `_EMPTY_HTML`).
pub fn login_controls(
    providers: &[(String, String, bool)],
    next_path: &str,
) -> Vec<ProviderControl> {
    let next_qs = next_query_fragment(next_path);
    providers
        .iter()
        .map(|(name, display_name, supports_password)| {
            if *supports_password {
                ProviderControl::Password {
                    provider: name.clone(),
                }
            } else {
                ProviderControl::OAuth {
                    href: format!("/auth/login?provider={}{next_qs}", html_escape(name, true)),
                    label: html_escape(display_name, false),
                }
            }
        })
        .collect()
}

/// Provider picker for a native authorize request with more than one
/// interactive provider. Every link re-enters
/// `/auth/native/authorize` with the SAME desktop PKCE inputs plus an
/// explicit `provider`, so the choice never leaves the validated
/// native flow.
pub fn native_choice_hrefs(
    providers: &[(String, String)],
    authorize_path: &str,
    pkce: &[(String, String)],
) -> Vec<(String, String)> {
    providers
        .iter()
        .map(|(name, display_name)| {
            let mut query: Vec<(String, String)> = pkce.to_vec();
            query.push(("provider".to_string(), name.clone()));
            let query_string = query
                .iter()
                .map(|(k, v)| format!("{}={}", percent_encode_all(k), percent_encode_all(v)))
                .collect::<Vec<_>>()
                .join("&");
            (
                html_escape(&format!("{authorize_path}?{query_string}"), true),
                html_escape(display_name, false),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_contexts_match_python_html_escape() {
        assert_eq!(
            html_escape("<a href=\"x\">&", false),
            "&lt;a href=\"x\"&gt;&amp;"
        );
        assert_eq!(
            html_escape("<a href=\"x\">&", true),
            "&lt;a href=&quot;x&quot;&gt;&amp;"
        );
        assert_eq!(html_escape("it's", true), "it&#x27;s");
        assert_eq!(html_escape("it's", false), "it's");
    }
}
