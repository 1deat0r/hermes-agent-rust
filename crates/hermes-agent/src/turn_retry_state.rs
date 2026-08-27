//! Per-attempt recovery bookkeeping for the conversation turn loop.
//!
//! PARITY: `agent/turn_retry_state.py` @ b9aa928 (whole module, lines 1-93).
//!
//! The inner retry loop in `run_conversation` (`while retry_count <
//! max_retries`) makes several distinct recovery attempts on a single model
//! API call: a credential-pool 429 retry, a per-provider OAuth refresh, a
//! long-context compression restart, a length-continuation restart, and a
//! handful of format-recovery branches. Each branch is guarded by a one-shot
//! boolean so it fires at most once per attempt.
//!
//! These used to be ~16 bare `*_attempted` / `has_retried_*` / `restart_with_*`
//! locals threaded through the loop body; [`TurnRetryState`] collapses them
//! into one object the loop mutates in place, giving the recovery bookkeeping a
//! single named, testable home. Loop-control variables (`retry_count`,
//! `max_retries`, `max_compression_attempts`) intentionally stay plain locals —
//! they are the `while` mechanics, not recovery bookkeeping (upstream module
//! docstring, lines 20-27).
//!
//! A fresh instance is created for each iteration of the outer turn loop (once
//! per `api_call_count`); the `restart_with_*` signals are read by the loop
//! after the attempt to decide whether to rebuild the request and retry.

/// The complete field contract in upstream declaration order, mirroring
/// `dataclasses.fields(TurnRetryState)`.
pub const TURN_RETRY_STATE_FIELDS: [&str; 20] = [
    "codex_auth_retry_attempted",
    "anthropic_auth_retry_attempted",
    "nous_auth_retry_attempted",
    "nous_paid_entitlement_refresh_attempted",
    "copilot_auth_retry_attempted",
    // Copilot surfaces a stale/degraded credential as a 400
    // `model_not_available_for_integrator` / `model_not_supported` instead of a
    // clean 401. This single-shot forced re-exchange guard is separate from the
    // 401 guard so both can fire within one attempt if needed.
    "copilot_stale_cred_retry_attempted",
    "vertex_auth_retry_attempted",
    "thinking_sig_retry_attempted",
    "invalid_encrypted_content_retry_attempted",
    "image_shrink_retry_attempted",
    "multimodal_tool_content_retry_attempted",
    "oauth_1m_beta_retry_attempted",
    "llama_cpp_grammar_retry_attempted",
    "primary_recovery_attempted",
    "has_retried_429",
    // Set once a persistent 401/403 has been escalated to the fallback chain
    // after the per-provider credential refresh above failed, so the same auth
    // failover never loops within one attempt.
    "auth_failover_attempted",
    "restart_with_compressed_messages",
    "restart_with_length_continuation",
    // A content-filter stream stall escalated to the fallback chain: the
    // partial-stream content was rolled back off `messages` and the loop should
    // re-issue the API call against the newly-activated provider (#32421).
    "restart_with_rebuilt_messages",
    // A user correction cancelled the in-flight provider request; the loop must
    // append a role-safe checkpoint + user message and retry the same logical
    // iteration.
    "restart_with_redirected_messages",
];

/// One-shot recovery guards plus restart signals for a single API-call attempt.
///
/// PARITY: `TurnRetryState` (upstream lines 34-92). Every field defaults to
/// `False`; the declaration order matches upstream so [`TurnRetryState::iter`]
/// yields the same `(name, value)` sequence as the dataclass `__iter__`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TurnRetryState {
    // ── Per-provider OAuth / credential refresh guards ───────────────────
    pub codex_auth_retry_attempted: bool,
    pub anthropic_auth_retry_attempted: bool,
    pub nous_auth_retry_attempted: bool,
    pub nous_paid_entitlement_refresh_attempted: bool,
    pub copilot_auth_retry_attempted: bool,
    pub copilot_stale_cred_retry_attempted: bool,
    pub vertex_auth_retry_attempted: bool,

    // ── Format / payload recovery guards ─────────────────────────────────
    pub thinking_sig_retry_attempted: bool,
    pub invalid_encrypted_content_retry_attempted: bool,
    pub image_shrink_retry_attempted: bool,
    pub multimodal_tool_content_retry_attempted: bool,
    pub oauth_1m_beta_retry_attempted: bool,
    pub llama_cpp_grammar_retry_attempted: bool,

    // ── Transport / rate-limit recovery ──────────────────────────────────
    pub primary_recovery_attempted: bool,
    pub has_retried_429: bool,

    // ── Auth-failure provider failover ───────────────────────────────────
    pub auth_failover_attempted: bool,

    // ── Restart signals (read by the outer loop after the attempt) ───────
    pub restart_with_compressed_messages: bool,
    pub restart_with_length_continuation: bool,
    pub restart_with_rebuilt_messages: bool,
    pub restart_with_redirected_messages: bool,
}

