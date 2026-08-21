//! First-party module identification (partial-update diagnostics).
//!
//! PARITY: hermes_constants.py lines 1406–1440 (`FIRST_PARTY_MODULE_ROOTS`,
//! `is_first_party_module`).

/// Top-level packages/modules that ship as part of Hermes itself. An
/// ImportError naming one of these means our own tree is inconsistent;
/// anything else is a third-party problem with different remediation.
pub const FIRST_PARTY_MODULE_ROOTS: [&str; 13] = [
    "agent", "acp_adapter", "cli", "cron", "gateway", "model_tools",
    "plugins", "providers", "tools", "toolsets", "run_agent", "tui_gateway",
    "utils",
];

/// True when `name` is a module that ships with Hermes.
///
/// Matches on the first dotted segment against an exact set — a substring or
/// `startswith` test would also claim third-party `agents`, `agentops`, and
/// `toolsets_x`.
///
/// PARITY: hermes_constants.py `is_first_party_module` (1421–1431).
pub fn is_first_party_module(name: Option<&str>) -> bool {
    let root = match name {
        Some(n) => n.split('.').next().unwrap_or(""),
        None => "",
    };
    if root.is_empty() {
        return false;
    }
    FIRST_PARTY_MODULE_ROOTS.contains(&root) || root.starts_with("hermes_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_party_names() {
        assert!(is_first_party_module(Some("agent.conversation_loop")));
        assert!(is_first_party_module(Some("cli")));
        assert!(is_first_party_module(Some("hermes_state")));
        assert!(is_first_party_module(Some("hermes_cli.main")));
        assert!(is_first_party_module(Some("tools.browser_tool")));
    }

    #[test]
    fn third_party_or_malformed() {
        assert!(!is_first_party_module(Some("agents")));
        assert!(!is_first_party_module(Some("agentops")));
        assert!(!is_first_party_module(Some("toolsets_x")));
        assert!(!is_first_party_module(Some("requests")));
        assert!(!is_first_party_module(None));
        assert!(!is_first_party_module(Some("")));
    }
}
