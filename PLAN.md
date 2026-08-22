# Hermes Agent in Rust — 1:1 Port Plan

Target: https://github.com/NousResearch/hermes-agent @ `b9aa928` (local clone:
`/home/mustbearn/Projects/Research/hermes-agent-repo`)
Goal: Functional **1:1 port** of Hermes Agent to idiomatic Rust — same CLI
surface, same on-disk formats, same wire protocols, same observable behavior —
different implementation language.

## 0. Governance — standing process rules

1. **Reassess after every phase.** No phase is "done" until this file is
   updated: phase status, criterion evidence, issues found. A stale plan is a
   process failure.
2. **Line-by-line expert assessment.** Each phase's ported code is assessed
   line by line against upstream: correctness, error paths, lifecycle,
   test quality — not just "does it compile".
3. **Evidence tiers, never blurred.** Every criterion claim carries a tier:
   `unit` | `mock` | `live` — plus the exact command that produced it.
4. **Parity oracles.** Any upstream behavior with observable semantics is
   pinned by tests that mirror upstream tests (vendored where possible; see
   `upstream/`). Guessing at upstream behavior is a defect, not a shortcut.
5. **One module per commit at most**; every commit must build and test green.
6. **Coverage ledger stays current.** After each phase, regenerate the
   inventory (`tools/inventory.sh`) and update the parity matrix below.
   A stale ledger is a process failure.

## 1. What Hermes is

Hermes Agent is a self-improving AI agent by Nous Research: skills that are
created and refined from experience, agent-curated memory, FTS5 session
search, cron scheduling, delegation/subagents, seven terminal backends,
multi-provider LLM support, and a multi-platform messaging gateway
(Telegram/Discord/Slack/WhatsApp/Signal/matrix/feishu/google_chat + CLI/TUI/ACP).

Entry points (pyproject `[project.scripts]`):
- `hermes`        → `hermes_cli.main:main`
- `hermes-agent`  → `run_agent:main`
- `hermes-acp`    → `acp_adapter.entry:main`

### Upstream inventory (current, pinned @ b9aa928)

| Area | ~LOC | Modules | Port status |
|---|---|---|---|
| tests | 665,706 | — | oracle source only |
| hermes_cli/ | 207,724 | 266 | ❌ Phase 3 |
| agent/ | 133,942 | 186 | ❌ Phase 2 |
| plugins/ | 124,195 | 181 | ❌ Phase 4 |
| tools/ | 123,020 | 131 | ❌ Phase 2 |
| gateway/ | 101,551 | 88 | ❌ Phase 4 |
| top-level .py (cli, run_agent, hermes_state, …) | 52,598 | 53 | 🟡 Phase 1/2 |
| tui_gateway/ | 25,883 | 22 | ❌ Phase 5 |
| skills (bundled) | 18,293 | 47 | ❌ (runtime data) |
| scripts/ | 15,836 | 14 | ❌ later |
| cron/ | 11,324 | 13 | ❌ Phase 4 |
| acp_adapter/ | 5,832 | 11 | ❌ Phase 5 |
| providers/ | 436 | 3 | ❌ Phase 2 |
| **Production** | **~843,792** | **1,103** | |

Full machine-readable inventory: `tools/inventory.json` (regenerate with
`tools/inventory.sh`). The authoritative per-module port ledger lives there.

## 2. Fidelity model — what "1:1" means

Not a line-for-line transpile (Python idioms → Rust require different code).
Fidelity is defined at the *observable contract* level:

1. **CLI surface**: same binary `hermes`, same commands, flags, help, exit codes.
2. **On-disk formats**: config.yaml schema + loaders, `~/.hermes` layout,
   state DB (SQLite schema), session files, skill/plugin dirs, kanban DB.
3. **Wire protocols**: gateway platform APIs (Telegram/Discord/Slack/etc.),
   MCP (stdio + HTTP), ACP, cron scheduling semantics.
4. **LLM provider behavior**: same providers/APIs, same message/stream shapes.
5. **Tool behavior**: same tool names/schemas/output formats.
6. **Behavioral equivalence**: same timezone resolution order, same path
   resolution rules, same env-var precedence, same fail-open behaviors.

**Non-goals**: reproducing packaging (uv/npm), Python's exact docstrings,
Pydantic metaprogramming, internal error types that aren't observable.

## 3. Crate architecture

Cargo workspace; crates mirror upstream dependency layers (bottom-up).

```
Hermes-Agent-Rust/
  Cargo.toml
  crates/
    hermes-constants/      # hermes_constants.py              (foundational subset ✅, node/profile pending)
    hermes-time/           # hermes_time.py                   (✅ complete)
    hermes-utils/          # utils.py                         (Phase 1)
    hermes-logging/        # hermes_logging.py                (Phase 1)
    hermes-state/          # hermes_state{,_schema,_common,_portability,_search}.py  (Phase 1, open — common/schema/lifecycle landed)
    hermes-toolsets/       # toolsets.py ✅, toolset_distributions.py ✅, model_tools.py 🟡 (schema/coercion surface landed; handle_function_call deferred with agent loop)   (Phase 2)
    hermes-agent/          # run_agent.py + agent/            (Phase 2, next after toolsets)
    hermes-tools/          # tools/ (✅ registry, schema_sanitizer, ansi_strip, clarify_tool, session_search_tool — more tool files scheduled)   (Phase 2)
    hermes-batch/          # batch_runner.py, trajectory_compressor.py, mini_swe_runner.py (Phase 2)
    hermes-cli/            # cli.py + hermes_cli/             (Phase 3, bin `hermes`)
    hermes-gateway/        # gateway/                         (Phase 4)
    hermes-platforms/      # plugins/                         (Phase 4)
    hermes-cron/           # cron/                            (Phase 4)
    hermes-tui/            # tui_gateway/                     (Phase 5)
    hermes-acp/            # acp_adapter/                     (Phase 5)
  tools/                   # inventory + parity helpers
  scripts/                 # build/parity scripts
  upstream/                # vendored upstream fixtures (golden oracles)
```

Dependency direction is bottom-up; a crate never depends on a higher layer.

## 4. Phases

| Phase | Scope | Exit criterion |
|---|---|---|
| **P0** | Infrastructure, governance, inventory, fidelity model | `cargo build` green; PLAN + ledger current; reviewer sign-off |
| **P1** | Foundation: hermes_constants, hermes_time, utils, logging, state schema/common/state, time | Foundation crates ported + tested; parity vs upstream unit tests |
| **P2** | Agent core: toolsets, model_tools, run_agent, agent/, tools/, batch/trajectory | Agent loop + tools run end-to-end against a model stub |
| **P3** | CLI: cli.py + hermes_cli/ | `hermes` binary matches CLI surface; config load/save parity |
| **P4** | Gateway: gateway/, plugins/, cron | Gateway connects to ≥1 real platform + cron fires |
| **P5** | tui_gateway, acp_adapter, remaining scripts | Full surface parity |

**P1 status: 🟡 FOUNDATION COMPLETE (2026-08-22, session 1u).** hermes-constants
(foundational subset), hermes-time, hermes-utils, hermes-logging (incl.
agent/redact.py), and the full hermes_state* module family (hermes_state.py
9,996 LOC / all 176 SessionDB methods, hermes_state_schema / _common /
_portability / _search) are ported with parity tests. Remaining P1-deferred
items (operator-flagged to P2/P3): apply_ipv4_preference, partial_update_
hint, managed-node bootstrap, agent_browser_runnable, resolve_reasoning_
config (config crate), node/profile constants remainder. Phase 2 (agent
core: toolsets, run_agent, agent/, tools/, batch) is open.

**P0 status: ✅ COMPLETE (2026-08-22).** Exit criteria met:
`cargo build --workspace` green; `cargo test --workspace` green (74 tests:
hermes-constants lib 45 + parity 8, hermes-time lib 13 + parity 8); clippy
clean via `cargo clippy --workspace --all-targets`; PLAN + ledger current
(`tools/inventory.sh` → prod 1,103 modules / 843,792 LOC, status counts
tracked); P1 foundation underway with hermes-constants (foundational subset)
and hermes-time (full) landed. Evidence tier: unit.