impl TurnRetryState {
    /// Field names in declaration order.
    ///
    /// PARITY: `dataclasses.fields(TurnRetryState)` names.
    pub fn field_names() -> impl Iterator<Item = &'static str> {
        TURN_RETRY_STATE_FIELDS.iter().copied()
    }

    /// Iterate `(name, value)` pairs in declaration order.
    ///
    /// PARITY: `TurnRetryState.__iter__` (upstream lines 90-93), the
    /// debugging/test convenience that walks `fields(self)`.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, bool)> {
        // The state is `Copy`, so the iterator owns its snapshot instead of
        // capturing a borrow that cannot outlive this call.
        let state = *self;
        Self::field_names().map(move |name| (name, state.get(name)))
    }

    /// Read one guard by its upstream field name.
    ///
    /// Rust has no `getattr`, so the name-keyed access the dataclass provides
    /// is an explicit lookup; unknown names read as `false`, which matches the
    /// loop's "guard not yet fired" default and never panics.
    pub fn get(&self, name: &str) -> bool {
        match name {
            "codex_auth_retry_attempted" => self.codex_auth_retry_attempted,
            "anthropic_auth_retry_attempted" => self.anthropic_auth_retry_attempted,
            "nous_auth_retry_attempted" => self.nous_auth_retry_attempted,
            "nous_paid_entitlement_refresh_attempted" => {
                self.nous_paid_entitlement_refresh_attempted
            }
            "copilot_auth_retry_attempted" => self.copilot_auth_retry_attempted,
            "copilot_stale_cred_retry_attempted" => self.copilot_stale_cred_retry_attempted,
            "vertex_auth_retry_attempted" => self.vertex_auth_retry_attempted,
            "thinking_sig_retry_attempted" => self.thinking_sig_retry_attempted,
            "invalid_encrypted_content_retry_attempted" => {
                self.invalid_encrypted_content_retry_attempted
            }
            "image_shrink_retry_attempted" => self.image_shrink_retry_attempted,
            "multimodal_tool_content_retry_attempted" => {
                self.multimodal_tool_content_retry_attempted
            }
            "oauth_1m_beta_retry_attempted" => self.oauth_1m_beta_retry_attempted,
            "llama_cpp_grammar_retry_attempted" => self.llama_cpp_grammar_retry_attempted,
            "primary_recovery_attempted" => self.primary_recovery_attempted,
            "has_retried_429" => self.has_retried_429,
            "auth_failover_attempted" => self.auth_failover_attempted,
            "restart_with_compressed_messages" => self.restart_with_compressed_messages,
            "restart_with_length_continuation" => self.restart_with_length_continuation,
            "restart_with_rebuilt_messages" => self.restart_with_rebuilt_messages,
            "restart_with_redirected_messages" => self.restart_with_redirected_messages,
            _ => false,
        }
    }

    /// Set one guard by its upstream field name; returns whether the name was
    /// part of the contract (the `false` case is a typo, mirroring Python's
    /// `AttributeError` on an unknown attribute assignment).
    pub fn set(&mut self, name: &str, value: bool) -> bool {
        let slot = match name {
            "codex_auth_retry_attempted" => Some(&mut self.codex_auth_retry_attempted),
            "anthropic_auth_retry_attempted" => Some(&mut self.anthropic_auth_retry_attempted),
            "nous_auth_retry_attempted" => Some(&mut self.nous_auth_retry_attempted),
            "nous_paid_entitlement_refresh_attempted" => {
                Some(&mut self.nous_paid_entitlement_refresh_attempted)
            }
            "copilot_auth_retry_attempted" => Some(&mut self.copilot_auth_retry_attempted),
            "copilot_stale_cred_retry_attempted" => {
                Some(&mut self.copilot_stale_cred_retry_attempted)
            }
            "vertex_auth_retry_attempted" => Some(&mut self.vertex_auth_retry_attempted),
            "thinking_sig_retry_attempted" => Some(&mut self.thinking_sig_retry_attempted),
            "invalid_encrypted_content_retry_attempted" => {
                Some(&mut self.invalid_encrypted_content_retry_attempted)
            }
            "image_shrink_retry_attempted" => Some(&mut self.image_shrink_retry_attempted),
            "multimodal_tool_content_retry_attempted" => {
                Some(&mut self.multimodal_tool_content_retry_attempted)
            }
            "oauth_1m_beta_retry_attempted" => Some(&mut self.oauth_1m_beta_retry_attempted),
            "llama_cpp_grammar_retry_attempted" => {
                Some(&mut self.llama_cpp_grammar_retry_attempted)
            }
            "primary_recovery_attempted" => Some(&mut self.primary_recovery_attempted),
            "has_retried_429" => Some(&mut self.has_retried_429),
            "auth_failover_attempted" => Some(&mut self.auth_failover_attempted),
            "restart_with_compressed_messages" => Some(&mut self.restart_with_compressed_messages),
            "restart_with_length_continuation" => Some(&mut self.restart_with_length_continuation),
            "restart_with_rebuilt_messages" => Some(&mut self.restart_with_rebuilt_messages),
            "restart_with_redirected_messages" => Some(&mut self.restart_with_redirected_messages),
            _ => None,
        };
        match slot {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }
}
