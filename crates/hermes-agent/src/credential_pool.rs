//! Deterministic credential-pool selection and rotation from
//! `agent/credential_pool.py`.
//!
//! This first slice keeps persistence, OAuth refresh, environment seeding,
//! and cross-process locking out of the core state machine. Callers provide
//! the already loaded entries and an observation timestamp; the selection and
//! failure semantics remain identical to the source's in-memory pool.

/// Pool entry status values persisted by the Python implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialStatus {
    Ok,
    Exhausted,
    Dead,
}

/// Selection strategies supported by the deterministic core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolStrategy {
    FillFirst,
    RoundRobin,
    LeastUsed,
}

/// A loaded credential-pool row and the fields used by selection/rotation.
#[derive(Debug, Clone, PartialEq)]
pub struct PooledCredential {
    pub provider: String,
    pub id: String,
    pub label: String,
    pub auth_type: String,
    pub priority: i32,
    pub source: String,
    pub access_token: String,
    pub status: Option<CredentialStatus>,
    pub last_status_at: Option<f64>,
    pub last_error_code: Option<u16>,
    pub last_error_reason: Option<String>,
    pub last_error_message: Option<String>,
    pub last_error_reset_at: Option<f64>,
    pub failure_reason: Option<String>,
    pub request_count: u64,
}

impl PooledCredential {
    /// Construct a fresh API-key-style entry with source defaults.
    pub fn new(provider: &str, id: &str, access_token: &str, priority: i32) -> Self {
        Self {
            provider: provider.into(),
            id: id.into(),
            label: id.into(),
            auth_type: "api_key".into(),
            priority,
            source: "manual".into(),
            access_token: access_token.into(),
            status: None,
            last_status_at: None,
            last_error_code: None,
            last_error_reason: None,
            last_error_message: None,
            last_error_reset_at: None,
            failure_reason: None,
            request_count: 0,
        }
    }
}

/// Error metadata supplied to `mark_exhausted_and_rotate`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoolErrorContext {
    pub reason: Option<String>,
    pub message: Option<String>,
    pub reset_at: Option<f64>,
}

/// In-memory credential pool state.
#[derive(Debug, Clone, PartialEq)]
pub struct CredentialPool {
    provider: String,
    entries: Vec<PooledCredential>,
    strategy: PoolStrategy,
    current_id: Option<String>,
    unmatched_rotation_streak: usize,
}

const EXHAUSTED_TTL_401_SECONDS: f64 = 5.0 * 60.0;
const EXHAUSTED_TTL_429_SECONDS: f64 = 60.0 * 60.0;
const EXHAUSTED_TTL_DEFAULT_SECONDS: f64 = 60.0 * 60.0;
const EXHAUSTED_TTL_SOLE_CREDENTIAL_SECONDS: f64 = 60.0;
const FAILURE_REASON_BILLING: &str = "billing";

const TERMINAL_AUTH_REASONS: &[&str] = &[
    "token_invalidated",
    "token_revoked",
    "invalid_token",
    "invalid_grant",
    "unauthorized_client",
    "refresh_token_reused",
];