## 5. Parity matrix (foundation)

Legend: ✅ done · 🟡 partial · ❌ missing

### hermes_constants (upstream: hermes_constants.py, 1,481 LOC)
| Function/surface | Status | Rust home |
|---|---|---|
| INDICATOR_STYLES, DEFAULT_INDICATOR_STYLE | ✅ | constants::styles |
| set/reset/get_hermes_home_override (ContextVar) | ✅ | constants::home::override_* |
| _get_platform_default_hermes_home | ✅ | constants::home::platform_default_home |
| _hermes_home_from_env | ✅ | constants::home::home_from_env |
| get_hermes_home / get_process_hermes_home | ✅ | constants::home |
| _warn_profile_fallback_once | ✅ | constants::home::warn_profile_fallback_once |
| get_default_hermes_root | ✅ | constants::home::default_hermes_root |
| get_optional_skills_dir / get_optional_mcps_dir / get_bundled_skills_dir | ✅ | constants::paths |
| get_hermes_dir + _legacy_path_has_content | ✅ | constants::paths |
| get_config_path / get_skills_dir / get_env_path | ✅ | constants::paths |
| is_termux / is_wsl / is_container | ✅ | constants::platform |
| windows_path_to_wsl / wsl_unc_path_to_posix / translate_cwd_for_wsl_backend | ✅ | constants::platform |
| VALID_REASONING_EFFORTS, parse_reasoning_effort | ✅ | constants::reasoning |
| _canonical_model_variants | ✅ | constants::reasoning |
| venv_bin_dir / venv_python_path | ✅ | constants::venv |
| FIRST_PARTY_MODULE_ROOTS, is_first_party_module | ✅ | constants::modules |
| PARTIAL_STREAM_STUB_ID, FINISH_REASON_LENGTH, OPENROUTER_*, AI_GATEWAY_BASE_URL | ✅ | constants::values |
| iter_hermes_node_dirs / _candidate_node_command_names | ✅ | platform::iter_hermes_node_dirs / candidate_node_command_names |
| node_tool_runnable / managed-node bootstrap/heal | ❌ deferred (node subsystem, P2) | — |
| agent_browser_runnable | ❌ deferred (P2) | — |
| display_hermes_home / secure_parent_dir | ✅ | paths::{display_hermes_home, secure_parent_dir} |
| profile-home helpers (_profile_home_path, get_real_home, …) | ✅ | home::{norm_home_path, profile_home_path, is_profile_home, iter_real_home_candidates, get_real_home, get_subprocess_home, apply_subprocess_home_env} |
| resolve_per_model_reasoning_effort | ✅ | reasoning::resolve_per_model_reasoning_effort (trait-based over `ReasoningOverrideMap`) |
| resolve_reasoning_config | 🟡 deferred to config crate (P1/P3) — consumes config dict shape | — |
| apply_ipv4_preference | ❌ deferred (P3 networking) | — |
| partial_update_hint / update diagnostics | ❌ deferred (P3) | — |

### hermes_utils (upstream: utils.py, 666 LOC) — ✅ complete
| Function/surface | Status | Rust home |
|---|---|---|
| TRUTHY_STRINGS, is_truthy_value, env_var_enabled | ✅ | truthy |
| env_int / env_float / env_bool | ✅ | truthy |
| _preserve_file_mode/_owner, _restore_file_mode/_owner | ✅ | atomic (POSIX chmod/chown, best-effort) |
| atomic_replace (symlink-preserving, EXDEV/EBUSY fallback) | ✅ | atomic::atomic_replace |
| atomic_write_text / atomic_json_write | ✅ | atomic |
| warn_if_credential_file_broadly_readable | ✅ | atomic |
| IndentDumper / atomic_yaml_write | 🟡 | yaml::atomic_yaml_write — serde_yaml rendering; value+atomicity parity, PyYAML byte-format divergence documented |
| atomic_roundtrip_yaml_update | 🟡 | yaml::atomic_roundtrip_yaml_update — comment-preserving for scalar/dotted-key updates; full-rewrite fallback for complex values (documented) |
| safe_json_loads | ✅ | json |
| fast_safe_load | ✅ | yaml::fast_safe_load (serde_yaml; identical safe-tag semantics) |
| normalize_proxy_url / normalize_proxy_env_vars | ✅ | proxy |
| base_url_hostname / base_url_host_matches | ✅ | urls |
| model_forces_max_completion_tokens | ✅ | urls |

### hermes_logging (upstream: hermes_logging.py, 800 LOC) — core complete (P1)
| Function/surface | Status | Rust home |
|---|---|---|
| LOG_FORMAT / LOG_FORMAT_VERBOSE | ✅ | record::LOG_FORMAT(_VERBOSE) |
| set_session_context / clear_session_context (thread-local) | ✅ | record |
| LogRecord factory session_tag injection | ✅ | record::LogRecord::new + session_tag |
| Level parsing (getattr fallback→INFO) | ✅ | record::Level::parse |
| COMPONENT_PREFIXES / NOISY_LOGGERS | ✅ | setup |
| setup_logging (agent.log/errors.log/gateway.log/gui.log, idempotent, force) | ✅ | setup |
| _read_logging_config (logging.level/max_size_mb/backup_count) | ✅ | setup (managed overlay identity until P3) |
| _ManagedRotatingFileHandler (rotation, managed chmod, inode reopen) | ✅ | rotating (managed chmod deferred to config crate) |
| _ComponentFilter | ✅ | rotating::ComponentFilter |
| QueueListener async path (_NonFormattingQueueHandler) | ✅ | queue (mpsc + worker thread) |
| flush_log_queue / drain_log_queue / rotating_file_handlers / _reset_queued_handlers | ✅ | queue (drain join is unbounded — documented) |
| RedactingFormatter (agent/redact.py, 1,197 LOC) | ✅ | logging::redact::RedactingFormatter, installed at setup_logging |
| setup_verbose_logging | ✅ | setup (stderr LogTarget with verbose format) |

### hermes_state_common (upstream: hermes_state_common.py, 614 LOC) — ✅ complete
| Function/surface | Status | Rust home |
|---|---|---|
| SCHEMA_SQL / DEFERRED_INDEX_SQL / FTS_SQL / FTS_TRIGRAM_SQL / LEGACY_FTS(_TRIGRAM)_SQL | ✅ byte-identical | common::* (generator: tools/gen_state_common_constants.py) |
| SCHEMA_VERSION / FTS_STORAGE_VERSION / MAX_FTS5_QUERY_CHARS / FTS_CJK_STALE_KEY | ✅ | common |
| _FTS_TRIGGERS / _FTS_CJK_TRIGGERS / FTS_CJK_TABLE_SQL / FTS_CJK_TRIGGER_SQL | ✅ | common |
| escape_like | ✅ | common::escape_like |
| preview shaping (_PREVIEW_RAW_SELECT, _shape_preview) | ✅ | common::{_preview_raw_select, _shape_preview} |
| child classification SQL (_branch/_compression/_listable/_ephemeral_child_sql) | ✅ | common |
| last-active SQL builders | ✅ | common |
| skill-commands subset (SKILL_SCAFFOLD_SQL_LIKE, describe_skill_invocation, extract_user_instruction_from_skill_message) | ✅ | skill (inlined until agent crate lands, P2) |

### hermes_state_schema (upstream: hermes_state_schema.py, 1,126 LOC) — ✅ complete
| Function/surface | Status | Rust home |
|---|---|---|
| schema_read_probe_statements (cached) | ✅ | schema::schema_read_probe_statements |
| _init_schema: SCHEMA_SQL, reconcile columns, PK heals (gateway_routing, session_model_usage), v16/v18/v20/v22/v23/v25 migrations, FTS storage stamp, title unique index | ✅ | schema::init_schema_inner |
| FTS DDL branch (legacy-vs-v23), trigger repair/rebuild, UPDATE OF migration, CJK quarantine | ✅ | schema + state impl |
| _parse_schema_columns (in-memory SQLite parse) | ✅ | schema::parse_schema_columns |

