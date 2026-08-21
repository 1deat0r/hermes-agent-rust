//! Proxy URL normalization.
//!
//! PARITY: utils.py lines 557–588 (`_PROXY_ENV_KEYS`, `normalize_proxy_url`,
//! `normalize_proxy_env_vars`).

/// Supported proxy env keys (uppercase + lowercase forms).
pub const PROXY_ENV_KEYS: [&str; 6] = [
    "HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY",
    "https_proxy", "http_proxy", "all_proxy",
];

/// Normalize proxy URLs for httpx/aiohttp compatibility.
///
/// WSL/Clash-style environments often export SOCKS proxies as
/// `socks://127.0.0.1:PORT`. httpx rejects that alias and expects the
/// explicit `socks5://` scheme instead.
///
/// PARITY: utils.py `normalize_proxy_url` (566–578).
pub fn normalize_proxy_url(proxy_url: Option<&str>) -> Option<String> {
    let candidate = proxy_url.unwrap_or("").trim().to_string();
    if candidate.is_empty() {
        return None;
    }
    if candidate.to_lowercase().starts_with("socks://") {
        return Some(format!("socks5://{}", &candidate["socks://".len()..]));
    }
    Some(candidate)
}

/// Rewrite supported proxy env vars to canonical URL forms in-place.
///
/// PARITY: utils.py `normalize_proxy_env_vars` (581–587).
pub fn normalize_proxy_env_vars() {
    for key in PROXY_ENV_KEYS {
        if let Ok(value) = std::env::var(key) {
            if let Some(normalized) = normalize_proxy_url(Some(&value)) {
                if normalized != value {
                    unsafe { std::env::set_var(key, normalized) };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn socks_alias_rewritten() {
        assert_eq!(
            normalize_proxy_url(Some("socks://127.0.0.1:1080")),
            Some("socks5://127.0.0.1:1080".into())
        );
        assert_eq!(
            normalize_proxy_url(Some("http://proxy:8080")),
            Some("http://proxy:8080".into())
        );
        assert_eq!(normalize_proxy_url(Some("  ")), None);
        assert_eq!(normalize_proxy_url(None), None);
    }

    #[test]
    fn env_normalization_in_place() {
        let _g = ENV_MUTEX.lock().unwrap();
        unsafe { std::env::set_var("ALL_PROXY", "socks://127.0.0.1:1080") };
        unsafe { std::env::set_var("HTTP_PROXY", "http://ok:80") };
        normalize_proxy_env_vars();
        assert_eq!(std::env::var("ALL_PROXY").unwrap(), "socks5://127.0.0.1:1080");
        assert_eq!(std::env::var("HTTP_PROXY").unwrap(), "http://ok:80");
        unsafe { std::env::remove_var("ALL_PROXY") };
        unsafe { std::env::remove_var("HTTP_PROXY") };
    }
}