impl CredentialPool {
    /// Create a pool sorted by the source priority order.
    ///
    /// PARITY: agent/credential_pool.py `CredentialPool.__init__`.
    pub fn new(provider: &str, mut entries: Vec<PooledCredential>, strategy: PoolStrategy) -> Self {
        entries.sort_by_key(|entry| entry.priority);
        Self {
            provider: provider.into(),
            entries,
            strategy,
            current_id: None,
            unmatched_rotation_streak: 0,
        }
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Return whether the pool has rows, including rows currently exhausted.
    ///
    /// PARITY: agent/credential_pool.py `has_credentials`.
    pub fn has_credentials(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Return a snapshot of the loaded rows.
    pub fn entries(&self) -> Vec<PooledCredential> {
        self.entries.clone()
    }

    /// Return the currently selected row without changing selection state.
    ///
    /// PARITY: agent/credential_pool.py `current`.
    pub fn current(&self) -> Option<PooledCredential> {
        self.current_id
            .as_deref()
            .and_then(|id| self.entries.iter().find(|entry| entry.id == id))
            .cloned()
    }

    /// Return the current row, or the first available row if nothing is
    /// selected. Expired cooldowns are not rewritten by a peek.
    ///
    /// PARITY: agent/credential_pool.py `peek`.
    pub fn peek(&self, now: f64) -> Option<PooledCredential> {
        if let Some(current) = self.current() {
            return Some(current);
        }
        self.available_indices_readonly(now)
            .first()
            .and_then(|index| self.entries.get(*index))
            .cloned()
    }

    /// Return whether any entry is available at `now`.
    ///
    /// PARITY: agent/credential_pool.py `has_available`.
    pub fn has_available(&self, now: f64) -> bool {
        !self.available_indices_readonly(now).is_empty()
    }

    /// Select a credential according to the configured strategy.
    ///
    /// `now` is explicit so Rust callers can test cooldown boundaries without
    /// patching process-global wall-clock state.
    ///
    /// PARITY: agent/credential_pool.py `_select_unlocked` / `select`.
    pub fn select(&mut self, now: f64) -> Option<PooledCredential> {
        let available = self.available_indices(now, true);
        self.select_from_available(available)
    }

    /// Mark the failed credential and return the next available credential.
    ///
    /// Credential identity takes precedence over the current cursor. An
    /// unmatched identity rotates without quarantining an innocent key, while
    /// a matched key quarantines every duplicate row backed by that key.
    ///
    /// PARITY: agent/credential_pool.py lines 2031-2165.
    #[allow(clippy::too_many_arguments)]
    pub fn mark_exhausted_and_rotate(
        &mut self,
        status_code: Option<u16>,
        error_context: Option<&PoolErrorContext>,
        api_key_hint: Option<&str>,
        credential_id: Option<&str>,
        failure_reason: Option<&str>,
        now: f64,
    ) -> Option<PooledCredential> {
        let identity_supplied = credential_id.is_some() || api_key_hint.is_some();
        let selected_index = credential_id
            .and_then(|id| self.entries.iter().position(|entry| entry.id == id))
            .or_else(|| {
                api_key_hint.and_then(|key| {
                    self.entries
                        .iter()
                        .position(|entry| entry.access_token == key)
                })
            });

        let Some(selected_index) = selected_index.or_else(|| {
            if identity_supplied {
                None
            } else {
                self.current_id
                    .as_deref()
                    .and_then(|id| self.entries.iter().position(|entry| entry.id == id))
                    .or_else(|| self.available_indices(now, false).first().copied())
            }
        }) else {
            if !identity_supplied {
                self.current_id = None;
                return None;
            }

            self.unmatched_rotation_streak += 1;
            let available = self.available_indices(now, false);
            if self.unmatched_rotation_streak > available.len().max(1) {
                self.unmatched_rotation_streak = 0;
                self.current_id = None;
                return None;
            }
            self.current_id = None;
            let next = self.select(now);
            if next.is_some() && self.available_indices_readonly(now).len() == 1 {
                self.unmatched_rotation_streak = 0;
                self.current_id = None;
                return None;
            }
            return next;
        };

        self.unmatched_rotation_streak = 0;
        let failed_key = self.entries[selected_index].access_token.clone();
        let duplicate_key = identity_supplied && !failed_key.is_empty();
        for index in 0..self.entries.len() {
            if index != selected_index
                && (!duplicate_key || self.entries[index].access_token != failed_key)
            {
                continue;
            }
            self.mark_entry(index, status_code, error_context, failure_reason, now);
        }
        self.current_id = None;
        self.select(now)
    }

    fn mark_entry(
        &mut self,
        index: usize,
        status_code: Option<u16>,
        error_context: Option<&PoolErrorContext>,
        failure_reason: Option<&str>,
        now: f64,
    ) {
        let entry = &mut self.entries[index];
        let reason = error_context
            .and_then(|context| context.reason.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let terminal = status_code == Some(401)
            && reason.as_deref().is_some_and(|value| {
                TERMINAL_AUTH_REASONS
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(value))
            });
        entry.status = Some(if terminal {
            CredentialStatus::Dead
        } else {
            CredentialStatus::Exhausted
        });
        entry.last_status_at = Some(now);
        entry.last_error_code = status_code;
        entry.last_error_reason = reason;
        entry.last_error_message = error_context
            .and_then(|context| context.message.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        entry.last_error_reset_at = error_context.and_then(|context| context.reset_at);
        entry.failure_reason = failure_reason
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
    }

    fn select_from_available(&mut self, available: Vec<usize>) -> Option<PooledCredential> {
        let index = match self.strategy {
            PoolStrategy::FillFirst | PoolStrategy::RoundRobin => *available.first()?,
            PoolStrategy::LeastUsed => *available
                .iter()
                .min_by_key(|index| self.entries[**index].request_count)?,
        };

        if self.strategy == PoolStrategy::LeastUsed && available.len() > 1 {
            self.entries[index].request_count = self.entries[index].request_count.saturating_add(1);
        }

        if self.strategy == PoolStrategy::RoundRobin && available.len() > 1 {
            let selected = self.entries.remove(index);
            let selected_id = selected.id.clone();
            self.entries.push(selected);
            for (priority, entry) in self.entries.iter_mut().enumerate() {
                entry.priority = priority as i32;
            }
            self.current_id = Some(selected_id);
            return self.current();
        }

        self.current_id = Some(self.entries[index].id.clone());
        self.entries.get(index).cloned()
    }

    fn available_indices(&mut self, now: f64, clear_expired: bool) -> Vec<usize> {
        let sole_credential = self
            .entries
            .iter()
            .filter(|entry| entry.status != Some(CredentialStatus::Dead))
            .count()
            <= 1;
        let mut available = Vec::new();
        for index in 0..self.entries.len() {
            let entry = &mut self.entries[index];
            if (entry.auth_type == "api_key" && entry.access_token.trim().is_empty())
                || entry.status == Some(CredentialStatus::Dead)
            {
                continue;
            }
            if entry.status == Some(CredentialStatus::Exhausted) {
                if let Some(until) = exhausted_until(entry, sole_credential) {
                    if now < until {
                        continue;
                    }
                }
                if clear_expired {
                    clear_failure_state(entry);
                }
            }
            available.push(index);
        }
        available
    }

    fn available_indices_readonly(&self, now: f64) -> Vec<usize> {
        let sole_credential = self
            .entries
            .iter()
            .filter(|entry| entry.status != Some(CredentialStatus::Dead))
            .count()
            <= 1;
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if (entry.auth_type == "api_key" && entry.access_token.trim().is_empty())
                    || entry.status == Some(CredentialStatus::Dead)
                {
                    return None;
                }
                if entry.status == Some(CredentialStatus::Exhausted)
                    && exhausted_until(entry, sole_credential).is_some_and(|until| now < until)
                {
                    return None;
                }
                Some(index)
            })
            .collect()
    }
}

fn clear_failure_state(entry: &mut PooledCredential) {
    entry.status = Some(CredentialStatus::Ok);
    entry.last_status_at = None;
    entry.last_error_code = None;
    entry.last_error_reason = None;
    entry.last_error_message = None;
    entry.last_error_reset_at = None;
    entry.failure_reason = None;
}

fn exhausted_until(entry: &PooledCredential, sole_credential: bool) -> Option<f64> {
    if entry.status != Some(CredentialStatus::Exhausted) {
        return None;
    }
    if let Some(reset_at) = entry.last_error_reset_at {
        return Some(reset_at);
    }
    let status_at = entry.last_status_at?;
    let base = match entry.last_error_code {
        Some(401) => EXHAUSTED_TTL_401_SECONDS,
        Some(429) => EXHAUSTED_TTL_429_SECONDS,
        _ => EXHAUSTED_TTL_DEFAULT_SECONDS,
    };
    let billing = entry.last_error_code == Some(402)
        || entry.failure_reason.as_deref() == Some(FAILURE_REASON_BILLING);
    let ttl = if sole_credential && !billing {
        base.min(EXHAUSTED_TTL_SOLE_CREDENTIAL_SECONDS)
    } else {
        base
    };
    Some(status_at + ttl)
}