### hermes_state (upstream: hermes_state.py, 9,996 LOC) — ✅ complete (all 176 SessionDB methods ported)
| Function/surface | Status | Rust home |
|---|---|---|
| SessionDB open (writable/RO), zeroed-DB quarantine, test-isolation guard, lock-patience open | ✅ | state::{open, open_read_only, open_writable} |
| apply_wal_with_fallback + WAL-reset gate + pragmas | ✅ | wal (see row above) |
| _execute_write (BEGIN IMMEDIATE + jitter retry, checkpoint cadence) | ✅ | state::execute_write |
| close (TRUNCATE checkpoint writable-only) | ✅ | state::close |
| get_meta / set_meta / _store_system_prompt / system_prompt_hash | ✅ | state |
| create_session (+_insert_session_row ON CONFLICT enrichment, parent backfill, compression-fork origin inheritance) | ✅ | crud::create_session |
| get_session / _session_row_dict (system-prompt resolution) | ✅ | crud::get_session + SESSION_SELECT |
| resolve_session_id (exact/prefix, LIKE-escaped) | ✅ | crud::resolve_session_id |
| end_session / reopen_session / promote_to_session_reset | ✅ | crud::{end_session, reopen_session, promote_to_session_reset} |
| update_session_cwd (git_branch/git_repo_root, replace_git_meta) | ✅ | crud::update_session_cwd |
| MAX_TITLE_LENGTH / sanitize_title (control-char strip, collapse, length ValueError) | ✅ | crud::sanitize_title |
| set_session_title / set_auto_title_if_empty (+ compression-ancestor title transfer, ValueError conflict) | ✅ | crud::set_session_title / set_auto_title_if_empty |
| get_session_title / get_session_by_title / resolve_session_by_title | ✅ | crud::{get_session_title, get_session_by_title, resolve_session_by_title} |
| get_next_title_in_lineage (#N suffix lineage) | ✅ | crud::get_next_title_in_lineage |
| append_message (full kwargs, JSON framing, timestamp, counters, transcript guards, long patience) | ✅ | crud::append_message |
| append_messages_batch (atomic, guards, chunk_rows, aggregated counters) | ✅ | crud::append_messages_batch |
| _insert_message_rows (role-gated reasoning, platform/message_id, monotonic ts) | ✅ | crud::insert_message_rows |
| get_messages (active filter, limit/offset, content/tool_calls/display_metadata decode) | ✅ | crud::get_messages |
| latest_message_row_id / latest_user_message_row_id / get_message_role | ✅ | crud::{latest_message_row_id, latest_user_message_row_id, get_message_role} |
| _check_transcript_write_guards + compression-busy short-wait in _execute_write | ✅ | crud::check_transcript_write_guards + state::execute_write |
| portability mixin (export/import, rich rows) | ✅ | portability |
| search mixin (search_messages, FTS rebuild engine, anchored views) | ✅ | search (search_sessions_by_id landed with surface read helpers) |
| compression locks (try_acquire/release/refresh, holder-dead reclaim, publish_compression_child, find_live_compression_child, reopen_orphaned_compression_session) | ✅ | locks ((+ _non_continuation_child_filter_sql in common)) |
| async token accounting (queue/flush/stop writer, update_token_counts, ensure_session, record_auxiliary_usage, per-model usage upsert) | ✅ | token (dedicated writer connection — documented divergence) |
| telegram topics (migration v1→v2, enable/disable/is_enabled, bind/get/list/delete/is_linked, list_unlinked with preview) | ✅ | topics |
| handoffs (request/get/list_pending/claim/complete/fail, 500-char error truncation) | ✅ | handoff |
| prune/archive (set_session_archived lineage CTE, _prune_filter_where full surface, list_prune_candidates, archive_sessions, archive_stale_sessions, prune_sessions, prune_empty_ghost_sessions, on-disk file cleanup) | ✅ | prune |
| replace_messages (active_only) / has_archived_messages / archive_and_compact (+ _merge_model_config_json) / rewind_to_message | ✅ | rewrite |
| portability mixin (rich rows, distinct cwds, cron runs, skill-scaffolded, export/import) | ✅ | portability (incl. get_compression_lineage + search_sessions deps) |
| search mixin (search_messages, FTS rebuild engine, anchored views) | ✅ | search (search_sessions_by_id deferred — needs list_sessions_rich) |


| surface read helpers: list_sessions_rich (preview/last-active, session_key, id/search needles, compact_rows, pinned back-fill, read state), list_gateway_sessions (newest per session_key) | ✅ | rich |
| set_session_read / session_unread (lineage watermark, NULL=read, returns bool) | ✅ | rich::{set_session_read, session_unread} |
| set_session_pinned (lineage pin, durable keep flag) | ✅ | rich::set_session_pinned |
| get_compression_tip (chain walk, branch/delegate/tool exclusion, 100-bound) | ✅ | rich::get_compression_tip |
| touch_session_activity / clear_session_activity_labels / get_session_activity (+ agent/session_activity.py helpers) | ✅ | hermes-state::activity + rich |
| session_count / session_count_ge / session_count_by_source / count_empty_sessions | ✅ | rich |
| search_sessions_by_id (exact/prefix/substring ranking via id_query) | ✅ | rich::search_sessions_by_id |
| gateway routing CRUD: record_gateway_session_peer (peer metadata, COALESCE display_name/origin_json, compression-lineage option), set_expiry_finalized, save/replace/load/delete_gateway_routing_entries (scoped index), find_session_by_origin (exact-user wins, distinct-user contamination guard, thread filter), find_latest_gateway_session_for_peer (recoverable ends, peer-tuple fallback) | ✅ | routing |
| compression cooldown + anti-thrash counters: record/get/get_row/restore/clear_compression_failure_cooldown (active-vs-raw row APIs, rollback verification, fail-open record/clear), get/set_compression_fallback_streak, get/set_compression_ineffective_count (≥0 clamped) | ✅ | cooldown |
| session meta/model surfaces: update_session_meta (COALESCE model), update_system_prompt (hash-table canonical prompt), update_session_model (json_remove browser_model_lock, preserves lineage, nulls prompt), patch_session_model_config (None deletes keys, no-op missing), get_session_model_config_value (tolerant read), update_session_runtime_lock (browser_model_lock merge + prompt null), set_session_yolo / session_yolo_enabled (lineage-preserving yolo_mode, False on parse failure), update_session_billing_route (unconditional billing fields + prompt null) | ✅ | meta |
| session meta/model surfaces: update_session_meta (COALESCE model), update_system_prompt (hash-table canonical prompt), update_session_model (json_remove browser_model_lock, preserves lineage, nulls prompt), patch_session_model_config (None deletes keys, no-op missing), get_session_model_config_value (tolerant read), update_session_runtime_lock (browser_model_lock merge + prompt null), set_session_yolo / session_yolo_enabled (lineage-preserving yolo_mode, False on parse failure), update_session_billing_route (unconditional billing fields + prompt null) | ✅ | meta |
| message reactions + display-kind + api_content surfaces: set_latest_matching_message_display_kind (turn stamp), set_message_reaction (tapback toggle/replace, per-author), get_message_reactions, take_unseen_reactions (seen stamp, exactly-once announce, author filter), set_latest_user_api_content (defensive content guard) | ✅ | reactions |
| conversation surface: resolve_resume_session_id (compression-tip first, empty-head walk, fork picks newest child), get_messages_as_conversation (OpenAI format, include_ancestors/inactive, row ids, api_content verbatim, sanitize_context, harness/stale-marker strip, repair_alternation), get_resume_conversations (model/display one-SELECT split, verification-candidate collapse in model history), get_ancestor_display_prefix (non-tip rows only), get_conversation_root (stable conversation id), session_lineage_root_to_tip, duplicate-replayed-user dedup, restore_rewound; helpers sanitize_context / repair_message_sequence / strip harness + stale markers | ✅ | conversation |
| delete surface: clear_messages, get_session_delete_targets (delegate-child collection), delete_session (delegate cascade + branch orphan + expected-set TOCTOU guard + on-disk cleanup), delete_session_if_empty (title/messages/children guard), delete_sessions (bulk, dedup, per-row contract), delete_empty_sessions (empty+ended+non-archived), finalize_orphaned_compression_sessions (#20001, 7-day cutoff) | ✅ | delete (+locks finalize) |
| maintenance: message_count, has_platform_message_id, purge_stale_tool_call_markers (dry-run/backup VACUUM INTO), retag_kanban_worker_sessions (per-root gate), logical_size_bytes, vacuum (FTS-merge then checkpoint then VACUUM), maybe_auto_prune_and_vacuum (interval gate + last_vacuum throttle), maybe_auto_archive (interval gate) | ✅ | delete |

**hermes_state.py surface: ✅ COMPLETE** — every SessionDB method ported across
crud/schema/search/portability/locks/token/topics/handoff/prune/rewrite/
rich/routing/cooldown/meta/reactions/conversation/delete modules.
### agent/redact (upstream: agent/redact.py, 1,197 LOC) — ✅ complete (homed in hermes-logging)
| Function/surface | Status | Rust home |
|---|---|---|
| redact_sensitive_text full pass chain (prefix/ENV/JSON/YAML/auth/headers/private keys/DB connstrs/JWT/phones/form) | ✅ | logging::redact::redact_sensitive_text |
| mask_secret / _mask_token / non-reusable sentinel | ✅ | logging::redact |
| redact_cdp_url / redact_terminal_output / is_env_dump_command / _command_reads_env_file | ✅ | logging::redact |
| RedactingFormatter on the logging seam | ✅ installed at setup_logging (first-install-wins) | logging::redact::RedactingFormatter |
| lookaround patterns | ✅ via fancy-regex (documented divergence) | — |

### hermes_time (upstream: hermes_time.py, 135 LOC)
| Function/surface | Status | Rust home |
|---|---|---|
| resolution order (env → config.yaml → local) | ✅ | time::resolve_timezone_name |
| get_timezone (cached) / reset_cache | ✅ | time::{get_timezone, reset_cache} |
| now() | ✅ | time::now |
| invalid-tz fallback (warn + local) | ✅ | time::{get_timezone, now} |
| read_raw_config compatibility (fail-open) | ✅ via config reader | time::config::raw_timezone |

## 6. Parity oracles

Every ported behavior is pinned by tests whose cases derive from upstream
(`upstream/` holds vendored fixtures; `tests/` in each crate hold the
oracle tests). Upstream oracle files currently mirrored:

- `tests/test_timezone.py` → `crates/hermes-time/tests/parity_timezone.rs`
- `tests/test_hermes_constants.py` (foundational subset) → `crates/hermes-constants/tests/parity_constants.rs`
- `tests/cron/test_reasoning_config_per_model.py`, `tests/gateway/test_reasoning_config_per_model.py`
  → deferred until reasoning-config crate lands
- `hermes_state_common.py` → `crates/hermes-state/src/common.rs` (golden `upstream/golden_state_common.json`)
- `tests/test_hermes_state.py` TestConnectionLifecycle/TestSchemaInit subset → `crates/hermes-state/tests/parity_state_lifecycle.rs`
- `tests/test_hermes_state.py` SessionLifecycle/MessageStorage/TimestampPreservation/Title families + `tests/hermes_state/test_append_messages_batch.py` → `crates/hermes-state/tests/parity_state_crud.rs`
- `agent/redact.py` → `crates/hermes-logging/src/redact.rs` (golden `upstream/golden_redact.json`)
- `tests/hermes_state/test_session_read_state.py` + TestCounts / TestExcludeSources / TestCompressionChainProjection / TestSessionPinAndStaleArchive / TestSessionIdSearch / gateway-listing subset → `crates/hermes-state/tests/parity_state_rich.rs`
- `agent/session_activity.py` → `crates/hermes-state/src/activity.rs` (bound/normalize/build helpers; Python `round(x,1)` tie-to-even rounding)

Evidence format: every claim in this file must cite `unit` | `mock` | `live`
+ the exact command, e.g. `cargo test -p hermes-time (unit)`.

## 7. Session log

- 2026-08-22 (session 3b): Stdlib document extraction landed — hermes-tools
  src/read_extract.rs (@ b9aa928, 346 LOC). .ipynb (cells in order,
  markdown/code labels + numbering, output payloads never leak, legacy
  worksheets fallback), .docx (paragraphs/runs with w:t/tab/br/cr
  handling), .xlsx (visible-sheet iteration, shared strings, cell types
  s/inlineStr/b/e/plain, column-letter index, 5000-row/256-col caps,
  empty-tail row pruning, hidden-sheet omission). `zip` + `roxmltree`
  deps; anydoc converter is a deferred seam (absent -> unsupported,
  matching the no-package import path). Oracle: tests/tools/test_read_
  extract.py fixture builders + cases (extension gate, notebook order/
  empty, docx paragraphs/missing xml, xlsx visible content/hidden
  omission/not-a-zip); 9 parity tests in parity_read_extract.rs (+4
  unit tests); workspace 548 tests green; clippy clean. Fixed the file_
  state kill-switch test race by splitting it into an isolated binary
  (the env var is process-global). Evidence: `cargo test --workspace`
  (unit).
- 2026-08-22 (session 3a): File-tool support cluster landed — hermes-tools
  src/file_state.rs + path_security.rs + binary_extensions.rs (@ b9aa928).
  file_state: process-wide FileStateRegistry (read stamps mtime/read_ts/
  partial, global last-writer map, bounded caps 4096, sibling-staleness
  warnings by severity class, partial-read and external-drift detectors,
  writes_since for delegate reminders, known_reads, HERMES_DISABLE_FILE_
  STATE_GUARD kill switch). path_security: validate_within_dir with a
  Python-resolve-equivalent (canonicalize deepest ancestor + append missing
  components so non-existent targets normalize safely). binary_extensions:
  the full ~60-entry binary-extension set + is_binary_extension.
  DIVERGENCE (documented): HashMap lacks Python's insertion-order eviction,
  so cap trimming drops arbitrary keys; per-path lock_path is a caller-side
  critical-section concern (registry maps are under one Mutex) until the
  executor task-concurrency layer lands. Oracle: test_file_state_registry
  .py unit subset (staleness classes, sibling flags, kill switch,
  writes_since, known_reads); 11 parity tests + 2 path_security unit tests
  + ansi unit; workspace 534 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).
- 2026-08-22 (session 2c): Tool result persistence + budgets landed —
  hermes-tools src/budget_config.rs + src/tool_result_storage.rs
  (@ b9aa928). budget_config: BudgetConfig (frozen defaults 100K/200K/
  1.5K), resolve_threshold priority (pinned read_file=inf > overrides >
  registry capped > default), budget_for_context_window (proportional
  scaling with floor/clamp). tool_result_storage: safe filename hashing,
  generate_preview (last-newline truncation), SandboxExecutor seam (the
  Python env.execute abstraction), write-to-sandbox via stdin (no argv
  ceiling), <persisted-output> replacement block (KB/MB sizing + thousands
  grouping), maybe_persist_tool_result (threshold/override/inf paths,
  inline-truncation fallback), enforce_turn_budget (aggregate spill of
  largest non-persisted results until under budget, persisted-tag skip).
  Oracle: test_tool_result_storage.py + test_budget_config.py core
  behaviors (unchanged boundary, path-escape neutralization, stdin
  verbatim, env-temp-dir, turn-budget spill); 14 parity tests in
  parity_tool_result_storage.rs; workspace 521 tests green; clippy clean.
  Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 2b): Second/third tool-file ports —
  tools/ansi_strip.py and tools/session_search_tool.py → hermes-tools
  src/ansi_strip.rs + src/session_search.rs (@ b9aa928). ansi_strip: full
  ECMA-48 strip + sanitize_display_text (C0 control removal, CR→LF);
  clarifies a real pattern bug (raw-string line-continuation backslashes).
  session_search: single-shape tool with DISCOVERY (FTS5 + lineage dedup +
  hidden/demoted source handling #19434 + current-lineage guards +
  compaction/compression-history carve-outs + anchored window/bookends +
  compaction-summary filter + rebuild-status annotation + title-match
  path), SCROLL (±window clamp [1,20], current-lineage rejection unless
  compacted/compression-ended, lineage rebind with warning), READ
  (head+tail truncation, ANSI strip, @session link), BROWSE (recent
  sessions excluding current lineage + hidden sources). Deferred: cross-
  profile reads (_resolve_profile_db / _locate_session_db) until the
  hermes_cli profiles crate (P3). Oracle: tests/tools/test_session_search
  .py core shapes + schema invariants; 10 parity tests in
  parity_session_search.rs (+3 ansi unit tests); workspace 507 tests
  green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 2a): First tool-file port — tools/clarify_tool.py →
  hermes-tools/src/clarify.rs (@ b9aa928, 266 LOC). clarify_tool
  (question validation, choices flatten + trim to MAX_CHOICES, empty→open-
  ended, multi-select parse: list / JSON-array / comma-separated),
  _flatten_choice (label→description→text→title unwrap; name/value
  excluded), platform callback via a thread-local injection slot (mirrors
  runner-provided callback= kwargs seam), check_clarify_requirements,
  CLARIFY_SCHEMA const, registry.register with ❓ emoji. Handler reads the
  thread-local callback at dispatch time; no callback → the upstream error
  JSON. Oracle: tests/tools/test_clarify_tool.py (13 tests); workspace 494
  tests green; clippy clean. Evidence: `cargo test --workspace` (unit).
  Registry now hosts the first real tool file; tool-file ports continue.
