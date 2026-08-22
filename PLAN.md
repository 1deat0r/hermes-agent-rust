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
    hermes-toolsets/       # toolsets.py, toolset_distributions.py, model_tools.py   (Phase 2)
    hermes-agent/          # run_agent.py + agent/            (Phase 2)
    hermes-tools/          # tools/                           (Phase 2)
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

### hermes_state (upstream: hermes_state.py, 9,996 LOC) — 🟡 partial (foundation)
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
| portability mixin (export/import, rich rows) | ❌ (hermes_state_portability.py, 714 LOC) | — |
| search mixin (search_messages, FTS rebuild engine, anchored views) | ❌ (hermes_state_search.py, 2,305 LOC) | — |
| compression locks (try_acquire/release/refresh, holder-dead reclaim, publish_compression_child, find_live_compression_child, reopen_orphaned_compression_session) | ✅ | locks ((+ _non_continuation_child_filter_sql in common)) |
| token writer / telegram topics / handoffs / prune/archive | ❌ next units | — |

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

Evidence format: every claim in this file must cite `unit` | `mock` | `live`
+ the exact command, e.g. `cargo test -p hermes-time (unit)`.

## 7. Session log

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
  REMAINING state subsystem: compression locks (try_acquire/release/
  refresh), token writer (queue_token_counts/flush), telegram topics,
  handoffs, prune/archive, replace_messages/rewind, surface read helpers
  (list_sessions_rich, list_gateway_sessions, counts) then
  hermes_state_portability + hermes_state_search (FTS rebuild engine,
  search_messages).
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