- 2026-08-22 (session 1z): tools/schema_sanitizer.py ported —
  hermes-tools src/schema_sanitizer.rs (@ b9aa928, 687 LOC). Deep-copy
  schema sanitization for strict backends: property-key renames
  (`[a-zA-Z0-9_.-]{1,64}` + collision suffixes + required remap + dispatch
  reverse-map unrename_tool_args), bare-string schema replacement, object
  empty-properties injection, `type: [X, "null"]` collapse + multi-type →
  anyOf, nullable-union collapse (keep_nullable_hint), const-union → enum
  (MCP path), top-level combinator strip (Codex), `$ref` sibling strip
  (Fireworks), required pruning, reactive strip_pattern_and_format (llama
  .cpp recovery) + strip_slash_enum (xAI Responses). Wired into
  model_tools::compute_tool_definitions before tool-search assembly (the
  previous seam). Unit tests in-module + existing coverage through the
  pipeline; workspace 481 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).
- 2026-08-22 (session 1y): model_tools surface landed — hermes-toolsets
  src/model_tools.rs. get_tool_definitions + _compute_tool_definitions
  (enabled/disabled/legacy toolset resolution, kanban worker auto-append,
  platform-bundle + posture delta subtraction, browser_navigate
  cross-reference strip, quiet-mode memo cache bounded at 8 with
  generation/config-key bytes, _last_resolved_tool_names),
  coerce_tool_args + coercion helpers (string→int/number/bool, union
  types, bare-value array wrap, JSON-encoded list/object parse, nested
  element/field normalization, nullable null preservation),
  sanitize_tool_error (role-tag/fence/CDATA strip + 2000-char cap),
  toolset query shims. REGEN: gen_toolsets.py now records the `posture`
  flag (coding=True) and data.rs carries it. FIXES: get_toolset now
  synthesizes registry-only (plugin/MCP) toolsets like upstream.
  DEFERRED (documented): handle_function_call + hooks + rewind +
  coordinator middleware (agent loop), execute_code/discord dynamic
  schema rebuilds, schema_sanitizer, tool_search assembly,
  _resolve_active_context_length. Oracle: run_agent/test_tool_arg_
  coercion.py + test_sanitize_tool_error.py + get_tool_definitions
  behaviors; 20 parity tests in parity_model_tools.rs; workspace 470
  tests green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1x): Registry seam wired — hermes-toolsets now
  depends on hermes-tools and reads the live registry singleton.
  get_toolset(include_registry=true) merges registry tools into builtin
  toolsets (sorted union); validate_toolset accepts registry aliases;
  resolve_toolset's hermes-* plugin-platform auto-gen now sees real
  registered platform tools. 2 integration parity tests added (registry
  merge + alias validation); workspace 450 tests green; clippy clean.
  Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1w): hermes-tools crate opened — tools/registry.py
  (@ b9aa928, 956 LOC). ToolRegistry singleton: register (cross-toolset
  shadow rejection, override opt-in + plugin ownership gate),
  deregister (unowned-plugin block, toolset-check/alias cleanup),
  get_definitions (check_fn-filtered OpenAI schemas, per-call shared
  check dedup, dynamic_schema_overrides, name fallback),
  dispatch (ToolHandler trait, panic -> error json, raw string result
  parity), check_fn TTL cache with flake suppression
  (30s TTL / 60s last-good grace / 512 cap, identity by Arc pointer so
  shared probes dedup like Python callable keys), toolset query surfaces
  (names/tool-names/aliases/emoji/limits/requirements/availability),
  tool_error / tool_result helpers, module singleton. DIVERGENCE
  (documented): async handlers are adapted to the sync ToolHandler trait
  by their tool crates; cache_scope (multiplex profile isolation) and
  discover_builtin_tools (AST scan) are seams until agent/tools-catalog
  land. Oracle: tests/tools/test_registry.py core behaviors (register/
  dispatch, definitions, shared check, availability, plugin gates,
  helpers); 12 parity tests in parity_registry.rs; workspace 448 tests
  green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1v): hermes-toolsets crate opened — toolsets.py +
  toolset_distributions.py (P2 start). Static data generated 1:1 from
  upstream via tools/gen_toolsets.py (AST-parses upstream, emits data.rs):
  56 core tools, 4 webhook-safe, 59 toolsets, 17 distributions. Logic port:
  get_toolset (static/custom view; registry merge deferred), bundle_non_core
  _tools (bundle delta minus core #33924), resolve_toolset (cycle-safe,
  visited-set sharing for diamonds, "all"/"*" special aliases, hermes-*
  plugin-platform auto-gen seam), resolve_multiple_toolsets,
  create_custom_toolset (runtime overlay), get_toolset_info,
  get_toolset_names / get_all_toolsets / validate_toolset;
  distributions::get/list/sample/validate (probability sampling, highest-
  probability fallback, ValueError for unknown). DIVERGENCE (documented):
  registry-dependent lookups (tools.registry overlays, plugin platforms, MCP
  aliases) return empty until the tools registry crate lands — the
  include_registry parameter is accepted for call-site parity and behaves
  statically; `registry_lookup()` names the seam. Oracle: test_toolsets.py +
  test_toolset_distributions.py (17 tests); live Python-vs-Rust output
  byte-identical for web/debugging/hermes-telegram/all/bundle-delta/names;
  workspace 435 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).
- 2026-08-22 (session 1u): Delete + maintenance surfaces landed —
  delete.rs (plus finalize_orphaned_compression_sessions in locks.rs).
  message_count, has_platform_message_id, clear_messages, module helpers
  collect_delegate_child_ids / delete_delegate_children (chain-walking
  _delegate_from markers, parent-cycle guard #49148), get_session_delete_
  targets, delete_session (delegate cascade, branch/compression orphan via
  parent NULL, expected_delete_ids TOCTOU fail-closed, on-disk file
  cleanup), delete_session_if_empty (title/messages/children guard, one
  txn), delete_sessions (bulk, dedup, silent-skip unknown), delete_empty_
  sessions, finalize_orphaned_compression_sessions (#20001, 7-day cutoff,
  api_call_count=0 + ended-parent + has-messages guard),
  purge_stale_tool_call_markers (dry-run, VACUUM INTO backup, marker-re
  content clear), retag_kanban_worker_sessions (per-workspace state_meta
  gate), logical_size_bytes (page_count*page_size), vacuum (FTS optimize +
  TRUNCATE checkpoint + VACUUM), maybe_auto_prune_and_vacuum (interval
  gate + last_vacuum throttle, never raises), maybe_auto_archive (interval
  gate, archive_stale_sessions drive). This closes the hermes_state.py
  surface: ALL 176 SessionDB methods ported; inventory marks hermes_state
  done. Oracle: TestCounts.message_count, TestDeleteAndExport.delete_session
  + expected-targets fail-closed, delete-empty reap, purge/retag contracts,
  auto-maintenance idempotency; 15 parity tests in parity_state_delete.rs;
  workspace 418 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).
- 2026-08-22 (session 1t): Conversation projection surface landed —
  conversation.rs. resolve_resume_session_id (get_compression_tip first so a
  long-lived parent with rows still redirects to the continuation; then the
  empty-head walk over non-branch children, newest-first, depth-capped 32),
  get_messages_as_conversation (ORDER BY id, optional ancestors/inactive,
  sanitize_context scrub, harness-turn + stale-tool-marker strips, optional
  repair_alternation), get_resume_conversations (one lineage SELECT feeds
  both the alternation-repaired model history and verbatim lineage display,
  both with _row_id), get_ancestor_display_prefix (non-tip rows only,
  #65919), get_conversation_root, session_lineage_root_to_tip, duplicate-
  replayed-user dedup, restore_rewound; ported helpers: sanitize_context
  (3 memory-context regexes), repair_message_sequence (pass 0 assistant
  merge + verification-candidate replace + codex interim exemption; pass 1
  stray-tool drop with id/call_id superset; pass 2 user merge with
  api_content invalidation), background-review harness + stale-marker
  strips. Oracle: test_resolve_resume_session_id, test_conversation_root,
  memory-context strip + reasoning/tool_calls restore, resume
  verification-candidate split, repair contracts; 20 parity tests in
  parity_state_conversation.rs; workspace 403 tests green; clippy clean.
  Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1s): Message presentation + reactions landed —
  reactions.rs. set_latest_matching_message_display_kind (stamps the freshly
  persisted turn row by session+role+content), set_message_reaction (one
  reaction per author per message; same-emoji retract; different-emoji
  replace; emoji=None clear; reactions live under display_metadata.reactions
  so rewind/compaction row rewrites keep them), get_message_reactions
  (never None), take_unseen_reactions (marks seen exactly-once, author
  filtered, decoded text), set_latest_user_api_content (backfills the
  api_content sidecar on the newest ACTIVE user row with a defensive
  content-match guard). Oracle: tests/test_message_reactions.py (tapback +
  seen-exactly-once + author independence + cache safety) + redisplay-kind
  turn-stamp + api_content lone-surrogate guard; 12 parity tests in
  parity_state_reactions.rs; workspace 383 tests green; clippy clean.
  Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1r): Session meta/model surfaces landed — meta.rs.
  update_session_meta (model COALESCE), update_system_prompt (canonical
  hash-table prompt + unreferenced-prompt GC), update_session_model
  (unconditional model, json_remove('$.browser_model_lock'), lineage
  markers preserved, prompt+hash nulled), patch_session_model_config
  (shared merge helper, None deletes key, missing-row no-op),
  get_session_model_config_value (tolerant JSON parse + default),
  update_session_runtime_lock (browser_model_lock merge with updated_at
  stamp + prompt null), set_session_yolo / session_yolo_enabled
  (lineage-preserving yolo_mode; False on any parse failure),
  update_session_billing_route (unconditional billing fields + prompt
  null). Oracle: update_session_model browser-lock clearing +
  first_accounted_route atomicity, TestSessionDbYoloFlag, system-prompt
  dedup; 9 parity tests in parity_state_meta.rs; workspace 371 tests
  green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1q): Compression cooldown + anti-thrash counters
  landed — cooldown.rs. record_compression_failure_cooldown (fail-open warn),
  get_compression_failure_cooldown (active-only, remaining_seconds),
  get_compression_failure_cooldown_row (raw exact columns, session_exists),
  restore_compression_failure_cooldown_row (transactional rollback + post-
  verify, RuntimeError on missing/divergent rows), clear (fail-open warn),
  get/set_compression_fallback_streak + get/set_compression_ineffective_count
  (>=0 clamp, empty-id no-ops). Oracle: force-cancel restore-exact-row and
  anti-thrash persistence contracts (agent-level orchestration deferred to
  P2); 8 parity tests in parity_state_cooldown.rs; workspace 362 tests
  green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1p): Gateway routing surface landed — routing.rs.
  record_gateway_session_peer (COALESCE display_name/origin_json, optional
  compression-lineage CTE stamping the whole ancestor chain),
  set_expiry_finalized, save_gateway_routing_entry (single-row UPSERT on
  (scope, session_key), updated_at bump), replace_gateway_routing_entries
  (atomic per-scope full rewrite), load_gateway_routing_entries,
  delete_gateway_routing_entries, find_session_by_origin (live-only,
  exact-user preferred, multi-user contamination guard, thread filter),
  find_latest_gateway_session_for_peer (recoverable-ended rows only:
  agent_close/ws_orphan_reap, message-count guard, peer-tuple fallback when
  exact session_key missing). Oracle: test_gateway_session_peer_round_trip_
  and_recovery + test_find_session_by_origin_matching_rules +
  gateway SessionStore routing-index roundtrips; 6 parity tests in
  parity_state_routing.rs; workspace 354 tests green; clippy clean.
  Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1o): Surface read helpers landed — rich.rs + activity.rs.
  list_sessions_rich (preview/last-active, source/sources/exclude_sources,
  session_key, cwd_prefix, min_message_count, include/archived_only,
  order_by_last_active CTE with id/search needles over the forward
  compression chain, compact_rows projection, include_pinned back-fill
  before projection, compression-root→tip projection with 13 merge keys +
  _lineage_root_id, derived unread), list_gateway_sessions (newest row per
  session_key, platform + active_only, activity-heartbeat last_active),
  set_session_read/session_unread (lineage watermark, NULL=read),
  set_session_pinned (lineage pin), get_compression_tip (chain walk,
  branch/delegate/tool exclusion, 100-bound), session_count /
  session_count_ge / session_count_by_source / count_empty_sessions,
  search_sessions_by_id (id_query-bounded candidates, exact/prefix/
  substring ranking), touch_session_activity / clear_session_activity_
  labels / get_session_activity (activity.rs mirrors agent/session_activity
  .py: ActivityProvenance, bound/normalize/build + Python round(x,1)
  tie-to-even). Fixes the mid-edit rich.rs that didn't compile (cloneable P
  param enum replaces Box<dyn ToSql> doubling). DIVERGENCE (documented in
  rich.rs): get_session_activity reads the three activity columns directly
  instead of via get_session dict; set_session_read uses state::now()
  (time.time equivalent). Oracle: test_session_read_state.py (6), TestCounts
  (2 + grouping), gateway heartbeat + newest-per-key, session_key filter +
  search-scoped projection, compression-chain walk + tip projection +
  two-chain batching, exclude-tool, pin roundtrip + limit-window back-fill,
  id search exact/prefix; 24 parity tests in parity_state_rich.rs; workspace
  348 tests green; clippy clean. Evidence: `cargo test --workspace` (unit).
  REMAINING hermes_state (beyond foundation, P2/P3 consumers): gateway
  routing CRUD, compression cooldown/streak counters, session
  meta/model surfaces, finalize_orphaned_compression_sessions, message
  reactions, conversation surface, delete surface, maintenance helpers.
- 2026-08-22 (session 1n): Space reclamation landed — prune.rs.
  set_session_archived (recursive lineage CTE archives the whole compression
  chain as a unit, tips+roots), _prune_filter_where full filter surface
  (bounds, LIKE+escape_like literal `_`/`%`, provider case-insensitive,
  cwd_prefix shared with portability, tri-state archived), list_prune_
  candidates (dry-run rows oldest-first), archive_sessions (archived=False
  default → idempotent, per-row lineage archive), archive_stale_sessions
  (real-recentcy cutoff, pinned guard, lineage tips only), prune_sessions
  (ended-only, orphan children, messages cascade, system-prompt GC, on-disk
  file cleanup outside the txn), prune_empty_ghost_sessions (>24h empty tui
  ghosts). SessionRow gained `archived` so get_session reports the flag;
  _delete_unreferenced_system_prompts factored into crud and reused.
  Oracle: TestPruneSessions + TestPruneSessionFilters +
  tests/hermes_state/test_session_archiving.py (lineage flip subset;
  list_sessions_rich projections deferred to the surface helpers unit);
  14 parity tests in parity_state_prune.rs; workspace 321 tests green;
  clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1m): Cross-platform handoff state machine landed —
  handoff.rs. request_handoff (pending only from None/completed/failed),
  get_handoff_state (fail-open None), list_pending_handoffs (session dicts
  with resolved system prompt, oldest first, fail-open []),
  claim_handoff (pending→running CAS), complete_handoff (clears error),
  fail_handoff (error[:500] truncation). Oracle:
  tests/hermes_cli/test_session_handoff.py TestHandoffStateDB subset + the
  dedup pending-system-prompt shape; 6 parity tests in parity_state_handoff
  .rs; workspace 307 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).
- 2026-08-22 (session 1l): Telegram DM topics landed — topics.rs.
  apply_telegram_topic_migration (explicit opt-in only: mode + bindings
  tables, unique/idx, v1→v2 FK rebuild gate pinned by state_meta version),
  enable/disable/is_telegram_topic_mode_enabled (capability flag ints),
  bind_telegram_topic (idempotent same-topic upsert; ValueError on
  re-link), get/list/get_by_session topic bindings, delete_telegram_topic_
  binding (rowcount; last-binding prune flips mode.enabled=0 in the same
  txn; missing tables are silent no-ops — #31501 contract),
  is_telegram_session_linked_to_topic, list_unlinked_telegram_sessions_
  for_user (preview shaping via common::_preview_raw_select/_shape_preview,
  last-active, absent-tables fallback without the NOT EXISTS clause; rows
  shaped via fold_session_dict). Oracle: test_hermes_state.py topic
  roundtrip + tests/gateway/test_telegram_prune_stale_topic_binding_31501
  (SessionDB subset) + v1→v2 migration gate; 10 parity tests in
  parity_state_topics.rs; workspace 301 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).
- 2026-08-22 (session 1k): Async token accounting landed — token.rs.
  Background writer queue (dedicated writer thread + condvar): queue_token_
  counts (append+notify, dead-thread respawn, sync fallback after close),
  flush_token_counts (fast path, live-writer authoritative, dead-writer
  caller-drain with busy-before-clear protocol), _token_writer_loop,
  _apply_token_batch (defensive coalesce-fallback), _coalesce_token_deltas
  (adjacent same-route merge; sum fields summed, cost fields None-preserving,
  absolute never merges), _stop_token_writer (bounded join via exit channel,
  leftover drain claims busy), writer respawn semantics. update_token_counts
  (absolute/incremental SQL, _insert_session_row ensure in its own write
  txn, first_accounted_route pre-read, incremental-only per-model usage via
  _record_model_usage upsert), ensure_session, record_auxiliary_usage (task
  dimension, aux rows never inherit route, api_call_count=1, empty-id/task
  short-circuit). get_session now drains the queue first (upstream flush
  seam). DIVERGENCE (documented in token.rs): SessionDB is !Sync, so the
  writer uses its own dedicated connection opened at spawn (observable
  semantics preserved: strict enqueue order, per-delta BEGIN IMMEDIATE with
  the same busy/jitter retry budget; the sessions-DB checkpoint cadence is
  skipped on the writer conn — performance only). close() joins the writer
  (bounded) and drains leftovers; Drop safety net leaks the shared queue
  state for a still-running writer (daemon-equivalent to atexit). Oracle:
  tests/agent/test_async_token_accounting.py (ordering/absolute-barrier/
  backlog-sum/coalesced-equals-sequential/read-your-writes/enqueue-after-
  close/field-contract) + tests/hermes_state/test_aux_usage_accounting.py
  (record/accumulate/coexist subset) — monkeypatch-gated cases noted
  non-portable. 17 parity tests in parity_state_token.rs; workspace 291
  tests green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1d): hermes-logging crate landed (core complete):
  rotating file handlers (agent/errors/gateway/gui), component routing,
  thread-local session tags, async queue listener, external-rotation inode
  reopening, config-driven defaults, verbose mode, pluggable redactor seam.
  140 workspace tests green; clippy clean. Evidence: `cargo test --workspace`
  (unit). Next foundation unit: `hermes-state` subsystem (state_schema /
  state_common / state / portability / search, ~14K LOC) — largest P1 chunk.
- 2026-08-22 (session 1c): P1 continuation. hermes-constants surfaces
  completed: node dir ordering, candidate node command names, display home,
  secure parent dir, profile-home helpers, per-model reasoning override
  (trait-based, config dict impl deferred to config crate). hermes-utils
  crate landed (full utils.py port: truthy/env coercions, atomic writes,
  YAML write + comment-preserving roundtrip, JSON safe-parse, proxy
  normalization, URL hostname helpers, max-completion-token families).
  Parity oracles vendored (`upstream/golden_utils.json`); workspace tests
  124 green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 1): P0 scaffold. Inventory generated, governance set,
  workspace created, hermes-constants foundational subset + hermes-time full
  port landed with parity tests. Binary renamed/shaped; golden fixtures
  vendored from upstream `_canonical_model_variants` /
  `parse_reasoning_effort` (@ b9aa928); parity tests assert exact full-list
  equality. Evidence: `cargo test --workspace` (unit) — 74 ok.
- 2026-08-22 (session 1b): GitHub repo `1deat0r/hermes-agent-rust` created
  (public, https://github.com/1deat0r/hermes-agent-rust), P0 commits pushed.
  Standing rule: every commit is made locally AND pushed to origin in the
  same step (no local-only commits). gh is authenticated as 1deat0r.
- 2026-08-22 (session 1e): State subsystem + redactor — largest P1 chunk.
  (a) hermes-state crate opened (rusqlite bundled; SQLite 3.50.2, FTS5 +
  trigram + external-content verified). hermes_state_common ported 1:1:
  byte-identical SCHEMA_SQL/FTS DDL (generator), preview shaping,
  child-session + last-active SQL builders, skill-commands subset
  (inlined until P2; golden upstream/golden_state_common.json). (b) WAL
  machinery: apply_wal_with_fallback + WAL-reset gate + operator
  journal_mode + config pragmas; bundled 3.50.2 matches upstream's
  vulnerable window so fresh DBs prefer DELETE, exactly like upstream's
  3.50.4. (c) SessionDB open/close + full schema init: writable+RO open,
  zeroed-DB quarantine, test-isolation guard, lock-patience open retry,
  _execute_write (BEGIN IMMEDIATE + jitter + checkpoint cadence),
  schema migrations v16/v18/v20/v22/v23/v25 + PK heals + FTS branch +
  UPDATE OF trigger migration + CJK ensure/quarantine. Lifecycle oracles
  mirror TestConnectionLifecycle/TestSchemaInit. (d) agent/redact.py
  ported into hermes-logging (fancy-regex for lookarounds): full 55-prefix
  chain, ENV/JSON/YAML/auth/JWT/DB/phone/form passes, term-output policy,
  RedactingFormatter installed at setup_logging (first-install-wins);
  golden upstream/golden_redact.json byte-equality; corpus caught one
  real bug (_ENV_ASSIGN_RE must not be IGNORECASE). Workspace tests 189
  green; clippy clean. Evidence: `cargo test --workspace` (unit).
  REMAINING state subsystem (post-2026-08-22 1n): surface read helpers
  landed in session 1o (rich.rs + activity.rs). Beyond foundation, the
  operator-flagged P2/P3 deferreds re main: (apply_ipv4_preference,
  partial_update_hint, managed-node bootstrap, agent_browser_runnable,
  resolve_reasoning_config), plus the non-foundation hermes_state surfaces
  listed in the parity matrix (gateway routing, reactions, conversations,
  delete surface, maintenance, meta/model, cooldown counters).
- 2026-08-22 (session 1f): SessionDB CRUD surface landed — sessions,
  messages, titles. create_session (ON CONFLICT COALESCE enrichment,
  parent cwd/git backfill, compression-fork origin inheritance),
  get_session with resolved system_prompt, resolve_session_id,
  end/reopen/promote_to_session_reset, update_session_cwd;
  sanitize_title + set/auto/get title + lineage + title-transfer off
  compression ancestors + ValueError conflicts; append_message (full
  21-column insert, JSON framing sentinel, explicit timestamps,
  message_count/tool_call_count counters, transcript guards,
  TRANSCRIPT_WRITE_PATIENCE_S), append_messages_batch (atomic,
  chunk_rows, aggregated counters, role-gated reasoning via
  insert_message_rows), get_messages (limit/offset, decode),
  latest/latest_user/get_message_role. _execute_write upgraded to
  WriteError taxonomy incl. the compression-busy short-wait retry
  (SessionCompressionInProgressError transient, closed/permanent
  propagate). Oracle: tests/test_hermes_state.py (SessionLifecycle/
  MessageStorage/TimestampPreservation/Title families) +
  tests/hermes_state/test_append_messages_batch.py; 29 CRUD parity
  tests added; workspace 218 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).

- 2026-08-22 (session 1g): Compression-lock lifecycle landed — locks.rs.
  try_acquire_compression_lock (single-txn DELETE-expired + INSERT OR
  IGNORE + ownership SELECT; dead-pid reclaim via libc kill probe),
  refresh_compression_lock (holder-only ownership, revives expired own
  row), release_compression_lock (idempotent, holder-checked),
  get_compression_lock_holder, _compression_lock_holder_process_is_dead
  (POSIX ESRCH probe; Windows stays TTL-only = upstream's nt early
  return; no psutil fast path — documented divergence), and the recovery
  family: find_live_compression_child (ambiguous -> None),
  reopen_orphaned_compression_session (Result<bool> — upstream has no
  try/except so sqlite3 errors propagate), publish_compression_child
  (atomic parent-close + child row + handoff; lease-required by default;
  CompressionSessionBusyError + RuntimeError mapped to new
  WriteError::CompressionBusy/Runtime variants; model_config empty-dict
  -> NULL like create_session). Oracle: test_refresh_compression_lock_*,
  tests/state/test_compression_lineage_guard.py,
  test_session_system_prompt_dedup.py::test_compression_child_*; 15
  parity tests; workspace 233 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).

- 2026-08-22 (session 1h): Message rewrite/rewind surfaces landed —
  rewrite.rs. replace_messages (atomic delete+reinsert; active_only=true
  #80216-safe path preserves soft-archived rows; CompressionSessionClosed
  rejection), has_archived_messages (cheap probe), archive_and_compact
  (non-destructive in-place compaction: active=0/compacted=1 archive +
  fresh active rows in one txn; model_config_patch merged via shared
  _merge_model_config_json with on_missing=raise; counts track the live
  set), rewind_to_message (soft-delete id>=target incl. target; target
  row returned with decoded content for prompt prefill; rewind_count
  always bumped; ValueError on missing/non-user target). StoredMessage
  gained the `compacted` flag (get_messages parity). Oracle:
  test_replace_messages_preserves_timestamps / compression roundtrip /
  display_metadata, tests/hermes_state/test_replace_messages_archive_
  siblings.py, gateway rewind DB contract; 13 parity tests; workspace
  246 tests green; clippy clean. Evidence: `cargo test --workspace`
  (unit).

- 2026-08-22 (session 1i): Portability mixin landed — portability.rs.
  _compact_session_cols (schema-derived, cached, excludes system_prompt*),
  distinct_session_cwds, list_cron_job_runs (id prefix range scan, newest
  first, preview+last_active enriched), _get_session_rich_rows_batch
  (900-chunked IN; single-row + public wrappers; compact_rows projection),
  list_skill_scaffolded_sessions, get_first_assistant_text,
  export_session / export_session_lineage / export_all (+ get_compression_
  lineage + _is_explicit/_is_compression_child_row), search_sessions
  (+ workspace-key/cwd-prefix clauses), import_sessions (full validation:
  size limits char→byte-str exact, text-field coercions, reasoning JSON
  round-trip, parent wiring with cycle detection that pops the broken
  edge, atomic single-txn import; shared-prompt dedup). New crud helpers:
  get_session_dict (full-row JSON dict), get_messages_dicts (decoded rows,
  0/1 ints), CONTENT_JSON_PREFIX pub(crate). K NOWN: pre-existing
  hermes-constants flake (paths::display_shorthand_under_home /
  home::profile_fallback_no_warning_for_default race their own cached
  home/env statics under full-workspace parallelism; unrelated to state
  port — scheduled-outside scope). Oracle: TestListCronJobRuns,
  TestCompactRows rich-row shapes, TestDeleteAndExport import guards,
  test_session_system_prompt_dedup import dedup, lineage walk
  (get_compression_lineage hand-mirror); 13 parity tests; workspace 259
  tests green; clippy clean. Evidence: `cargo test --workspace` (unit).

- 2026-08-22 (session 1j): Search mixin landed — search.rs (largest state
  surface, 2,305 upstream LOC). search_messages full routing: unicode61
  FTS5 → CJK-bigram (when available) → trigram (>=3 CJK chars/token,
  tokenizer present) → LIKE substring (short CJK, lone 1-char runs,
  role='tool' queries), with sort newest/oldest, include_inactive
  (rewound hidden / compacted-archived discoverable #38763), source/
  exclude/role filters, fields projection (context-aware enrichment),
  context (1-before/after WITH TARGET), deferred-rebuild unindexed-gap
  supplement, pure-Latin-miss bigram/trigram recovery (#54242), slow-
  search log (HERMES_SEARCH_SLOW_MS). sanitize_fts5_query (linear quote
  scan, special-char strip, collapse, dangling-operator prune, dotted/
  hyphen quoting; unit parity). FTS engine: fts_rebuild_status/step/
  finish + CJK counterparts, chunked trash teardown (PK high-water),
  marker seeding, repair-bookkeeping, fts_optimize_available,
  _demote_legacy_fts_to_trash, optimize_fts_storage (throttled chunks +
  settle + vacuum + layout stamp), optimize_fts, rebuild_fts,
  _merge_fts_incrementally (usermerge floor + bounded merge in
  try_incremental_merge_fts cadence), runtime corruption self-heal
  (_try_runtime_fts_rebuild). get_anchored_view + get_messages_around,
  list_recent_user_messages (display_kind exclusion, vendored
  compression-handoff prefixes via tools/gen_compression_prefixes.py →
  golden_compression_prefixes.json). search_sessions_by_id DEFERRED
  (depends on list_sessions_rich, "surface read helpers" unit).
  Oracle: TestFTS5Search sanitizer/projection/context, TestCJK
  SearchFallback (ranges, mixed CJK+EN, %-escape), exclude-sources +
  tool-visibility regressions, test_get_anchored_view, rebuild chunk
  loop (30 msgs), fresh-DB optimize settle; 15 parity tests; workspace
  274 tests green; clippy clean. Evidence: `cargo test --workspace`
  (unit).
