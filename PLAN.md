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
7. **Completion gates stay executable.** `GATES.md` is the root contract for
   full-conversion closure; the `.unlazy/hermes-conversion/` depth tree records
   leaf ownership, dependencies, and current evidence without replacing this
   plan or the generated inventory.

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
    hermes-providers/      # providers/base.py + providers/__init__.py + bundled profiles (Phase 2; base/registry plus Actual/AI Gateway/Alibaba/Alibaba Coding Plan/Anthropic/Arcee/Azure Foundry/Bedrock/Copilot/Copilot ACP/Custom/DeepInfra/DeepSeek/Fireworks/Gemini/GMI/Kilo/Kimi Coding/Minimax/Novita/NVIDIA/Nous/Ollama Cloud/OpenAI Codex/Qwen OAuth/StepFun/Upstage/Vertex/XAI/Xiaomi/ZAI/Hugging Face profiles landed, CLI-version/opener and remaining loaders remain)
    hermes-agent/          # run_agent.py + agent/            (Phase 2; auxiliary_client routing predicate slice partial, client/transport surface pending)
    hermes-tools/          # tools/ (✅ registry, schema_sanitizer, ansi_strip, clarify_tool, session_search_tool, file_safety, read_extract, file_state, path_security, binary_extensions, budget_config, tool_result_storage, tts_text_normalize)   (Phase 2)
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

**P2 status: 🟡 OPEN — hermes-tools support wave and provider profile
base/registry/Actual/AI Gateway/Alibaba/Alibaba Coding Plan/Anthropic/Arcee/Azure Foundry/Bedrock/Copilot/Copilot ACP/Custom/DeepInfra/DeepSeek/Fireworks/Gemini/GMI/Kilo/Kimi Coding/Minimax/Novita/NVIDIA/Nous/Ollama Cloud/OpenAI Codex/Qwen OAuth/StepFun/Upstage/Vertex/XAI/Xiaomi/ZAI/Hugging Face profiles landed (2026-08-24).** The support wave below
ports small, dependency-light modules needed by the agent surface.
The first `hermes-agent::auxiliary_client` sections are now partial: provider
alias normalization, max-token wire-key selection, payment/rate-limit/
model-error predicates, and the explicit task provider/model/endpoint
precedence resolver are ported; client construction, credential pools, async
transport, cancellation, and fallback chains remain pending. The next
credential-safe pool-runtime projection is also ported: runtime key/access
token fallback, runtime/inference/base/fallback URL precedence, normalization,
and the Nous-only inference override are explicit adapter inputs; JWT
validation and secret lookup remain in the auth-layer seam. OpenAI-compatible
endpoint normalization and the exact Anthropic host guard are also ported;
SDK construction, proxy/TLS bootstrap, and request transport remain pending.
The transport-independent OpenAI client options now also preserve the source's
`max_retries=0` default and explicit retry override; SDK/httpx construction
remains pending.
`hermes-providers` now contains the declarative `providers.base` profile,
secure model-catalog probe, process-global registry/discovery cache, and the
first statically linked bundled profiles (`actual`, `ai-gateway`, `alibaba`, `alibaba-coding-plan`, `anthropic`, `arcee`, `azure-foundry`, `bedrock`, `copilot`, `copilot-acp`, `custom`, `deepinfra`, `deepseek`, `fireworks`, `gemini`, `gmi`, `huggingface`, `kilocode`, `kimi-coding`, `minimax`, `novita`, `nvidia`, `nous`, `ollama-cloud`, `openai-codex`, `qwen-oauth`, `stepfun`, `upstage`, `vertex`, `xai`, `xiaomi`, `zai`). The provider surface
remains partial until the future CLI crate supplies the runtime version used
in `_profile_user_agent`, the application-installed urllib opener policy is
represented, and the remaining bundled/user provider plugin profiles have
Rust loaders. The modules marked partial retain explicit seams; those seams
are recorded rather than silently treated as complete.

**P0 status: ✅ COMPLETE (2026-08-22).** Exit criteria met:
`cargo build --workspace` green; `cargo test --workspace` green (74 tests:
hermes-constants lib 45 + parity 8, hermes-time lib 13 + parity 8); clippy
clean via `cargo clippy --workspace --all-targets`; PLAN + ledger current
(`tools/inventory.sh` → prod 1,103 modules / 843,792 LOC, status counts
tracked); P1 foundation underway with hermes-constants (foundational subset)
and hermes-time (full) landed. Evidence tier: unit.

## 5. Parity matrix (foundation + current Phase 2 wave)

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

### hermes-tools support wave (Phase 2, upstream @ b9aa928)

| Module / upstream surface | Status | Rust home, oracle, and evidence tier |
|---|---|---|
| tools/audio_container.py (97 LOC) | ✅ | `audio_container.rs`; 6 parity tests (`unit`) |
| tools/computer_use/schema.py (353 LOC) | ✅ | generated `computer_use_schema.rs`; byte-identical golden + 4 parity tests (`unit`) |
| tools/credential_files.py (530 LOC) | 🟡 | `credential_files.rs`; 27 parity tests (`mock`); config, external-skills, atexit, and ContextVar→thread-local seams remain |
| tools/daemon_pool.py (64 LOC) | ✅ | `daemon_pool.rs`; 10 parity tests (`unit`) |
| tools/debug_helpers.py (105 LOC) | ✅ | `debug_helpers.rs`; 5 parity tests (`mock`) |
| tools/delegation_output_schema.py (151 LOC) | 🟡 | `delegation_output_schema.rs`; 17 active parity tests (`mock`), 2 validator tests ignored pending real `jsonschema` wiring |
| tools/desktop_ui.py (40 LOC) | ✅ | `desktop_ui.rs`; 2 emitter-routing parity tests (`mock`) |
| tools/env_probe.py (370 LOC) | ✅ | `env_probe.rs`; 13 fake-executable/cache parity tests (`mock`) |
| tools/fal_common.py (163 LOC) | ✅ | `fal_common.rs`; 47 injected-client/dependency parity tests (`mock`) |
| tools/interrupt.py (124 LOC) | ✅ | `interrupt.rs`; 4 parity tests (`unit`); broader agent/gateway signal tests deferred with those crates |
| tools/mcp_schema_cache.py (121 LOC) | ✅ | `mcp_schema_cache.rs`; 12 cache/fingerprint parity tests (`mock`) |
| tools/read_preview_tool.py (94 LOC) | ✅ | `read_preview_tool.rs`; 8 callback/schema parity tests (`mock`) |
| tools/read_terminal_tool.py (89 LOC) | ✅ | `read_terminal_tool.rs`; 9 callback/schema parity tests (`mock`); no dedicated upstream test file, so source is the oracle |
| tools/slash_confirm.py (167 LOC) | ✅ | `slash_confirm.rs`; 16 confirmation-state parity tests (`mock`) |
| tools/terminal_hints.py (170 LOC) | ✅ | `terminal_hints.rs`; 17 pattern/order parity tests (`unit`) |
| tools/thread_context.py (120 LOC) | ✅ | `thread_context.rs`; 6 context/cleanup parity tests (`mock`) |
| tools/threat_patterns.py (284 LOC) | 🟡 | `threat_patterns.rs`; 24 pattern/scope parity tests (`unit`); full NFKC and Python full-Unicode case-folding deferred |
| tools/todo_tool.py (335 LOC) | 🟡 | `todo_tool.rs`; 23 store/schema parity tests (`mock`); AIAgent-owned task context and exact Python non-string `str()` coercion remain seams |
| tools/tool_backend_helpers.py (311 LOC) | 🟡 | `tool_backend_helpers.rs`; 40 helper parity tests (`mock`); CLI/provider/secret-scope/credential-pool integrations deferred |
| tools/tool_output_limits.py (110 LOC) | 🟡 | `tool_output_limits.rs`; 9 config/coercion parity tests (`mock`); hermes-cli config loader is an injected seam |
| tools/working_diff.py (130 LOC) | ✅ | `working_diff.rs`; 11 subprocess/diff parity tests (`mock`) |
| Workspace evidence for this wave | ✅ | `cargo build --workspace` and `cargo test --workspace` (`unit`/`mock`); final result recorded in §7 |

### hermes-agent credential pool (Phase 2, upstream @ b9aa928)

| Module / upstream surface | Status | Rust home, oracle, and evidence tier |
|---|---|---|
| `agent/credential_pool.py` (3,147 LOC) — selection/rotation, pooled-row model/serialization, source upsert, priority normalization, strategy parsing, and custom-provider identity sections | 🟡 | `hermes-agent::credential_pool` plus `hermes-agent::credential_store`; 81 source-derived parity tests (`unit`/`mock`, 66 pool + 15 persistence) cover fill-first priority/current/peek, least-used counting, round-robin order, random selection support, explicit reset timestamps, terminal `token_invalidated` → `DEAD` rotation, unmatched-key fail-open rotation, duplicate-key quarantine, sole-credential transient-versus-billing cooldowns, `PooledCredential` JSON defaults/metadata round-trip, Anthropic OAT auth normalization and seeded-priority ordering, borrowed-secret redaction/fingerprints, owned OAuth persistence exceptions, Nous invoke-JWT runtime selection, token labels/runtime base URLs, source-key upsert and key rotation, configured strategies, custom endpoint/name scoping, versioned auth-store defaults, legacy `systems` migration, stale Nous URL migration, corruption quarantine versus read-error propagation, atomic `0600`/`0700` writes, profile/global fallback reads, profile-scoped borrowed-secret-safe writes, newer/live cooldown recency merging with token-change and expiry guards, per-path reentrant cross-process auth-store locking, dotenv parsing, dotenv-over-process precedence, process fallback, env-source suppression/pruning, unresolved `op://` secret-scope substitution, Anthropic OAuth-prefix classification, the lower environment-aware `load_pool` transaction, explicit Nous singleton state seeding with invoke-JWT agent-key/runtime metadata preservation, Qwen CLI OAuth token/source/expiry/base-URL/label seeding with absent-token fail-open, MiniMax OAuth token/refresh/ISO-expiry/base-URL/label seeding with suppression, OpenAI Codex nested-token/fixed-endpoint/last-refresh/label seeding with suppression, xAI OAuth nested-token/fixed-endpoint/last-refresh/label seeding with suppression, Anthropic resolved `hermes_pkce`/`claude_code` OAuth seeding with provider opt-in, API-key-path pruning, and source suppression, Copilot resolved exchanged-token seeding with CLI/env source classification, all-source/per-source suppression, and enterprise/default endpoint mapping, custom-pool config/model API-key seeding with normalized endpoints and suppression, the explicit full `load_pool` composition boundary with custom-versus-non-custom branch selection, singleton/environment ordering, stale-row pruning, priority normalization, and persistence, the pure custom-provider compatibility adapter with legacy/keyed schema merge, URL precedence, aliases, model normalization, enablement filtering, deduplication, and fail-open malformed legacy handling, the transport-neutral OAuth refresh result/application boundary with expiry-skewed deferred re-selection, fail-open refresh exhaustion, and borrowed-safe refresh sanitization, and the read-only config snapshot/signature cache with config-to-pool projection; the environment, singleton, loader, compatibility, refresh, and config snapshot boundaries now accept explicit provider/config/auth inputs, while full merged CLI config discovery/loading, concrete Z.AI HTTP probing/cache, provider-specific OAuth transport/auth-store write-through, lease locking, and logging throttles remain pending |

Intentional credential-pool seam: the environment seeder, lower
  environment-aware `load_pool` transaction, current Anthropic/Nous/Copilot/
  Qwen/MiniMax/OpenAI Codex/xAI singleton seeders, and custom-pool config/model
  seeders and the full `load_pool` composition boundary receive provider registry
  metadata, pool config, resolved provider singleton state, secret-scope values,
  suppression state, and auth-store paths as
explicit inputs because `hermes-agent` is below the CLI/provider crates in the
planned dependency graph. Anthropic's explicit provider/API-key-path gates and
  resolved `hermes_pkce`/`claude_code` credential-file outputs are represented
  by that input map; Kimi's pure key-prefix endpoint routing, the Nous
  state-to-pool field copy, Copilot resolved exchanged-token/source/endpoint
  field copy, Qwen resolved-credential field copy, MiniMax
  OAuth state-to-pool field copy, OpenAI Codex nested-token field copy, xAI
  OAuth nested-token field copy, custom config/model API-key field copy, the
  custom-versus-non-custom loader branch ordering, and the pure custom-provider
  compatibility adapter are mirrored; outer config discovery/loading and
  Z.AI's network endpoint probe remain deferred to the owning auth/provider
  layer rather than being silently guessed. The OAuth refresh boundary accepts
  already-resolved provider results so this crate remains below the
  auth/transport layers; network calls, cross-process auth-store write-through,
  and provider-specific terminal quarantine remain deferred rather than
  silently guessed.

### hermes-agent auxiliary client (Phase 2, upstream @ b9aa928)

| Module / upstream surface | Status | Rust home, oracle, and evidence tier |
|---|---|---|
| `agent/auxiliary_client.py` (10,044 LOC) — routing, wire-parameter, task-provider precedence, pool-runtime projection, endpoint normalization, client-option, proxy/TLS policy, Codex credential-header, Codex token-selection, keepalive transport options, concrete reqwest client construction, and pool-first runtime credential sections | 🟡 | `hermes-agent::auxiliary_client`; 41 source-derived parity tests (`unit`) cover provider aliases/special forms, OpenAI-compatible max-token keyword selection, payment/quota and rate-limit classification, disjoint stale-model/capability errors, explicit/configured endpoint and key precedence, MoA unwrapping, direct OpenAI aliasing, `auto` model normalization, pool key fallback and fail-open selection states, URL precedence/normalization, the Nous-only base-URL override, MiniMax/Z.AI/Kimi OpenAI wire paths, exact Anthropic host validation, default/explicit SDK retry options, env proxy precedence/SOCKS normalization/`NO_PROXY` bypass, keepalive pool/timeout/mount options for sync and async modes, concrete sync/async reqwest client selection, explicit proxy forwarding, full PEM-bundle TLS roots, insecure TLS, fail-open construction, Codex originator/User-Agent/account-ID header shaping, pool-first/auth-store Codex token selection, pool-first runtime credential fallback, JWT expiry filtering, and non-JWT fail-open behavior; full SDK request/response adapters, exact max-connection/write/pool-timeout wiring, full credential-pool lifecycle/rotation, cancellation, and provider fallback chains remain pending |

### hermes-providers base (Phase 2, upstream @ b9aa928)

| Module / upstream surface | Status | Rust home, oracle, and evidence tier |
|---|---|---|
| plugins/model-providers/upstage/__init__.py (115 LOC) | ✅ | `hermes-providers::profiles::upstage`; 4 source-derived profile/reasoning-mapping parity tests (`unit`); Upstage Solar metadata, alias, endpoint/env contract, `solar-pro3` fallback, deny-listed model families, medium default, explicit effort mapping, minimal omission, unknown/high clamp, and disabled omission are ported; CLI provider overlay and full transport integration remain future higher-layer seams |
| plugins/model-providers/qwen-oauth/__init__.py (108 LOC) | ✅ | `hermes-providers::profiles::qwen_oauth`; 4 source-derived profile/message-normalization/request-hook parity tests (`unit`); aliases, QWEN_API_KEY, Portal URL/OAuth auth, 65,536 max-token cap, first-system cache metadata, nested image retry-copy guard, high-resolution image body, and top-level session metadata are ported; Qwen CLI OAuth credential resolution and full transport integration remain future higher-layer seams |
| providers/base.py (238 LOC) — declarative profile fields/defaults and no-op hooks | ✅ | `hermes-providers::base`; 9 parity tests (`unit`/`mock`); Actual environment-aware catalog discovery, AI Gateway reasoning passthrough, Anthropic native model discovery, Gemini and Vertex thinking translation, Copilot catalog-gated reasoning, Custom/Ollama reasoning and user-catalog gating, DeepInfra vision-catalog discovery, DeepSeek V4+ reasoning-wire mapping, Nous Portal tags/reasoning omission, Ollama Cloud top-level reasoning-effort translation, and MiniMax M3 route-gated reasoning are represented by profile capabilities |
| `get_hostname`, `get_max_tokens`, and `OMIT_TEMPERATURE` | ✅ | `base.rs`; included in the focused profile tests (`unit`) |
| `fetch_models` endpoint precedence, JSON shaping, strict fail-open behavior | ✅ | `base.rs`; loopback standard and Anthropic-native catalog tests (`mock`) |
| Kimi Coding model-discovery and reasoning capability | ✅ | `base.rs`; exact Coding endpoint confirmation, /coding to /coding/v1 normalization, unconfirmed k3 filtering, and mutually-exclusive thinking/reasoning_effort mapping covered by `parity_kimi_coding.rs` (`unit`/`mock`) |
| credential-safe redirects (same-origin retention, cross-origin `accept`/`user-agent` allowlist) | ✅ | `base.rs`; 2 redirect tests (`mock`) |
| `_profile_user_agent` runtime CLI version and installed urllib opener policy | 🟡 | stable fallback is implemented; CLI-version injection and application opener integration remain with `hermes-cli` |
| providers/__init__.py canonical/alias registry, cache, and lazy discovery order | ✅ | `hermes-providers::registry`; 8 parity tests (`unit`/`mock`) |
| Qwen Portal bundled registration and alias resolution | ✅ | `hermes-providers::profiles::qwen_oauth`; covered by `parity_qwen_oauth.rs` and the registry order assertions (`unit`) |
| Upstage Solar bundled registration and alias resolution | ✅ | `hermes-providers::profiles::upstage`; covered by `parity_upstage.rs` and the registry order assertions (`unit`) |
| Kimi Coding bundled registration and alias resolution | ✅ | `hermes-providers::profiles::kimi_coding`; covered by `parity_kimi_coding.rs` and the registry order assertions (`unit`) |
| Z.AI GLM bundled registration, aliases, and reasoning translation | ✅ | `hermes-providers::profiles::zai`; covered by `parity_zai.rs` and the registry order assertions (`unit`) |
| providers/__init__.py bundled/user/legacy import execution | 🟡 | filesystem scan and explicit loader seam are implemented; statically linked Actual, AI Gateway, Alibaba, Alibaba Coding Plan, Anthropic, Arcee, Azure Foundry, Bedrock, Copilot, Copilot ACP, Custom, DeepInfra, DeepSeek, Fireworks, Gemini, GMI, Hugging Face, Kilo, Kimi Coding, MiniMax (three profiles), Novita, NVIDIA, Nous, Ollama Cloud, OpenAI Codex, StepFun, Upstage, Vertex, XAI, Xiaomi, and ZAI are wired, remaining Rust plugin profiles/loaders remain pending |
| plugins/model-providers/actual/__init__.py (89 LOC) | ✅ | `hermes-providers::profiles::actual`; 3 source-derived profile/catalog parity tests (`unit`/`mock`) mirror aliases, environment-precedence URL normalization, optional Bearer auth, headers, list/`data` response shapes, and fail-open parsing; runtime credentials/model-picker/transport integration remains a future hermes-cli seam |
| plugins/model-providers/ai-gateway/__init__.py (43 LOC) | ✅ | `hermes-providers::profiles::ai_gateway`; 2 source-derived profile/registration/reasoning-hook parity tests (`unit`); `reasoning_passthrough` represents the upstream `build_api_kwargs_extras` override; related CLI/model catalog tests remain future-crate oracles |
| plugins/model-providers/alibaba/__init__.py (13 LOC) | ✅ | `hermes-providers::profiles::alibaba`; 2 source-derived parity tests (`unit`); no dedicated upstream test module |
| plugins/model-providers/alibaba-coding-plan/__init__.py (21 LOC) | ✅ | `hermes-providers::profiles::alibaba_coding_plan`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/anthropic/__init__.py (54 LOC) | ✅ | `hermes-providers::profiles::anthropic`; 3 source-derived parity tests (`unit`/`mock`) mirror the native profile fields and custom `x-api-key` model fetch; fixed endpoint and fail-open behavior remain explicit, with CLI/agent tests future-crate oracles |
| plugins/model-providers/arcee/__init__.py (13 LOC) | ✅ | `hermes-providers::profiles::arcee`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/azure-foundry/__init__.py (21 LOC) | ✅ | `hermes-providers::profiles::azure_foundry`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/bedrock/__init__.py (30 LOC) | ✅ | `hermes-providers::profiles::bedrock`; 2 source-derived parity tests (`unit`); AWS SDK model-discovery override represented by `models_fetch_disabled`; related Bedrock adapter/transport tests remain future-crate oracles |
| plugins/model-providers/copilot/__init__.py (74 LOC) | ✅ | `hermes-providers::profiles::copilot`; 3 source-derived parity tests (`unit`) mirror profile fields, catalog-gated reasoning, and exact clamp precedence; live CLI/model-catalog lookup remains an explicit injected seam |
| plugins/model-providers/copilot-acp/__init__.py (35 LOC) | ✅ | `hermes-providers::profiles::copilot_acp`; 2 source-derived parity tests (`unit`); external ACP model-discovery override represented by `models_fetch_disabled`; no dedicated upstream plugin-profile test module |
| plugins/model-providers/custom/__init__.py (103 LOC) | ✅ | `hermes-providers::profiles::custom`; 4 source-derived reasoning/num_ctx/profile parity tests (`unit`); six aliases, 65,536 default max tokens, disabled/`none` dual wire fields, effort passthrough, empty-config omission, and user-configured catalog guard are ported; broader CLI/custom endpoint integration remains a future higher-layer seam |
| plugins/model-providers/deepinfra/__init__.py (81 LOC) | ✅ | `hermes-providers::profiles::deepinfra`; 2 source-derived profile/registration/vision-catalog parity tests (`unit`/`mock`); key-gated chat+vision tag selection and process-global cache are ported, while profile-scoped secret resolution and CLI opener integration remain future-crate seams |
| plugins/model-providers/deepseek/__init__.py (102 LOC) | ✅ | `hermes-providers::profiles::deepseek`; 2 source-derived profile/registration/reasoning-wire parity tests (`unit`); V4+ model gating, thinking enabled/disabled shape, and effort clamp mapping are ported; broader transport/history replay tests remain future-crate oracles |
| plugins/model-providers/fireworks/__init__.py (46 LOC) | ✅ | `hermes-providers::profiles::fireworks`; 2 parity tests (`unit`) mirror the dedicated upstream profile tests; pinned `HermesAgent/0.20.0` attribution header awaits future CLI-version wiring; related CLI/runtime tests remain future-crate oracles |
| plugins/model-providers/gemini/__init__.py (61 LOC) | ✅ | `hermes-providers::profiles::gemini`; 3 source-derived parity tests (`unit`) mirror the profile fields, native thinking-config hook, and OpenAI-compatible nested snake_case branch; related native client/transport tests remain future-crate oracles |
| plugins/model-providers/gmi/__init__.py (32 LOC) | ✅ | `hermes-providers::profiles::gmi`; 2 source-derived parity tests (`unit`); pinned `HermesAgent/0.20.0` attribution header awaits future CLI-version wiring; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/huggingface/__init__.py (20 LOC) | ✅ | `hermes-providers::profiles::huggingface`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/kilocode/__init__.py (14 LOC) | ✅ | `hermes-providers::profiles::kilocode`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/kimi-coding/__init__.py (121 LOC) | ✅ | `hermes-providers::profiles::kimi_coding`; 3 source-derived profile/reasoning/model-discovery parity tests (`unit`/`mock`); global and China metadata, Kimi Coding URL confirmation and /v1 path normalization, fail-open unconfirmed k3 filtering, mutually-exclusive thinking/reasoning_effort mapping, and sorted registration are ported; CLI credential auto-detection and full transport integration remain future higher-layer seams |
| plugins/model-providers/minimax/__init__.py (97 LOC) | ✅ | `hermes-providers::profiles::minimax`; 3 source-derived profile/registration/reasoning-wire parity tests (`unit`); direct, China, and OAuth metadata plus MiniMax-M3 global OpenAI-compatible `reasoning_split`/adaptive/disabled hook are ported; auxiliary-client, OAuth runtime, and broader agent/transport integration remain future higher-layer oracles |
| plugins/model-providers/novita/__init__.py (27 LOC) | ✅ | `hermes-providers::profiles::novita`; 2 source-derived parity tests (`unit`); upstream `tests/hermes_cli/test_api_key_providers.py` covers profile loading and pricing-cache behavior; pricing helper remains a future hermes-cli seam |
| plugins/model-providers/nvidia/__init__.py (21 LOC) | ✅ | `hermes-providers::profiles::nvidia`; 2 source-derived parity tests (`unit`); upstream provider profile/wiring tests cover max-token and endpoint behavior; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/stepfun/__init__.py (14 LOC) | ✅ | `hermes-providers::profiles::stepfun`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/vertex/__init__.py (75 LOC) | ✅ | `hermes-providers::profiles::vertex`; 3 source-derived parity tests (`unit`) cover OAuth metadata, nested Gemini thinking, and disabled REST discovery; no dedicated upstream profile test module, and the runtime OAuth adapter remains a future-crate seam |
| plugins/model-providers/openai-codex/__init__.py (15 LOC) | ✅ | `hermes-providers::profiles::openai_codex`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/TUI tests remain future-crate oracles |
| plugins/model-providers/xiaomi/__init__.py (16 LOC) | ✅ | `hermes-providers::profiles::xiaomi`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/xai/__init__.py (17 LOC) | ✅ | `hermes-providers::profiles::xai`; 2 source-derived parity tests (`unit`); no dedicated upstream plugin-profile test module; pinned `Hermes-Agent/0.20.0` header awaits future CLI-version wiring; related CLI/agent tests remain future-crate oracles |
| plugins/model-providers/zai/__init__.py (127 LOC) | ✅ | `hermes-providers::profiles::zai` plus the public pure endpoint helpers in `hermes-providers::zai`; 10 source-derived profile/GLM-gating/endpoint parity tests (`unit`) cover metadata, aliases, GLM 4.5+ thinking toggles, GLM-5.2 aliases/effort mapping, the four auth endpoint specs, candidate-model fallback, priority selection, all-fail behavior, and env/cache/detected URL precedence; concrete HTTP probing, early-exit threading, auth-store cache persistence, and CLI credential/model-picker integration remain future higher-layer seams |
| plugins/model-providers/nous/__init__.py (88 LOC) | ✅ | `hermes-providers::profiles::nous`; 3 source-derived profile/registration/Portal-hook parity tests (`unit`); pinned `HermesAgent/0.20.0` client tag and explicit conversation-context adapter preserve the current source contract while runtime CLI version/context propagation remain future higher-layer seams |
| plugins/model-providers/ollama-cloud/__init__.py (89 LOC) | ✅ | `hermes-providers::profiles::ollama_cloud`; 3 source-derived profile/reasoning-wire parity tests (`unit`); top-level `reasoning_effort` capability gate, disable/`none` switch, xhigh/max/ultra normalization, standard effort passthrough, and unknown-effort omission are ported; `/api/show` capability probing, dynamic catalog merging, and CLI credential/model-picker seams remain future higher-layer oracles |

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
- `tests/tools/test_audio_container.py` → `crates/hermes-tools/tests/parity_audio_container.rs`
- `tools/computer_use/schema.py` → `crates/hermes-tools/tests/parity_computer_use_schema.rs` + `upstream/golden_computer_use_schema.json`
- `tests/tools/test_credential_files.py` → `crates/hermes-tools/tests/parity_credential_files.rs`
- `tests/tools/test_daemon_pool.py` → `crates/hermes-tools/tests/parity_daemon_pool.rs`
- `tests/tools/test_debug_helpers.py` → `crates/hermes-tools/tests/parity_debug_helpers*.rs`
- `tests/tools/test_delegate_output_schema.py` → `crates/hermes-tools/tests/parity_delegation_output_schema.rs`
- `tests/tools/test_desktop_ui.py` → `crates/hermes-tools/tests/parity_desktop_ui.rs`
- `tests/tools/test_env_probe.py` → `crates/hermes-tools/tests/parity_env_probe.rs`
- `tests/tools/test_fal_common.py` → `crates/hermes-tools/tests/parity_fal_common.rs`
- `tests/tools/test_interrupt.py` → `crates/hermes-tools/tests/parity_interrupt.rs` (agent/gateway interrupt suites remain deferred)
- `tests/tools/test_mcp_schema_cache.py` → `crates/hermes-tools/tests/parity_mcp_schema_cache.rs`
- `tests/tools/test_read_preview_tool.py` → `crates/hermes-tools/tests/parity_read_preview_tool.rs`; `read_terminal_tool.py` is code-oracle-only because upstream has no dedicated test module
- `tests/tools/test_slash_confirm.py` → `crates/hermes-tools/tests/parity_slash_confirm.rs`
- `tests/tools/test_terminal_hints.py` → `crates/hermes-tools/tests/parity_terminal_hints.rs`
- `tools/thread_context.py` → `crates/hermes-tools/tests/parity_thread_context.rs` (no dedicated upstream test module)
- `tests/tools/test_threat_patterns.py` → `crates/hermes-tools/tests/parity_threat_patterns.rs`
- `tests/tools/test_todo_tool.py` + `tests/tools/test_todo_tool_type_coercion.py` → `crates/hermes-tools/tests/parity_todo_tool.rs` + `upstream/golden_todo_schema.json`
- `tests/tools/test_tool_backend_helpers.py` → `crates/hermes-tools/tests/parity_tool_backend_helpers.rs`
- `tests/tools/test_tool_output_limits.py` → `crates/hermes-tools/tests/parity_tool_output_limits.rs`
- `tests/tools/test_working_diff.py` → `crates/hermes-tools/tests/parity_working_diff.rs`
- `providers/base.py` + `tests/providers/test_fetch_models_base_url.py` → `crates/hermes-providers/tests/parity_base.rs` (9 profile/catalog/redirect parity tests; `unit`/`mock`)
- `agent/auxiliary_client.py` + `agent/process_bootstrap.py` + `agent/ssl_verify.py` + proxy/TLS/Codex-header/token tests → `crates/hermes-agent/tests/parity_auxiliary_client.rs` (41 source-derived routing/wire/error/task-provider-resolution/pool-runtime/endpoint/client-option/proxy/TLS/Codex-header/token/client-construction parity tests; `unit`; the Rust `AuxiliaryTlsVerify` value preserves source precedence and fail-open decisions, `build_auxiliary_http_client` constructs concrete blocking/async reqwest clients with explicit proxy/no-proxy policy and PEM-bundle roots, `codex_cloudflare_headers` preserves fixed-header/JWT extraction behavior, and `read_codex_access_token` preserves pool-first/auth-store/expiry behavior through explicit lookup adapters, while full SDK request/response integration, exact max-connection/write/pool timeout wiring, remaining credential pools, cancellation, and provider fallback oracles remain future sections)
- `providers/__init__.py` + `tests/providers/test_provider_registry.py` + `tests/providers/test_plugin_discovery.py` → `crates/hermes-providers/tests/parity_registry.rs` (8 registry/discovery parity tests; `unit`/`mock`)
- `plugins/model-providers/ai-gateway/__init__.py` → `crates/hermes-providers/tests/parity_ai_gateway.rs` (2 source-derived profile/registration/reasoning-hook parity tests; `unit`; no dedicated upstream plugin-profile test; related CLI/model catalog tests remain future-crate oracles)
- `plugins/model-providers/alibaba/__init__.py` → `crates/hermes-providers/tests/parity_alibaba.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream test module)
- `plugins/model-providers/alibaba-coding-plan/__init__.py` → `crates/hermes-providers/tests/parity_alibaba_coding_plan.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/anthropic/__init__.py` → `crates/hermes-providers/tests/parity_anthropic.rs` (3 source-derived profile/registration/native model-fetch parity tests; `unit`/`mock`; no dedicated upstream plugin-profile test module; fixed native endpoint and CLI/agent integrations remain future-crate seams)
- `plugins/model-providers/arcee/__init__.py` → `crates/hermes-providers/tests/parity_arcee.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/azure-foundry/__init__.py` → `crates/hermes-providers/tests/parity_azure_foundry.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/bedrock/__init__.py` → `crates/hermes-providers/tests/parity_bedrock.rs` (2 source-derived profile/registration/fetch-override parity tests; `unit`; AWS SDK discovery is represented by the shared `models_fetch_disabled` capability)
- `plugins/model-providers/copilot/__init__.py` + `tests/plugins/model_providers/test_copilot_profile.py` → `crates/hermes-providers/tests/parity_copilot.rs` (3 profile/registration/catalog-gated reasoning parity tests; `unit`; live `github_model_reasoning_efforts` lookup is represented by an injected context seam)
- `plugins/model-providers/copilot-acp/__init__.py` → `crates/hermes-providers/tests/parity_copilot_acp.rs` (2 source-derived profile/registration/fetch-override parity tests; `unit`; no dedicated upstream plugin-profile test; external ACP discovery is represented by the shared `models_fetch_disabled` capability)
- `plugins/model-providers/custom/__init__.py` + `tests/plugins/model_providers/test_custom_profile.py` → `crates/hermes-providers/tests/parity_custom.rs` (4 source-derived profile/reasoning/num_ctx parity tests; `unit`; user-configured catalog guard and broader CLI/custom endpoint integration remain future higher-layer oracles)
- `plugins/model-providers/deepinfra/__init__.py` + `tests/hermes_cli/test_api_key_providers.py` DeepInfra profile/tag cases → `crates/hermes-providers/tests/parity_deepinfra.rs` (2 profile/registration/vision-catalog parity tests; `unit`/`mock`; secret-scope and full hermes-cli catalog/pricing integrations remain future-crate oracles)
- `plugins/model-providers/deepseek/__init__.py` + provider wiring/transport cases → `crates/hermes-providers/tests/parity_deepseek.rs` (2 source-derived profile/registration/reasoning-wire parity tests; `unit`; broader reasoning-content history repair and live API tests remain future-crate/live oracles)
- `plugins/model-providers/fireworks/__init__.py` + `tests/plugins/model_providers/test_fireworks_profile.py` → `crates/hermes-providers/tests/parity_fireworks.rs` (2 profile/registration/header/model parity tests; `unit`; CLI/provider-resolution tests remain future-crate oracles; pinned `HermesAgent/0.20.0` header awaits future CLI-version wiring)
- `plugins/model-providers/gemini/__init__.py` + `tests/plugins/model_providers/test_gemini_profile.py` + related transport thinking tests → `crates/hermes-providers/tests/parity_gemini.rs` (3 profile/registration/thinking-hook parity tests; `unit`; native client and full transport integrations remain future-crate oracles)
- `plugins/model-providers/gmi/__init__.py` → `crates/hermes-providers/tests/parity_gmi.rs` (2 source-derived profile/registration/header/model parity tests; `unit`; pinned `HermesAgent/0.20.0` header awaits future CLI-version wiring)
- `plugins/model-providers/huggingface/__init__.py` → `crates/hermes-providers/tests/parity_huggingface.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/kilocode/__init__.py` → `crates/hermes-providers/tests/parity_kilocode.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/kimi-coding/__init__.py` + `tests/plugins/model_providers/test_kimi_profile.py` → `crates/hermes-providers/tests/parity_kimi_coding.rs` (3 source-derived profile/reasoning/model-discovery parity tests; `unit`/`mock`; the Rust loopback catalog seam replaces the upstream monkeypatch while preserving malformed-URL filtering)
- `plugins/model-providers/minimax/__init__.py` + `tests/plugins/model_providers/test_minimax_profile.py` → `crates/hermes-providers/tests/parity_minimax.rs` (3 source-derived profile/registration/reasoning-wire parity tests; `unit`; auxiliary-client, OAuth runtime, and broader agent/transport cases remain future higher-layer oracles)
- `plugins/model-providers/novita/__init__.py` → `crates/hermes-providers/tests/parity_novita.rs` (2 source-derived profile/registration parity tests; `unit`; upstream `tests/hermes_cli/test_api_key_providers.py` also covers pricing-cache behavior, deferred with the hermes-cli seam)
- `plugins/model-providers/nvidia/__init__.py` → `crates/hermes-providers/tests/parity_nvidia.rs` (2 source-derived profile/registration parity tests; `unit`; upstream `tests/providers/test_provider_profiles.py` and `test_profile_wiring.py` cover max-token and endpoint/wiring behavior)
- `plugins/model-providers/actual/__init__.py` + `tests/hermes_cli/test_actual_provider.py` → `crates/hermes-providers/tests/parity_actual.rs` (3 source-derived profile/catalog parity tests; `unit`/`mock`; runtime credential/model-picker/transport cases remain future hermes-cli oracles)
- `plugins/model-providers/stepfun/__init__.py` → `crates/hermes-providers/tests/parity_stepfun.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/vertex/__init__.py` → `crates/hermes-providers/tests/parity_vertex.rs` (3 source-derived profile/registration/thinking/fetch-override tests; `unit`; no dedicated upstream profile test; Vertex runtime OAuth adapter remains a future-crate seam)
- `plugins/model-providers/openai-codex/__init__.py` → `crates/hermes-providers/tests/parity_openai_codex.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/xiaomi/__init__.py` → `crates/hermes-providers/tests/parity_xiaomi.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/xai/__init__.py` → `crates/hermes-providers/tests/parity_xai.rs` (2 source-derived profile/registration parity tests; `unit`; no dedicated upstream plugin-profile test module)
- `plugins/model-providers/zai/__init__.py` + `tests/plugins/model_providers/test_zai_profile.py` + provider wiring/transport cases → `crates/hermes-providers/tests/parity_zai.rs` (5 source-derived profile/GLM-gating/reasoning parity tests; `unit`; CLI credential/model-picker and full transport integration remain future higher-layer oracles)

- `plugins/model-providers/nous/__init__.py` + `tests/providers/test_provider_profiles.py` + `tests/agent/transports/test_chat_completions.py` → `crates/hermes-providers/tests/parity_nous.rs` (3 source-derived profile/registration/Portal-hook parity tests; `unit`; pinned CLI version and ambient conversation propagation remain future higher-layer seams)
- `plugins/model-providers/ollama-cloud/__init__.py` + `tests/plugins/model_providers/test_ollama_cloud_profile.py` → `crates/hermes-providers/tests/parity_ollama_cloud.rs` (3 source-derived profile/reasoning-wire parity tests; `unit`; `/api/show` capability probing, dynamic catalog merging, and CLI credential/model-picker cases remain future higher-layer oracles)
- `plugins/model-providers/qwen-oauth/__init__.py` + `tests/providers/test_provider_profiles.py` + `tests/providers/test_profile_wiring.py` + `tests/providers/test_transport_parity.py` → `crates/hermes-providers/tests/parity_qwen_oauth.rs` (4 source-derived profile/message-normalization/request-hook parity tests; `unit`; Qwen CLI OAuth credential resolution and full transport integration remain future higher-layer oracles)
- `plugins/model-providers/upstage/__init__.py` + `tests/plugins/model_providers/test_upstage_profile.py` → `crates/hermes-providers/tests/parity_upstage.rs` (4 source-derived profile/reasoning-mapping parity tests; `unit`; CLI provider overlay and full transport integration remain future higher-layer oracles)

Evidence format: every claim in this file must cite `unit` | `mock` | `live`
+ the exact command, e.g. `cargo test -p hermes-time (unit)`.

## 7. Session log

- 2026-08-24 (session 4cf): Continued the partial `agent.credential_pool`
  and Z.AI provider seams through parallel subagents. Added the read-only
  `hermes-agent::config` snapshot with resolved/default and explicit paths,
  `(mtime_ns,size)` cache invalidation, root pool/model/custom-provider
  projection, missing/malformed/non-map fail-open behavior, and
  valid-to-malformed last-known-good retention. Added the public pure
  `hermes-providers::zai` endpoint table, candidate-model fallback chooser,
  static-priority selection, all-fail behavior, and env/cache/detected
  base-URL precedence; concrete HTTP/threading/auth-store persistence remain
  higher-layer seams. Added 7 config tests, 5 Z.AI tests, and one
  config-to-pool integration test first; the focused credential-pool wave now
  has 66 pool plus 15 persistence tests (81 total), with 10 Z.AI parity tests.
  Parallel leaf gates passed 3/3 each; the integration gate passed 3/3.
  Targeted rustfmt, focused leaf/integration suites, workspace build, and
  serialized workspace tests passed (1,125 passed, 5 ignored, 12 warnings).
  Local source commit `a092a8f21d4ea6deacac92503e67f7e4bdd809df` was mirrored
  as GitHub `a092a8f21d4ea6deacac92503e67f7e4bdd809df`; both refs resolve to
  tree `68766578f9e2d002b4a4f8caee8c4a07e94cf1e7` with 273 matching tracked
  blobs. No ledger status changed. Full merged CLI config discovery/loading,
  concrete Z.AI HTTP probing/cache, OAuth transport/auth-store write-through,
  leases, and logging throttles remain pending.

- 2026-08-24 (session 4cf): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the transport-neutral OAuth refresh boundary.
  Added `OAuthRefreshResult`/`OAuthRefreshError`, successful token replacement
  with status/error clearing and source-compatible optional-field preservation,
  provider-specific expiry-skew detection for Anthropic, OpenAI Codex, and
  xAI, deferred refresh/re-selection through an injected provider callback,
  fail-open refresh exhaustion, and borrowed-secret-safe serialization after
  refresh. Added four source-derived `mock`/`unit` parity tests first; the
  focused credential-pool wave now has 65 pool plus 15 persistence tests
  (80 total). Validation passed: targeted `/home/mustbearnold/.cargo/bin/rustfmt
  --edition 2021` on the two changed Rust files, focused
  `cargo test -p hermes-agent --test parity_credential_pool --test
  parity_credential_store --test parity_auxiliary_client`, `cargo build
  --workspace`, `cargo test --workspace -- --test-threads=1`, `git diff
  --check`, and the approved 5/5-gate credential-lifecycle recheck. No ledger
  status changed:
  inventory remains 73 done / 11 partial / 3,798 missing tracked modules and
  73 done / 11 partial / 1,019 missing production modules. Provider-specific
  OAuth transport/auth-store write-through, outer config discovery/loading,
  Z.AI endpoint probing, leases, and logging throttles remain pending.
  Local source commit `e568a282a692d65ee574ce2ca25db10741b95515` was
  mirrored as GitHub `e568a282a692d65ee574ce2ca25db10741b95515`; both refs
  resolve to tree `646b395e0a3fc431aa0fb09401a3465f92f02022` with 273 matching
  tracked blobs.

- 2026-08-24 (session 4cf): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the pure custom-provider configuration
  compatibility boundary. Added explicit normalization, legacy
  `custom_providers` plus keyed `providers` merging, URL precedence and
  template acceptance, camelCase/snake_case aliases, model-list conversion,
  keyed enablement filtering, case-insensitive deduplication, extra-header
  stringification, input immutability, and malformed-legacy fail-open behavior.
  Added four source-derived `unit` parity tests first; the focused
  credential-pool wave now has 61 pool plus 15 persistence tests (76 total).
  Targeted rustfmt, focused tests, workspace build, serialized workspace test,
  Clippy review, and the approved 4/4-gate leaf recheck passed. Targeted
  `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Outer config discovery/loading, Z.AI endpoint
  probing, OAuth refresh, leases, and logging throttles remain pending. Local
  source commit `9216c3849362a39145b9147394630cfa112171e1` was mirrored as
  GitHub `1167a7381d7623247a37267edd4f16e2df7371e5`; both refs resolve to tree
  `9b6f57ac636b2f247cf5fb196b28fda08081ac3d` with 273 matching tracked blobs.

- 2026-08-24 (session 4cf): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the full explicit-input `load_pool` composition
  boundary. Added `PoolLoadInputs` and `load_pool_with_inputs_at`, preserving
  the upstream `custom:*` branch split, custom config/model seeding, singleton
  then environment ordering, non-destructive environment pruning, custom-pool
  stale-row pruning, Anthropic priority normalization, strategy selection, and
  borrowed-safe persistence. Added two source-derived `mock` parity tests first;
  the focused credential-pool wave now has 57 pool plus 15 persistence tests
  (72 total). Relaxed one persistence assertion only to account for sub-
  microsecond JSON floating-point round-trip precision; no source semantics
  changed. Targeted rustfmt, focused tests, workspace build, serialized
  workspace test, Clippy review, and the approved 4/4-gate leaf recheck passed.
  Targeted `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Local source commit
  `f6a266e6e5ef9417c9b49cf9f2d53563e0ac9ae4` was mirrored as GitHub
  `940ead9f102e6aba3ea0d2c308d7cb9c732cbd5e`; both refs resolve to tree
  `450e2757900fb773f3295e593782ec1a153b077f` with 273 matching tracked blobs.

- 2026-08-24 (session 4cf): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the custom-pool seeding boundary. Added the
  explicit `seed_custom_pool` seam for `custom_providers` API keys and matching
  `model.api_key`/`model.api` values, preserving source labels, endpoint
  normalization, custom-pool matching, and suppression. Added two
  source-derived `mock` parity tests first; the focused credential-pool wave
  now has 55 pool plus 15 persistence tests (70 total). Full loader/config
  composition, configuration discovery, OAuth refresh, leases, and throttles
  remain open. Targeted rustfmt, focused tests, workspace build, serialized
  workspace test, and the approved 4/4-gate leaf recheck passed. Targeted
  `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Local source commit
  `2c7798500593d6ac290c235181da28a7ef81e8d2` was mirrored as GitHub
  `dadb41d8b1e8cf4da277d6b133a81e3314122ad1`; both refs resolve to tree
  `13b3e6306207c0e0b0f8b4d878208aea459b8d4f` with 273 matching tracked blobs.

- 2026-08-24 (session 4ce): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the Copilot singleton branch. Added the explicit
  `copilot` resolved-token seeding seam with exact `gh_cli` versus
  `env:<VAR>` source classification, all-source and per-source suppression
  gates before upsert, exchanged-token persistence, enterprise/default endpoint
  selection, source labels, and missing-input fail-open behavior. Added five
  source-derived `mock` parity tests first; the focused credential-pool wave
  now has 53 pool plus 15 persistence tests (68 total). The higher auth layer
  remains responsible for `gh auth token` resolution and network token
  exchange. Targeted rustfmt, focused tests, workspace build, serialized
  workspace test, and the approved 4/4-gate leaf recheck passed. Targeted
  `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Local source commit
  `19d0adb6efd8dbaf52d92c053d625a56993c2ea9` was mirrored as GitHub
  `33187eafa6a98fd0e7c22e582449e41bf102bb96`; both refs resolve to tree
  `00f4120fe2d977522030ba41a17ca571370629ca` with 273 matching tracked blobs.

- 2026-08-24 (session 4cd): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the Anthropic OAuth singleton branch. Added the
  explicit `anthropic` singleton seeding seam with provider opt-in gating,
  API-key-path pruning of stale `hermes_pkce`/`claude_code` rows, resolved
  credential-file access/refresh/expiry/label mapping, and per-source
  suppression. Added four source-derived `mock` parity tests first; the
  focused credential-pool wave now has 48 pool plus 15 persistence tests (63
  total). The Rust adapter takes the already-resolved provider/config and
  credential-file results as explicit inputs because this crate is below the
  CLI/provider layers. Targeted rustfmt, focused tests, workspace build,
  serialized workspace test, and the approved 4/4-gate leaf recheck passed.
  Targeted `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Local source commit
  `4659f291b9bdc3637d3ebe5a2109f2cb8ee33f65` was mirrored as GitHub
  `502491d1a790c369613d717b64620c98c82b94fa`; both refs resolve to tree
  `e60b32164a164b88f010aede28e80c2bd1d5edbf` with 273 matching tracked blobs.

- 2026-08-24 (session 4cc): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the xAI OAuth singleton branch. Added the explicit
  `xai-oauth` singleton seeding seam with device-code suppression, nested
  access/refresh token extraction, fixed `https://api.x.ai/v1` endpoint,
  `last_refresh` propagation, token-derived labels, and absent-token fail-open
  behavior. Added three source-derived `mock` parity tests first; the focused
  credential-pool wave now has 44 pool plus 15 persistence tests. Dedicated
  upstream xAI pool-seeding tests cover materialization and suppression; the
  Rust tests additionally pin label fallback and missing-token behavior.
  Targeted rustfmt, focused tests, workspace build, serialized workspace test,
  and the approved leaf-gate recheck passed. Targeted `hermes-agent` Clippy
  still reports only the two pre-existing `auxiliary_client` lints. Remaining
  singleton branches, full loader/config composition, Z.AI probing, OAuth
  refresh, leases, and logging throttles remain pending. Local source commit
  `3d6ae211cdddfd859c5d86487b5811c8b8d9afd3` was mirrored as GitHub
  `f6bc402553af0c828fca84fc382e060dadef326a`; both refs resolve to tree
  `156891e38d39a8aa21bde13a133a4202a4672e91` with 273 matching tracked blobs.

- 2026-08-24 (session 4cb): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the OpenAI Codex singleton branch. Added the
  explicit `openai-codex` singleton seeding seam with device-code suppression,
  nested access/refresh token extraction, fixed Codex backend URL,
  `last_refresh` propagation, custom-or-token-derived labels, and absent-token
  fail-open behavior. Added three source-derived `mock` parity tests first;
  the focused credential-pool wave now has 41 pool plus 15 persistence tests.
  Existing upstream Codex auth-provider tests establish the nested auth-store
  shape, but there is no direct `_seed_from_singletons` test; the branch code
  remains the oracle for that seam. Targeted rustfmt, focused tests, workspace
  build, serialized workspace test, and the approved leaf-gate recheck passed.
  Targeted `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Remaining singleton branches, full loader/config
  composition, Z.AI probing, OAuth refresh, leases, and logging throttles
  remain pending. Local source commit
  `aedda102d13da1515aa0ea7724702514a7d6a63d` was mirrored as GitHub
  `970720d0aba3a104423fb6b06b141e326d25854e`; both refs resolve to tree
  `c692cc51cfc473dcbb288ad0a5fad7dc097561a7` with 273 matching tracked blobs.

- 2026-08-24 (session 4ca): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the MiniMax OAuth singleton branch. Added the
  explicit `minimax-oauth` singleton seeding seam with fixed `oauth` source,
  suppression, access/refresh token propagation, ISO `expires_at` conversion
  to pool milliseconds with fail-open parsing, trailing-slash-stripped
  inference base URL, and custom-or-token-derived labels. Added three
  source-derived `mock` parity tests first; the focused credential-pool wave
  now has 38 pool plus 15 persistence tests. The upstream branch has no
  dedicated singleton-seeding test, so its implementation is the code oracle
  and that gap is recorded here. Targeted rustfmt, focused tests, workspace
  build, serialized workspace test, and the approved leaf-gate recheck passed.
  Targeted `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Remaining singleton branches, full loader/config
  composition, Z.AI probing, OAuth refresh, leases, and logging throttles
  remain pending. Local source commit
  `5abcc34da3bca6ac682931743ce85023cef183eb` was mirrored as GitHub
  `f9f4c457df15052546da0cf498f847e4d7d53e26`; both refs resolve to tree
  `f2de2de704c4ba7d61a57b230f0c6770ee303ed3` with 273 matching tracked blobs.

- 2026-08-24 (session 4c9): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the Qwen OAuth singleton branch. Extended the
  explicit singleton seeding seam with Qwen CLI runtime credentials, preserving
  source/auth type, access token, expiry milliseconds, base URL, auth-file label,
  suppression, and absent-token fail-open behavior. Added two source-derived
  `mock` parity tests first; the focused credential-pool wave now has 35 pool
  plus 15 persistence tests. Targeted rustfmt, focused tests, workspace build,
  serialized workspace test, and the approved leaf-gate recheck passed.
  Targeted `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. Remaining singleton branches, full loader/config
  composition, Z.AI probing, OAuth refresh, leases, and logging throttles
  remain pending. Local source commit
  `c471092747601784ee50b4d9503b6877c379bb25` was mirrored as GitHub
  `e4bd1f2199e3290a5adf381888c06f1bd0f3337d`; both refs resolve to tree
  `115b270c7e7bfeae69ca6c628fbc1ee844c2e22f` with 273 matching tracked blobs.

- 2026-08-24 (session 4c8): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the provider-singleton seeding boundary. Added the
  explicit `seed_from_singletons` seam for the upstream Nous branch: device-code
  source suppression, stale device-code removal when singleton state has no
  runtime material, invoke-JWT agent-key/runtime selection, custom labels, and
  direct/extra metadata preservation for access/refresh expiry, obtained-at,
  agent-key, endpoint, scope, and TLS fields. Added two source-derived `mock`
  parity tests first; the focused credential-pool wave now has 33 pool plus 15
  persistence tests. Targeted rustfmt, focused tests, workspace build, the
  clean serialized workspace test, and the approved leaf-gate recheck passed.
  Targeted `hermes-agent` Clippy still reports only the two pre-existing
  `auxiliary_client` lints. The remaining singleton branches, full loader/config
  composition, Z.AI probing, OAuth refresh, leases, and logging throttles
  remain pending. Local source commit
  `e0d804b3b851b49ccc7688ee5e044ccdef5e7f26` was mirrored as GitHub
  `a8e152495fe343c6f793c7e1980add0ccda466ce`; both refs resolve to tree
  `cca9c7bdb420b42dd7abb7bf7b443de3ce6da2da` with 273 matching tracked blobs.

- 2026-08-24 (session 4c7): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the lower environment-aware `load_pool` transaction.
  Added `load_pool_with_environment_at`, which mirrors the source's profile/
  global pool read, borrowed-secret and auth-type healing ownership rules,
  environment seeding, non-destructive env-row pruning, priority normalization,
  sorted borrowed-safe persistence, and configured strategy selection. Added
  two source-derived `mock` parity tests first; the focused credential-pool
  wave now has 31 pool plus 15 persistence tests. Targeted rustfmt, the focused
  suite, workspace build, serialized workspace test, and the approved leaf-gate
  recheck passed. Targeted `hermes-agent` Clippy still reports only the two
  pre-existing `auxiliary_client` lints. Singleton/config/custom-provider
  composition, Z.AI endpoint probing, OAuth refresh, leases, and logging
  throttles remain pending. Local source commit
  `d4322dded66b9ef9340212116514f6db63ee565a` was mirrored as GitHub
  `038a61c9f78b34426c07dcdf487df5fbd86ba808`; both refs resolve to tree
  `d3765b69460cc5c11e066f1e2268cb9b2354ec46` with 273 matching tracked blobs.

- 2026-08-24 (session 4c6): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the environment-seeding boundary. Added an
  explicit bottom-up `ProviderCredentialConfig` and `EnvironmentSnapshot`
  seam, source-compatible `.env` parsing (BOM/UTF-8-lossy, `export`, quoted
  values, first-`=` preservation, missing-file fail-open), dotenv-over-process
  precedence with unresolved `op://` secret-scope substitution, source
  suppression metadata, generic API-key and OpenRouter seeding, Kimi key-prefix
  routing, stale seeded-row pruning, and Anthropic `sk-ant-oat` auth typing.
  Added eight source-derived `unit` parity tests first; the focused
  credential-pool wave now has 29 pool plus 15 persistence tests. The
  required targeted rustfmt, focused suite, workspace build, and serialized
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --
  --test-threads=1` passed. Targeted `hermes-agent` Clippy still reports only
  the two pre-existing `auxiliary_client` lints. Full `load_pool` composition,
  provider singleton/config discovery, Z.AI endpoint probing, OAuth refresh,
  leases, and logging throttles remain pending. Local source commit
  `608f5d409848a35b9e10c8971269cbea662d7a74` was mirrored as GitHub
  `628c0ee13a14ed5a77e59e88da6accfb083e4ff9`; both refs resolve to tree
  `84981e3d9e9f964dfcfc31e8f9bb8f3b755747b5` with 273 matching tracked blobs.

- 2026-08-24 (session 4c5): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the auth-store advisory-lock boundary. Added
  platform-native exclusive `.lock` files with the source's 15-second minimum
  timeout, same-thread reentrancy, independent per-path holders for profile
  and global stores, and transaction coverage around credential-pool
  read-modify-write persistence. Added two source-derived `unit` parity tests
  first; the focused credential-pool wave now has 21 pool plus 15 persistence
  tests. Targeted rustfmt and the focused suite passed. The required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and serialized
  `/home/mustbearnold/.cargo/bin/cargo test --workspace -- --test-threads=1`
  also passed. Environment/config discovery, provider seeding, OAuth refresh,
  leases, logging throttles, and cross-process orchestration beyond the
  auth-store lock remain pending. Local source commit
  `197c14819ebc37739d7a501aa1d94a2133ec4d32` was mirrored as GitHub
  `b98265b02c4b65fdb7aae8ace265a5cd5d925efc`; both refs resolve to tree
  `a7498bc6a0ba7d9f8f2b6e816664b70dcfc9ac43` with 273 matching tracked blobs.

- 2026-08-24 (session 4c4): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through cooldown-recency persistence merging. The Rust
  writer now preserves a newer live `EXHAUSTED` cooldown or `DEAD` quarantine
  from disk for the same credential token, rejects stale state after re-auth,
  and does not resurrect expired cooldowns; timestamp parsing mirrors the
  source's seconds/milliseconds/ISO fail-open path and explicit reset-time
  precedence. Added four source-derived `unit` parity tests first, bringing
  the focused credential-pool wave to 21 pool plus 13 persistence tests. The
  required targeted parity suite, workspace build, and serialized workspace
  test passed; the approved lifecycle leaf reverified 4/4 gates. Local source
  commit `fd7e26d07e1efbab67885f56ca8d9eae2ce9b4a9` was mirrored as GitHub
  `e022d323070e2672017e26e71fb9c24678412b4d`; both refs resolve to tree
  `11def3079a8a63a329b59a963907afec98f041a0` with 273 matching tracked
  blobs. Auth-store locks, environment/config discovery, provider seeding,
  OAuth refresh, leases, logging throttles, and cross-process orchestration
  remain pending.

- 2026-08-24 (session 4c3): Continued the partial `agent.credential_pool`
  port (@ b9aa928) through the auth-store persistence boundary. Added
  `hermes-agent::credential_store`, which mirrors versioned empty-store
  defaults, accepted pool/provider schema, legacy `systems` migration, stale
  Nous Portal URL migration, malformed-file quarantine versus existing-file
  read-error propagation, atomic owner-only auth writes, per-provider
  profile/global fallback reads, and the final borrowed-secret disk
  sanitizer with intentional removal handling. Added 9 source-derived
  `unit`/`mock` parity tests first (`parity_credential_store.rs`), bringing
  the credential-pool wave to 30 focused tests. The approved credential
  lifecycle leaf gates passed; `/home/mustbearnold/.cargo/bin/cargo build
  --workspace` and serialized `/home/mustbearnold/.cargo/bin/cargo test
  --workspace -- --test-threads=1` passed. Full-workspace formatting remains
  blocked by pre-existing unformatted foundation files outside this wave;
  targeted `hermes-agent` Clippy still reports only the pre-existing
  `auxiliary_client` `too_many_arguments` and `needless_lifetimes` lints.
  Local source commit `43b4baf` was mirrored as GitHub
  `72976e0748ed6c1b708cc35465e463594806c6f1`; both refs resolve to tree
  `8b37b3b341423388e68c275c0f1e8d4467c43f61` with 273 matching tracked
  blobs.
  The follow-on cooldown-recency merge is now covered by four additional
  source-derived `unit` parity tests: re-auth token changes never resurrect
  stale status, newer live cooldowns and dead quarantines are adopted for the
  same token, and expired cooldowns are not restored. Auth-store locks,
  environment/config discovery, provider seeding, OAuth refresh, leases,
  logging throttles, and cross-process orchestration remain pending.

- 2026-08-24 (session 4bf): Continued the partial
  `agent.auxiliary_client` port (@ b9aa928, 10,044 LOC) with concrete
  keepalive client construction. The Rust `build_auxiliary_http_client`
  selects blocking or async reqwest clients, preserves explicit proxy routing
  while disabling ambient proxy lookup, applies connect and idle-pool settings,
  forwards insecure TLS, and loads an explicit PEM CA bundle as the sole
  certificate root set. Any concrete transport/build or unusable explicit
  bundle error fails open to `None`; the existing resolver still falls back to
  default roots for missing CA paths as upstream does. Added 4 source-derived
  `unit` parity tests first (41 total in `parity_auxiliary_client.rs`). Reqwest
  cannot express the source's total max-connection, write-timeout, and pool
  acquisition limits through its public builder, so those values remain in the
  transport-neutral config for the future lower-level adapter. The full SDK
  request/response path, credential-pool persistence/refresh, cancellation,
  and provider fallback chains remain pending. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` both passed; the
  local source commit `56871db` was mirrored as GitHub `7984aaa` with verified
  tree `cabc30ddb12626e265c4a2ec186c97e06b686815` and 270 matching tracked
  blobs. Handoff checkpoints local `5762c3c` → GitHub `2cd0262` and local
  `743fbcf` → GitHub `539b7c0` were mirrored immediately afterward; final
  `main`/`origin/main` resolve to the latter tree
  `74f37c6dca740a36866a6ca975ebb215ac2a85a8` with 270 matching blobs.

- 2026-08-24 (session 4be): Continued the missing
  `agent.credential_pool` port (@ b9aa928, 3,147 LOC) with the deterministic
  in-memory selection/rotation core. The Rust `CredentialPool` preserves
  priority fill-first selection, least-used request counting, round-robin
  priority rotation, explicit reset timestamp precedence, status-dependent
  cooldowns including the sole-credential transient exception, terminal OAuth
  `DEAD` transitions, failed-key identity matching, duplicate-key quarantine,
  and unmatched-identity fail-open rotation. Persistence, auth-store and env
  seeding, serialization, OAuth refresh, lease locking, random selection,
  logging throttles, and cross-process locking remain higher-layer seams.
  Added 8 source-derived `unit` parity tests first; the focused pool suite and
  the existing 37 auxiliary-client tests passed. The next auxiliary seam is
  concrete SDK/network client construction and pool persistence/refresh.
  Required `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` both passed; local
  source commit `6fcc72f` was mirrored as GitHub `1d46bab`, with verified tree
  `b69eb818bc34145186f7432c8ebe8910e3f461da` and 270 matching tracked blobs.

- 2026-08-24 (session 4c2): Added the root completion contract for the full
  conversion. `GATES.md` now requires generated-inventory closure, serialized
  workspace tests, formatting and documentation-hook validity, a clean
  generated snapshot, source-derived parity evidence, exact local/GitHub tree
  parity, and end-to-end surface review. The ignored `.unlazy/hermes-conversion`
  depth tree records foundation, agent-core, providers, CLI, integrations,
  surfaces, and root-integration branches; the active leaf is credential
  lifecycle auth-store persistence and provider/environment seeding. Current
  gates are intentionally unmet because the pinned inventory still reports 73
  done / 11 partial / 3,798 missing tracked modules and 73 done / 11 partial /
  1,019 missing production modules.

- 2026-08-24 (session 4c1): Continued the partial
  `agent.credential_pool` port (@ b9aa928) through the pure orchestration
  boundary after the row model. Rust now mirrors source-scoped `_upsert_entry`
  identity preservation, duplicate-source collapse, changed-key failure-state
  clearing, Anthropic seeded/manual priority normalization, configured
  fill-first/round-robin/least-used/random strategy parsing and random
  selection, custom provider name/endpoint pool-key lookup, non-empty custom
  pool listing, provider-boundary matching, and custom config lookup. Added 7
  source-derived `unit` parity tests first (21 total in
  `parity_credential_pool.rs`). The required workspace build, default test,
  and serialized workspace test all passed; workspace Clippy was killed by the
  environment (exit 137), while targeted `hermes-agent` Clippy reports only
  pre-existing auxiliary-client lint failures. Auth-store I/O, environment and
  config loading, provider seeding, OAuth refresh, leases, logging throttles,
  and cross-process locking remain pending.

- 2026-08-24 (session 4c0): Continued the partial
  `agent.credential_pool` port (@ b9aa928, 3,147 LOC) with the
  `PooledCredential` model boundary. The Rust row now mirrors the source's
  optional OAuth/provider metadata, `_EXTRA_KEYS` JSON round-trip, missing-row
  defaults, ISO status-timestamp rehydration, persisted `last_status` view,
  Anthropic `sk-ant-oat` OAuth normalization, token-derived labels, provider
  runtime base URLs, and Nous invoke-JWT scope/expiry filtering. Its
  `to_dict`/`to_json` path also mirrors the borrowed-source sanitizer: raw
  access/refresh/agent/secret fields are removed and replaced by a short
  SHA-256 fingerprint, while manual and provider-owned device-code state stays
  persistable. Added 6 source-derived `unit` parity tests first (14 total in
  `parity_credential_pool.rs`); the focused suite and workspace build passed.
  The default parallel workspace test command reproduced the existing
  process-global `hermes-tools::parity_credential_files` race twice; its
  exact isolated test and serialized `/home/mustbearnold/.cargo/bin/cargo test
  --workspace -- --test-threads=1` both passed. Only the three intentional
  delegation/schema doc tests remain ignored. Auth-store orchestration,
  environment/config seeding, OAuth refresh, leases, and cross-process pool
  locking remain the next credential-pool seams.

- 2026-08-24 (session 4bd): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with the
  transport and broader pool-selection boundary. The Rust
  `AuxiliaryHttpClientConfig` mirrors `build_keepalive_http_client`'s sync /
  async switch, 20/100 connection limits, 20-second idle expiry,
  connect/read/write/pool timeout contract, env-only proxy selection, and
  explicit no-proxy scheme mounts. `openai_client_config_with_transport`
  preserves injected-client and explicit-client precedence alongside the
  `max_retries=0` default. `select_auxiliary_pool_entry` preserves the
  source's distinction between no pool, present-but-unselectable pool, and a
  selected entry; `resolve_pool_first_runtime_credentials` adds the pool-first
  Nous/xAI runtime fallback projection. Added 5 source-derived `unit` parity
  tests first (37 total in `parity_auxiliary_client.rs`); the focused suite
  passed. These are transport-neutral and injected-input adapters: concrete
  SDK/network construction, pool persistence/rotation/refresh, cancellation,
  and provider fallback chains remain higher-layer seams. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` both passed; the
  workspace test run reports only the three intentional delegation/schema doc
  tests as ignored. Local source commit `895fbcf` was mirrored as GitHub
  `ead6b5f`; both refs now resolve to the verified tree
  `d9720eddbe0198216912d7c3de6c8fb3693a45b1` with all 268 tracked blobs
  matching by path, mode, and SHA.

- 2026-08-24 (session 4bc): Added the tracked documentation/GitHub metadata
  hook workflow. `.githooks/pre-commit` refreshes `tools/inventory.json`,
  `CONVERSION-LEDGER.md`, and the README status snapshot for source/parity/
  tooling changes (including manually staged generated outputs), stages
  generated outputs, and requires `PLAN.md` plus `HANDOFF.md` in the same index.
  `.githooks/post-commit` synchronizes the
  reviewed `.github/repository-description.txt` through the GitHub API and
  verifies the remote README without creating a second README commit; explicit
  Contents-API README writes remain opt-in. `tools/install_hooks.sh` configures
  the tracked hooks, while `tools/pre_commit_docs.py`, `tools/refresh_docs.py`,
  and `tools/sync_github_metadata.py` keep the workflow reviewable. No ledger
  change: 73 done / 10 partial / 3,799 missing tracked modules and 73 done /
  10 partial / 1,020 missing production modules. Validation: `bash -n
  .githooks/pre-commit .githooks/post-commit tools/install_hooks.sh`, Python
  syntax compilation for all three helper scripts, the README/inventory refresh
  command, the no-token metadata smoke check, `.githooks/pre-commit`,
  `git diff --check`, `/home/mustbearnold/.cargo/bin/cargo build --workspace`,
  and the final `/home/mustbearnold/.cargo/bin/cargo test --workspace` retry
  all passed. The first parallel workspace test run reproduced the known
  process-global credential-file race once; its exact targeted test and the
  complete retry passed. The next conversion unit remains concrete
  `agent.auxiliary_client` SDK/httpx construction and broader credential-pool
  selection.

- 2026-08-24 (session 4bb): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with Codex
  access-token selection. The Rust `read_codex_access_token` helper mirrors
  pool-first runtime-key selection, trimmed Hermes auth-store fallback,
  decoded JWT `exp` filtering with the source's strict comparison, and
  fail-open use of malformed/non-JWT tokens. Pool selection and
  `_read_codex_tokens()` are explicit Rust inputs; actual auth-file locking,
  credential-pool rotation, and SDK construction remain higher-layer seams.
  Added 4 source-derived `unit` parity tests first (32 total in
  `parity_auxiliary_client.rs`); the focused suite passed. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and the complete
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` both passed; only the
  three intentional delegation/schema doc tests remain ignored. Inventory and
  conversion ledger remain at 73 done / 10 partial / 3,799 missing tracked
  modules and 73 done / 10 partial / 1,020 missing production modules. The
  next seam is concrete SDK/httpx client construction and broader
  credential-pool selection.

- 2026-08-24 (session 4ba): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with the Codex
  OAuth/Cloudflare credential-header seam. The Rust `codex_cloudflare_headers`
  helper preserves the fixed `codex_cli_rs` originator and User-Agent, extracts
  `https://api.openai.com/auth.chatgpt_account_id` from a URL-safe JWT payload,
  uses the exact `ChatGPT-Account-ID` casing, and fails open by retaining only
  fixed headers for empty, malformed, or claim-less tokens. Added 3
  source-derived `unit` parity tests first (28 total in
  `parity_auxiliary_client.rs`); the focused suite passed. Added the direct
  `base64` dependency already present in the workspace lockfile. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and the complete
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` both passed; only the
  three intentional delegation/schema doc tests remain ignored. Inventory and
  conversion ledger remain at 73 done / 10 partial / 3,799 missing tracked
  modules and 73 done / 10 partial / 1,020 missing production modules. The
  next seam is concrete SDK/httpx client construction and credential-pool
  selection.

- 2026-08-24 (session 4b9): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with the
  transport-independent proxy/TLS policy boundary. The Rust adapter mirrors
  `process_bootstrap.py` proxy precedence (`HTTPS_PROXY`, `HTTP_PROXY`,
  `ALL_PROXY`, then lowercase forms), SOCKS URL normalization,
  `NO_PROXY` suffix matching, and fail-open behavior for absent or malformed
  base URLs. Its `AuxiliaryTlsVerify` value mirrors `ssl_verify.py`'s insecure
  setting precedence, explicit/provider and `HERMES_CA_BUNDLE` →
  `SSL_CERT_FILE` → `REQUESTS_CA_BUNDLE` → `CURL_CA_BUNDLE` lookup order,
  user expansion, existing-file check, and default-certificate fallback.
  Python's httpx client and SSL context are intentionally represented as a
  transport-neutral value until the SDK/transport seam is ported. Added 6
  source-derived `unit` parity tests first (25 total in
  `parity_auxiliary_client.rs`). The focused suite, required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and the complete
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` all passed; only the
  three intentional delegation/schema doc tests remain ignored. Inventory and
  conversion ledger remain at 73 done / 10 partial / 3,799 missing tracked
  modules and 73 done / 10 partial / 1,020 missing production modules. The
  next seam is concrete SDK/httpx client construction and credential selection.

- 2026-08-24 (session 4b8): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with the
  transport-independent OpenAI client-option boundary. The Rust config keeps
  the source's API key and base URL inputs and defaults `max_retries` to zero,
  while preserving an explicit retry override so Hermes remains the owner of
  retry/fallback policy. Actual OpenAI SDK/httpx construction, env-only proxy,
  TLS verification, and async transport remain future seams. Added 2
  source-derived `unit` parity tests (19 total in
  `parity_auxiliary_client.rs`) first; the focused suite passed. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` passed. The first
  full workspace test exposed the existing parallel credential-files race;
  its isolated rerun and the second full workspace test both passed. Only the
  three intentional delegation/schema doc tests are ignored. Inventory and
  conversion ledger remain at 73 done / 10 partial / 3,799 missing tracked
  modules and 73 done / 10 partial / 1,020 missing production modules. The
  next seam is SDK/httpx client construction and explicit credential selection.

- 2026-08-24 (session 4b7): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with OpenAI-
  compatible endpoint normalization and the Anthropic host-validation guard.
  The Rust adapter mirrors the source's trimmed `/anthropic` rewrite to `/v1`,
  Z.AI `bigmodel` rewrite to `/paas/v4`, Kimi Coding `/coding` rewrite to
  `/coding/v1`, unchanged endpoint normalization, and exact
  `api.anthropic.com` acceptance including case/trailing-dot/protocol-relative
  URL forms while failing closed for foreign hosts and malformed/bare values.
  Added 2 source-derived `unit` parity tests (17 total in
  `parity_auxiliary_client.rs`) first; the focused suite passed. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` passed. The first
  full workspace test exposed an existing daemon-pool scheduling assertion;
  its isolated rerun and the second full workspace test both passed. Only the
  three intentional delegation/schema doc tests are ignored. Inventory and
  conversion ledger remain at 73 done / 10 partial / 3,799 missing tracked
  modules and 73 done / 10 partial / 1,020 missing production modules. The
  next seam is SDK client construction and explicit credential selection.

- 2026-08-24 (session 4b6): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with the
  credential-safe pool-runtime projection. The Rust adapter mirrors the
  source's projected `runtime_api_key` → `access_token` fallback, URL
  precedence (`runtime_base_url` → `inference_base_url` → `base_url` →
  fallback), whitespace/trailing-slash normalization, and Nous-only
  `NOUS_INFERENCE_BASE_URL` override. Pool JWT validation, secret lookup, and
  actual SDK/client construction remain explicit auth/transport seams. Added
  3 source-derived `unit` parity tests (15 total in
  `parity_auxiliary_client.rs`) first; the focused suite passed. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green; only the
  three intentional delegation/schema doc tests are ignored. Inventory and
  conversion ledger remain at 73 done / 10 partial / 3,799 missing tracked
  modules and 73 done / 10 partial / 1,020 missing production modules. The
  next seam is OpenAI-compatible client construction and credential selection.

- 2026-08-24 (session 4b5): Continued the partial
  `agent/auxiliary_client.py` port (@ b9aa928, 10,044 LOC) with the pure
  `_resolve_task_provider_model` routing seam. The Rust resolver mirrors
  explicit-over-config precedence, matching task endpoint/key adoption,
  first-class provider identity with an explicit base URL, direct
  `openai` → `custom` expansion, MoA aggregator unwrapping with virtual
  credential removal, unresolved-MoA fail-through, and explicit/configured
  `model: auto` normalization. Config maps, MoA preset results, and provider
  registry membership are explicit Rust adapter inputs; secret-scope/key-env
  lookup and client construction remain pending higher-layer seams. Added 7
  source-derived `unit` parity tests (12 total in `parity_auxiliary_client.rs`)
  first; the focused suite passed. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` passed. The first
  full workspace test exposed a pre-existing parallel credential-files test
  race; its isolated rerun and the second full workspace test both passed.
  Only the three intentional delegation/schema doc tests are ignored. The
  inventory and conversion ledger remain at 73 done / 10 partial / 3,799
  missing tracked modules and 73 done / 10 partial / 1,020 missing production
  modules. The next seam is auxiliary client construction and credential
  resolution before transport/fallback lifecycle work.

- 2026-08-24 (session 4b4): Opened the `hermes-agent` crate and ported the
  dependency-safe predicate/wire-parameter section of
  `agent/auxiliary_client.py` (@ b9aa928, 10,044 LOC). The Rust
  `hermes_agent::auxiliary_client` module mirrors provider alias normalization
  including `custom:`, `codex`, and `main` special forms; the explicit
  OpenAI-compatible `max_tokens` versus `max_completion_tokens` selection
  based on endpoint host, OpenRouter/Nous credential presence, and model
  family; and the source's payment/quota, rate-limit, stale-model, and
  model-capability error predicates. The Python module's hidden config and
  exception introspection are explicit Rust adapter arguments
  (`main_provider`, credential-presence booleans, and `AuxiliaryError`).
  Added 5 `unit` parity tests first. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green; only the
  three intentional delegation/schema doc tests are ignored. The module is
  intentionally `partial`: client construction, credential pools, async
  transport, cancellation/progress, provider fallback chains, and the
  remaining 10k-line call path are pending. Inventory and conversion ledger
  now record 73 done / 10 partial / 3,799 missing tracked modules and
  73 done / 10 partial / 1,020 missing production modules. The next
  dependency-safe production unit is `run_agent` (8,206 LOC).

- 2026-08-24 (session 4b3): Ported
  `plugins/model-providers/zai/__init__.py` (@ b9aa928, 127 LOC) through a
  source-derived TDD pass against
  `tests/plugins/model_providers/test_zai_profile.py`,
  `tests/providers/test_profile_wiring.py`, and
  `tests/providers/test_transport_parity.py`. Added the Z.AI/GLM profile with
  its aliases, ordered API-key environment variables, Z.AI endpoint, fallback
  models, and GLM-4.5 Flash auxiliary model. Added the `zai_reasoning`
  capability: the source's GLM version predicate enables or disables the
  `thinking` body for GLM 4.5+, while GLM-5.2 token aliases also emit the
  top-level `reasoning_effort`; low/medium/minimal/high map to `high`,
  xhigh/max/ultra map to `max`, and empty/none/disabled values omit the
  effort field. Added 5 unit parity tests, sorted bundled/user loading, and
  registry expectations. Focused ZAI/base/registry regressions,
  `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green; only the
  three intentional delegation/schema doc tests are ignored. Inventory and
  conversion ledger now record 73 done / 9 partial / 3,800 missing tracked
  modules and 73 done / 9 partial / 1,021 missing production modules. The
  next dependency-safe production unit is `agent.auxiliary_client`
  (10,044 LOC).

- 2026-08-24 (session 4b2): Ported
  `plugins/model-providers/kimi-coding/__init__.py` (@ b9aa928, 121 LOC) through
  a source-derived TDD pass against
  `tests/plugins/model_providers/test_kimi_profile.py`,
  `tests/providers/test_profile_wiring.py`, and
  `tests/providers/test_transport_parity.py`. Added the Kimi Coding and China
  profiles with their aliases, ordered environment variables, Moonshot
  endpoints, omitted temperature, 32,000 max-token cap, hermes-agent header,
  and auxiliary model. Added the kimi_coding capability: only the exact
  HTTPS api.kimi.com /coding or /coding/v1 endpoint is confirmed, /coding
  receives the source's /v1 normalization, unconfirmed catalogs filter
  whitespace/case-insensitive k3 IDs, and probe failures remain fail-open.
  The reasoning hook emits thinking enabled/disabled or a top-level
  low/medium/high reasoning_effort, never both. Added 3 unit/mock parity tests,
  sorted bundled/user loading, and registry expectations. Focused
  Kimi/base/registry regressions,
  `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green.
  Inventory and conversion ledger now record 72 done / 9 partial / 3,801
  missing tracked modules and 72 done / 9 partial / 1,022 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.zai.__init__` (127 LOC).

- 2026-08-24 (session 4b1): Ported
  `plugins/model-providers/upstage/__init__.py` (@ b9aa928, 115 LOC) through
  a source-derived TDD pass against
  `tests/plugins/model_providers/test_upstage_profile.py`. Added the explicit
  `upstage_reasoning` capability for Solar's top-level
  `reasoning_effort`: `solar-mini` and `syn-pro` substring families are
  deny-listed, unset/empty effort defaults to `medium`, low/medium/high pass
  through, `minimal` omits the field, xhigh/max/ultra and unknown efforts
  clamp to `high`, and explicit `enabled=False` omits the field. Added the
  Upstage profile metadata, `solar` alias, ordered API-key/base-URL env vars,
  `solar-pro3` fallback, sorted bundled/user loading, and 4 `unit` parity
  tests. Focused Upstage/base/registry regressions,
  `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 71 done / 9 partial / 3,802
  missing tracked modules and 71 done / 9 partial / 1,023 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.kimi-coding.__init__` (121 LOC).

- 2026-08-24 (session 4b0): Ported
  `plugins/model-providers/qwen-oauth/__init__.py` (@ b9aa928, 108 LOC) through
  a source-derived TDD pass against the Qwen provider-profile, profile-wiring,
  and chat-completions transport cases. Added the explicit `qwen_portal`
  capability for message normalization: string and mixed content parts become
  text dictionaries, unsupported parts are filtered when normalized content is
  non-empty, nested `image_url` objects are copied for retry safety, and the
  last part of the first system message receives
  `cache_control={"type":"ephemeral"}`. The profile always emits
  `vl_high_resolution_images=true` in `extra_body` and keeps non-empty
  `qwen_session_metadata` at top-level `metadata`, never in `extra_body`.
  Because the Rust adapter owns `serde_json::Value` trees, it clones the
  complete message value rather than selectively sharing immutable parts;
  this is the ownership-equivalent of the source's nested-image retry guard.
  Added the `qwen-oauth` profile, its three aliases, OAuth metadata, and
  65,536 default max-token cap; wired sorted bundled/user loading and added 4
  `unit` parity tests. Focused Qwen/base/registry regressions,
  `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 70 done / 9 partial / 3,803
  missing tracked modules and 70 done / 9 partial / 1,024 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.upstage.__init__` (115 LOC).

- 2026-08-24 (session 4af): Ported
  `plugins/model-providers/custom/__init__.py` (@ b9aa928, 103 LOC) through
  a source-derived TDD pass against
  `tests/plugins/model_providers/test_custom_profile.py`. The profile mirrors
  the canonical `custom` name, six Ollama/local/OpenAI-compatible aliases,
  empty environment/base URL fields, and the `65536` default max-token cap.
  Added the explicit `custom_provider` capability for the shared
  `CustomProfile.build_api_kwargs_extras` and `fetch_models` overrides:
  truthy `ollama_num_ctx` maps to `extra_body.options.num_ctx`, disabled or
  `effort=none` emits top-level `reasoning_effort=none` plus
  `extra_body.think=false`, enabled efforts are trimmed/lowercased and
  passed through top-level, empty/unset configs omit reasoning, and the
  configured-base guard fails open before catalog probing. Added 4 `unit`
  parity tests, wired the profile in sorted bundled/user loader order, and
  updated registry/base expectations. Focused Custom/base/registry
  regressions, `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 69 done / 9 partial / 3,804
  missing tracked modules and 69 done / 9 partial / 1,025 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.qwen-oauth.__init__` (108 LOC).

- 2026-08-24 (session 4ae): Ported
  `plugins/model-providers/minimax/__init__.py` (@ b9aa928, 97 LOC) through
  a source-derived TDD pass against
  `tests/plugins/model_providers/test_minimax_profile.py`. The three
  statically registered profiles mirror the direct `minimax`, `minimax-cn`,
  and `minimax-oauth` names, aliases, API modes, credentials, endpoints,
  OAuth metadata, and auxiliary defaults (`MiniMax-M3` for direct/China and
  `MiniMax-M2.7` for OAuth). Added the explicit `minimax_reasoning`
  capability for the shared `MiniMaxProfile.build_api_kwargs_extras` hook:
  exact `api.minimax.io/v1` route and M3 model/slug gating, mandatory
  `reasoning_split`, adaptive thinking for any supplied config, disabled
  thinking for explicit `enabled=False`, and no thinking body when config is
  absent are preserved. Query-bearing `/v1` URLs remain accepted because the
  upstream predicate compares the parsed path. Auxiliary-client, OAuth
  runtime, and broader agent/transport integration remain future higher-layer
  seams. Added 3 `unit` parity tests, wired all three profiles in sorted
  bundled registration, and updated registry/base expectations. Focused
  Minimax/base/registry regressions,
  `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 69 done / 9 partial / 3,804
  missing tracked modules and 69 done / 9 partial / 1,025 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.custom.__init__` (103 LOC).

- 2026-08-24 (session 4ad): Ported
  `plugins/model-providers/ollama-cloud/__init__.py` (@ b9aa928, 89 LOC)
  through a source-derived TDD pass against
  `tests/plugins/model_providers/test_ollama_cloud_profile.py`. The profile
  mirrors the canonical `ollama-cloud` name, `ollama_cloud` alias,
  `OLLAMA_API_KEY`, `https://ollama.com/v1`, and
  `nemotron-3-nano:30b` auxiliary model. Added the explicit
  `ollama_cloud_reasoning` capability for the custom hook: native thinking
  capability gating, disabled/`none` top-level off switch,
  xhigh/max/ultra-to-max normalization, low/medium/high passthrough, blank
  and unknown-effort omission, and no extra-body reasoning field are
  preserved. The `/api/show` probe, dynamic live+models.dev catalog merge,
  and hermes-cli credential/model-picker integrations remain future seams.
  Added 3 `unit` parity tests, wired the profile in the bundled/user loader,
  and updated registry expectations. Focused Ollama Cloud/base/registry
  regressions, `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 67 done / 9 partial / 3,806
  missing tracked modules and 67 done / 9 partial / 1,027 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.minimax.__init__` (97 LOC).

- 2026-08-24 (session 4ac): Ported
  `plugins/model-providers/actual/__init__.py` (@ b9aa928, 89 LOC) through a
  source-derived TDD pass against `tests/hermes_cli/test_actual_provider.py`.
  The profile mirrors Actual Computer's canonical name, aliases, metadata,
  `ACTUAL_API_KEY`/`ACTUAL_BASE_URL` environment contract, hosted base URL,
  API-key auth, and `codex_responses` mode. Added the explicit
  `actual_catalog` capability for `ActualProfile.fetch_models`: environment
  precedence, hosted/local root `/v1` normalization, optional Bearer auth,
  JSON/Accept/User-Agent headers, list and `{data: [...]}` payloads, ID
  filtering, and fail-open error handling are preserved. Runtime credential
  resolution, model-picker integration, and the application transport/opener
  remain future hermes-cli seams. Added 3 `unit`/`mock` parity tests, wired
  Actual first in the static bundled/user loader order, and updated registry
  expectations. Focused Actual/base regressions plus
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 66 done / 9 partial / 3,807
  missing tracked modules and 66 done / 9 partial / 1,028 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.ollama-cloud.__init__` (89 LOC).

- 2026-08-24 (session 4ab): Ported
  `plugins/model-providers/nous/__init__.py` (@ b9aa928, 88 LOC) through a
  source-derived TDD pass against `tests/providers/test_provider_profiles.py`
  and the Nous chat-completions transport cases. The profile mirrors the
  canonical `nous` name, `nous-portal`/`nousresearch` aliases, Nous Research
  metadata, `NOUS_API_KEY`, inference base URL, OAuth device-code auth, and
  Hermes 3 fallback models. Added the explicit `nous_portal` capability for
  the `build_extra_body` and `build_api_kwargs_extras` overrides: product and
  pinned client tags, conversation/sticky routing with cron timestamp
  normalization, truthy provider preferences, reasoning default passthrough,
  and omission when disabled are preserved. The context map's
  `conversation_context` key is the current adapter for the upstream
  `ContextVar`; runtime CLI-version and higher-layer context propagation remain
  explicit future seams. Added 3 `unit` parity tests, wired the profile in
  sorted bundled/user loader order, and updated registry expectations. Focused
  provider suites, `/home/mustbearnold/.cargo/bin/cargo build --workspace`, and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 65 done / 9 partial / 3,808
  missing tracked modules and 65 done / 9 partial / 1,029 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.actual.__init__` (89 LOC).

- 2026-08-24 (session 4aa): Ported
  `plugins/model-providers/deepseek/__init__.py` (@ b9aa928, 102 LOC) through
  a source-derived TDD pass against the pinned provider wiring and transport
  cases. No dedicated plugin-profile test module exists, so the implementation
  and DeepSeek profile-path behavior are the oracles. The profile mirrors the
  canonical `deepseek` name, `deepseek-chat` alias, display/description/signup
  metadata, API-key environment variable, native base URL, V4 Pro/Flash
  fallbacks, and V4 Flash auxiliary model. Added the explicit
  `deepseek_reasoning` capability for `build_api_kwargs_extras`: V4+ model
  gating excludes V3/unknown models, thinking defaults enabled, disabled
  reasoning emits `thinking.type=disabled`, and low/medium/high pass through
  while xhigh/max/ultra clamp to top-level `reasoning_effort=max`; unsupported
  efforts omit the top-level field. Added 2 `unit` parity tests, wired the
  profile in sorted bundled/user loader order, and updated registry
  expectations. Required `/home/mustbearnold/.cargo/bin/cargo build
  --workspace` and `/home/mustbearnold/.cargo/bin/cargo test --workspace
  --quiet` are green. Inventory and conversion ledger now record 64 done / 9
  partial / 3,809 missing tracked modules and 64 done / 9 partial / 1,030
  missing production modules. The next dependency-safe production unit is
  `plugins.model-providers.nous.__init__` (88 LOC).

- 2026-08-24 (session 4z): Ported
  `plugins/model-providers/deepinfra/__init__.py` (@ b9aa928, 81 LOC) through
  a source-derived TDD pass against the DeepInfra profile/tag cases in
  `tests/hermes_cli/test_api_key_providers.py`. The profile mirrors the
  canonical `deepinfra` name, `deep-infra`/`deepinfra-ai` aliases, DeepInfra
  display/description/signup metadata, ordered API-key/base-url environment
  variables, OpenAI-compatible base URL, API-key auth, empty fallback list,
  and `deepseek-ai/DeepSeek-V4-Flash` auxiliary model. Added the explicit
  `deepinfra_vision` capability for the subclass `default_vision_model()` hook:
  a non-empty `DEEPINFRA_API_KEY` gates the tagged chat catalog probe, image
  surfaces are excluded, the first chat+vision model wins, and the raw catalog
  uses the upstream base-URL cache plus 60-second negative cache. The probe
  sends the Bearer header and fails open on missing/malformed/unreachable
  catalogs. Profile-scoped `get_secret` resolution and the installed CLI
  opener remain future higher-layer seams; single-profile environment behavior
  is preserved. Added 2 `unit`/`mock` parity tests, wired the profile in sorted
  bundled/user loader order, and updated registry expectations. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace --quiet` are green.
  Inventory and conversion ledger now record 63 done / 9 partial / 3,810
  missing tracked modules and 63 done / 9 partial / 1,031 missing production
  modules. The next dependency-safe production unit is
  `plugins.model-providers.deepseek.__init__` (102 LOC).

- 2026-08-24 (session 4y): Ported
  `plugins/model-providers/vertex/__init__.py` (@ b9aa928, 75 LOC) through a
  source-derived TDD pass. No dedicated upstream Vertex profile test module
  exists, so the pinned implementation plus Gemini transport helper behavior
  are the oracles. The profile mirrors the canonical `vertex` name,
  `google-vertex`/`vertex-ai`/`gcp-vertex` aliases, `chat_completions` routing,
  empty static environment-variable list, Vertex OAuth auth type, Google
  Vertex base URL, and `google/gemini-3.6-flash` auxiliary model. Added the
  explicit `vertex_thinking` capability: Gemini reasoning is translated to
  the nested snake_case `extra_body.google.thinking_config` shape, with empty
  and non-Gemini cases failing open exactly as the upstream helper does.
  `models_fetch_disabled` preserves the upstream unconditional `None` model
  discovery override before any endpoint access; runtime OAuth token
  resolution remains a future adapter seam. Added 3 `unit` parity tests,
  wired the profile in sorted bundled/user loader order, and updated registry
  expectations. The required workspace run first exposed a race among existing
  process-global platform-cache tests; serialized only those test resets in
  commit `b2c1f4f` (mirrored as GitHub `1682e21`) without changing production
  detection semantics. Required `/home/mustbearnold/.cargo/bin/cargo
  build --workspace` and `/home/mustbearnold/.cargo/bin/cargo test
  --workspace --quiet` are green. Inventory and conversion ledger now record
  62 done / 9 partial / 3,811 missing tracked modules and 62 done / 9 partial /
  1,032 missing production modules. The next dependency-safe production unit
  is `plugins.model-providers.deepinfra.__init__` (81 LOC).

- 2026-08-24 (session 4x): Ported
  `plugins/model-providers/copilot/__init__.py` (@ b9aa928, 74 LOC) through a
  source-derived TDD pass against the dedicated Copilot profile tests. The
  profile mirrors the canonical `copilot` name,
  `github-copilot`/`github-models`/`github-model`/`github` aliases, ordered
  `COPILOT_GITHUB_TOKEN`/`GH_TOKEN`/`GITHUB_TOKEN` variables, GitHub Copilot
  base URL, and `copilot` auth type. Added the explicit `copilot_reasoning`
  capability for the subclass `build_api_kwargs_extras` hook: supports-
  reasoning/model/catalog gating, live effort injection seam, xhigh→high and
  minimal→low downgrades, medium fallback, first-supported fallback, and the
  upstream no-config medium behavior are preserved; all missing catalog or
  capability cases fail open. Added 3 `unit` parity tests, wired the profile
  before `copilot-acp` in sorted bundled/user loader order, and updated registry
  expectations. Focused Copilot, base, and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green (3
  intentional delegation/schema doc tests ignored). Inventory and conversion
  ledger now record 61 done / 9 partial / 3,812 missing tracked modules and
  61 done / 9 partial / 1,033 missing production modules. The next
  dependency-safe production unit is
  `plugins.model-providers.vertex.__init__` (75 LOC).

- 2026-08-24 (session 4w): Ported
  `plugins/model-providers/gemini/__init__.py` (@ b9aa928, 61 LOC) through a
  source-derived TDD pass. The dedicated upstream profile test pins the
  auxiliary model; related transport tests pin the profile's thinking hook.
  The profile mirrors the canonical `gemini` name,
  `google`/`google-gemini`/`google-ai-studio` aliases,
  `chat_completions` routing, ordered `GOOGLE_API_KEY`/`GEMINI_API_KEY`
  variables, Google AI Studio base URL, `api_key` auth, and
  `gemini-3.6-flash` auxiliary model. Added the explicit `gemini_thinking`
  capability and 3 `unit` parity tests covering Gemini model gating,
  `google/` normalization, disabled/none handling, Gemini 2.5 behavior,
  Gemini 3 Flash/Pro effort clamping, native camelCase output, and the exact
  Google OpenAI-compatible `/openai` nested snake_case output. Wired the
  profile in sorted bundled/user loader order and updated registry
  expectations. Focused Gemini, base, and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green (3
  intentional delegation/schema doc tests ignored). Inventory and conversion
  ledger now record 60 done / 9 partial / 3,813 missing tracked modules and
  60 done / 9 partial / 1,034 missing production modules. The next
  dependency-safe production unit is
  `plugins.model-providers.copilot.__init__` (74 LOC).

- 2026-08-24 (session 4v): Ported
  `plugins/model-providers/anthropic/__init__.py` (@ b9aa928, 54 LOC) through
  a source-derived TDD pass. No dedicated upstream profile test module exists,
  so the pinned implementation is the oracle. The profile mirrors the
  canonical `anthropic` name, `claude`/`claude-oauth`/`claude-code` aliases,
  `anthropic_messages` routing, ordered Anthropic API/OAuth environment
  variables, native API base URL, `api_key` auth, and the
  `claude-haiku-4-5-20251001` auxiliary model. Added an explicit
  `ModelsFetchMode::Anthropic` capability for the subclass `fetch_models()`:
  it requires a non-empty key, probes the fixed `/v1/models` endpoint with
  `x-api-key`, `anthropic-version: 2023-06-01`, and `Accept` headers, filters
  string IDs from the `data` array, and fails open on transport/JSON errors.
  The cloned-profile `models_url` override is a mock/integration seam; the
  production profile retains the upstream fixed endpoint and ignores caller
  `base_url`. Added 3 parity tests (`unit`/`mock`), wired the profile in sorted
  bundled/user loader order, and updated registry expectations. Focused
  Anthropic, base, and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green (3
  intentional delegation/schema doc tests ignored). Inventory and conversion
  ledger now record 59 done / 9 partial / 3,814 missing tracked modules and
  59 done / 9 partial / 1,035 missing production modules. The next
  dependency-safe production unit is
  `plugins/model-providers.gemini.__init__` (61 LOC).

- 2026-08-24 (session 4u): Ported
  `plugins/model-providers/fireworks/__init__.py` (@ b9aa928, 46 LOC) through
  a TDD pass against the dedicated upstream profile tests. The profile mirrors
  the canonical `fireworks` name, `fireworks-ai`/`fw` aliases, Fireworks AI
  display metadata and signup URL, `FIREWORKS_API_KEY`, the OpenAI-compatible
  endpoint, exact attribution headers, the `glm-5p2` auxiliary model, and all
  three ordered pay-as-you-go fallback model IDs. The dynamic upstream
  `hermes_cli.__version__` import remains an explicit future CLI seam. Added 2
  `unit` parity tests for identity, hostname, headers, aliases, auxiliary and
  fallback models, and canonical list identity; wired the profile in sorted
  bundled/user loader order. Focused Fireworks and registry suites are green.
  Required `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green (3
  intentional delegation/schema doc tests ignored). Inventory and conversion
  ledger now record 58 done / 9 partial / 3,815 missing tracked modules and
  58 done / 9 partial / 1,036 missing production modules. The next
  dependency-safe production unit is
  `plugins.model-providers.anthropic.__init__` (54 LOC).

- 2026-08-24 (session 4t): Ported
  `plugins/model-providers/ai-gateway/__init__.py` (@ b9aa928, 43 LOC) through
  a source-derived TDD pass. The upstream module has no dedicated profile test;
  its source and related routing tests are mirrored by the canonical
  `ai-gateway` name, `vercel`/`vercel-ai-gateway`/`ai_gateway`/`aigateway`
  aliases, `AI_GATEWAY_API_KEY`, Vercel base URL, attribution headers, and
  Gemini auxiliary model. Added the shared `reasoning_passthrough` capability
  so `build_api_kwargs_extras` copies a supplied reasoning config into
  `extra_body.reasoning`, defaults to enabled/medium, and suppresses the body
  when `supports_reasoning` is false. Added 2 `unit` parity tests for fields,
  aliases, hostname, headers, auxiliary model, reasoning behavior, and
  canonical list identity; wired the profile in sorted bundled/user loader
  order. Focused AI Gateway, base, and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green (3
  intentional delegation/schema doc tests ignored). Inventory and conversion
  ledger now record 57 done / 9 partial / 3,816 missing tracked modules and
  57 done / 9 partial / 1,037 missing production modules. The next
  dependency-safe production unit is
  `plugins.model-providers.fireworks.__init__` (46 LOC).

- 2026-08-24 (session 4s): Ported
  `plugins/model-providers/copilot-acp/__init__.py` (@ b9aa928, 35 LOC) through
  a source-derived TDD pass. The upstream module has no dedicated profile test;
  its source contract is mirrored by the canonical `copilot-acp` name,
  `github-copilot-acp`/`copilot-acp-agent` aliases, explicit
  `chat_completions` routing, empty environment-variable tuple,
  `acp://copilot` internal scheme, and `external_process` auth type. Its
  `CopilotACPProfile.fetch_models()` override is represented by the shared
  `models_fetch_disabled` capability, returning `None` before endpoint
  validation or network I/O. Added 2 `unit` parity tests for all declarative
  fields, aliases, external routing, fetch override, and canonical list
  identity; wired the profile in sorted bundled/user loader order. Focused
  Copilot ACP and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green (3
  intentional delegation/schema doc tests ignored). Inventory and conversion
  ledger now record 56 done / 9 partial / 3,817 missing tracked modules and
  56 done / 9 partial / 1,038 missing production modules. The next
  dependency-safe production unit is
  `plugins.model-providers.ai-gateway.__init__` (43 LOC).

- 2026-08-24 (session 4r): Ported
  `plugins/model-providers/gmi/__init__.py` (@ b9aa928, 32 LOC) through a
  source-derived TDD pass. The profile mirrors the canonical `gmi` name,
  `gmi-cloud`/`gmicloud` aliases, GMI Cloud display metadata and signup URL,
  ordered `GMI_API_KEY`/`GMI_BASE_URL` environment variables,
  `https://api.gmi-serving.com/v1`, explicit `api_key` auth, the
  `HermesAgent/0.20.0` attribution header, the Gemini auxiliary model, and all
  seven ordered fallback models. It is registered in sorted bundled-profile
  order between Bedrock and Hugging Face. The dynamic upstream
  `hermes_cli.__version__` import remains an explicit future CLI seam. Added 2
  `unit` parity tests for all declarative fields, aliases, hostname, header,
  auxiliary/fallback models, and canonical list identity. Focused GMI and
  registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green. The next
  dependency-safe production unit is
  `plugins.model-providers.copilot-acp.__init__` (35 LOC).

- 2026-08-24 (session 4q): Ported
  `plugins/model-providers/bedrock/__init__.py` (@ b9aa928, 30 LOC) through a
  source-derived TDD pass. The profile mirrors the canonical `bedrock` name,
  `aws`/`aws-bedrock`/`amazon-bedrock`/`amazon` aliases,
  `bedrock_converse` API mode, empty environment-variable tuple,
  `https://bedrock-runtime.us-east-1.amazonaws.com`, and `aws_sdk` auth. Its
  upstream `BedrockProfile.fetch_models()` subclass override is represented by
  the shared `models_fetch_disabled` capability, which returns `None` before
  REST endpoint validation or network I/O. Added 2 `unit` parity tests for all
  profile fields, aliases, hostname, canonical list identity, and the
  no-network fetch override. Focused Bedrock, base, and registry suites are
  green. Required `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green. The next
  dependency-safe production unit is `plugins.model-providers.gmi.__init__`
  (32 LOC).

- 2026-08-24 (session 4p): Ported
  `plugins/model-providers/novita/__init__.py` (@ b9aa928, 27 LOC) through a
  source-derived TDD pass. The profile mirrors the canonical `novita` name,
  `novita-ai`/`novitaai` aliases, NovitaAI display metadata and signup URL,
  ordered `NOVITA_API_KEY`/`NOVITA_BASE_URL` environment variables,
  `https://api.novita.ai/openai/v1`, explicit `api_key` auth,
  `deepseek/deepseek-v3-0324` as the auxiliary model, and all six ordered
  fallback models. It is registered in sorted bundled-profile order before
  NVIDIA. Upstream `tests/hermes_cli/test_api_key_providers.py` covers profile
  loading and the pricing-cache helper; the latter remains a future
  hermes-cli seam. Added 2 `unit` parity tests for all declarative fields,
  aliases, hostname, auxiliary/fallback models, and canonical list identity.
  Focused Novita and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green. The next
  dependency-safe production unit is `plugins.model-providers.bedrock.__init__`
  (30 LOC).

- 2026-08-24 (session 4o): Ported
  `plugins/model-providers/nvidia/__init__.py` (@ b9aa928, 21 LOC) through a
  source-derived TDD pass. The profile mirrors the canonical `nvidia` name,
  `nvidia-nim` alias, `NVIDIA_API_KEY` environment variable, NVIDIA NIM
  display metadata and signup URL, the two ordered fallback models,
  `https://integrate.api.nvidia.com/v1`, and the 16,384 default max-token cap.
  It is registered in sorted bundled-profile order between Kilo Code and
  OpenAI Codex. Upstream `tests/providers/test_provider_profiles.py` covers
  profile discovery, the max-token cap, and endpoint; `test_profile_wiring.py`
  covers transport propagation. Added 2 `unit` parity tests for all profile
  fields, aliases, hostname, max tokens, and canonical list identity. Focused
  NVIDIA and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green after a
  sequential rerun; the first concurrent workspace test invocation had one
  unrelated hermes-toolsets environment-isolation flake, and the isolated
  test plus sequential required run passed. The next dependency-safe
  production unit is `plugins.model-providers.novita.__init__` (27 LOC).

- 2026-08-24 (session 4n): Ported
  `plugins/model-providers/azure-foundry/__init__.py` (@ b9aa928, 21 LOC)
  through a source-derived TDD pass. The profile mirrors the canonical
  `azure-foundry` name, `azure`/`azure-ai-foundry`/`azure-ai` aliases,
  Microsoft Foundry display metadata and signup URL,
  `AZURE_FOUNDRY_API_KEY`/`AZURE_FOUNDRY_BASE_URL` environment variables,
  the intentionally empty per-resource base URL, and explicit `api_key`
  auth. It is registered in sorted bundled-profile order immediately after
  Arcee. The upstream checkout has no dedicated plugin profile test; related
  CLI/agent tests remain future-crate oracles. Added 2 `unit` parity tests for
  all declarative fields, aliases, empty-hostname behavior, and canonical list
  identity. Focused Azure Foundry and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green
  (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored). The
  next dependency-safe production unit is
  `plugins.model-providers.nvidia.__init__` (21 LOC).

- 2026-08-24 (session 4m): Ported
  `plugins/model-providers/alibaba-coding-plan/__init__.py` (@ b9aa928, 21
  LOC) through a source-derived TDD pass. The profile mirrors the canonical
  `alibaba-coding-plan` name, `alibaba_coding`/`alibaba-coding`/
  `dashscope-coding` aliases, Coding Plan display metadata and signup URL,
  ordered `ALIBABA_CODING_PLAN_API_KEY`/`DASHSCOPE_API_KEY`/
  `ALIBABA_CODING_PLAN_BASE_URL` env vars, the dedicated coding endpoint, and
  explicit `api_key` auth. It is registered in sorted bundled-profile order
  immediately after standard Alibaba. The upstream checkout has no dedicated
  plugin profile test; related CLI/agent tests remain future-crate oracles.
  Added 2 `unit` parity tests for all declarative fields, aliases, hostname,
  and canonical list identity. Focused Alibaba Coding Plan and registry suites
  are green. Required `/home/mustbearnold/.cargo/bin/cargo build --workspace`
  and `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green
  (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored). The
  next dependency-safe production unit is
  `plugins.model-providers.azure-foundry.__init__` (21 LOC).

- 2026-08-24 (session 4l): Ported
  `plugins/model-providers/huggingface/__init__.py` (@ b9aa928, 20 LOC)
  through a source-derived TDD pass. The profile mirrors the canonical
  `huggingface` name, `hf`/`hugging-face`/`huggingface-hub` aliases,
  `HF_TOKEN`, HuggingFace display metadata, token signup URL, the two pinned
  fallback models, and `https://router.huggingface.co/v1`. It is registered in
  sorted bundled-profile order between Arcee and Kilo Code. The upstream
  checkout has no dedicated plugin profile test; related CLI/agent tests remain
  future-crate oracles. Added 2 `unit` parity tests for all declarative fields,
  aliases, hostname, and canonical list identity. Focused Hugging Face and
  registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green
  (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored). The
  next dependency-safe production unit is one of the tied 21-line profiles;
  the selected next unit is `plugins.model-providers.alibaba-coding-plan.__init__`.

- 2026-08-24 (session 4k): Ported
  `plugins/model-providers/xai/__init__.py` (@ b9aa928, 17 LOC)
  through a source-derived TDD pass. The profile mirrors the canonical `xai`
  name, `grok`/`x-ai`/`x.ai` aliases, `codex_responses` API mode,
  `XAI_API_KEY`, `https://api.x.ai/v1`, explicit `api_key` auth, and the
  pinned `Hermes-Agent/0.20.0` default header from upstream's
  `hermes_cli.__version__`. Its static registration is sorted before Xiaomi;
  the runtime CLI-version injection remains an explicit future seam. The
  upstream checkout has no dedicated plugin profile test; related CLI/agent
  tests remain future-crate oracles. Added 2 `unit` parity tests for all
  declarative fields, aliases, hostname, header, and canonical list identity.
  Focused XAI and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green
  (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored). The
  next dependency-safe production unit is
  `plugins.model-providers.huggingface.__init__` (20 LOC).

- 2026-08-24 (session 4j): Ported
  `plugins/model-providers/xiaomi/__init__.py` (@ b9aa928, 16 LOC)
  through a source-derived TDD pass. The profile mirrors the canonical
  `xiaomi` name, `mimo`/`xiaomi-mimo` aliases, `XIAOMI_API_KEY`, and
  `https://api.xiaomimimo.com/v1` endpoint, with health checks disabled,
  vision enabled, and vision tool messages disabled exactly as upstream.
  It is registered in sorted bundled-profile order with the five earlier
  static profiles. The upstream checkout has no dedicated plugin profile
  test; related CLI/agent tests remain future-crate oracles. Added 2 `unit`
  parity tests for all declarative fields, aliases, hostname, capabilities,
  and canonical list identity. Focused Xiaomi and registry suites are green.
  Required `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green
  (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored). The
  next dependency-safe production unit is the smallest remaining bundled
  provider profile, `plugins.model-providers.xai.__init__` (17 LOC).

- 2026-08-24 (session 4i): Ported
  `plugins/model-providers/openai-codex/__init__.py` (@ b9aa928, 15 LOC)
  through a source-derived TDD pass. The profile mirrors the canonical
  `openai-codex` name, `codex`/`openai_codex` aliases, `codex_responses` API
  mode, empty API-key environment tuple, ChatGPT Codex endpoint, and
  `oauth_external` auth type. It is registered in sorted bundled-profile order
  with the four earlier static profiles. The upstream checkout has no dedicated
  plugin profile test; related CLI/TUI tests remain future-crate oracles. Added
  2 `unit` parity tests for all declarative fields, aliases, hostname, and
  canonical list identity. Focused Codex and registry suites are green.
  Required `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green
  (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored). Next
  dependency-safe production unit is `plugins.model-providers.xiaomi.__init__`
  (16 LOC), the smallest remaining bundled provider profile.

- 2026-08-24 (session 4h): Ported `plugins/model-providers/stepfun/__init__.py`
  (@ b9aa928, 14 LOC) through a source-derived TDD pass. The profile mirrors
  the canonical name, `step`/`stepfun-coding-plan` aliases,
  `STEPFUN_API_KEY`, `https://api.stepfun.ai/step_plan/v1`, and the
  `step-3.5-flash` default auxiliary model. It is registered in sorted
  bundled-profile order with Alibaba, Arcee, and Kilo Code. The upstream
  checkout has no dedicated plugin profile test; related CLI/agent tests remain
  future-crate oracles. Added 2 `unit` parity tests for all declarative fields,
  aliases, hostname, and canonical list identity. Focused StepFun and registry
  suites are green. Required `/home/mustbearnold/.cargo/bin/cargo build
  --workspace` and `/home/mustbearnold/.cargo/bin/cargo test --workspace` are
  green (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored).
  Next dependency-safe production unit is
  `plugins.model-providers.openai-codex.__init__` (15 LOC), the smallest
  remaining bundled provider profile.

- 2026-08-24 (session 4g): Ported `plugins/model-providers/kilocode/__init__.py`
  (@ b9aa928, 14 LOC) through a source-derived TDD pass. The profile mirrors
  the canonical name, `kilo-code`/`kilo`/`kilo-gateway` aliases,
  `KILOCODE_API_KEY`, `https://api.kilo.ai/api/gateway`, and the
  `google/gemini-3.6-flash` default auxiliary model. It is registered in
  sorted bundled-profile order with Alibaba and Arcee. The upstream checkout
  has no dedicated plugin profile test; related CLI/agent tests remain future
  crate oracles. Added 2 `unit` parity tests for all declarative fields,
  aliases, hostname, and canonical list identity. Focused Kilo and registry
  suites are green. Required `/home/mustbearnold/.cargo/bin/cargo build
  --workspace` and `/home/mustbearnold/.cargo/bin/cargo test --workspace` are
  green (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored).
  Next dependency-safe production unit is
  `plugins.model-providers.stepfun.__init__` (14 LOC), the smallest remaining
  bundled provider profile.

- 2026-08-24 (session 4f): Ported `plugins/model-providers/arcee/__init__.py`
  (@ b9aa928, 13 LOC) through a source-derived TDD pass. The profile mirrors
  the canonical name, `arcee-ai`/`arceeai` aliases, `ARCEEAI_API_KEY`, and
  `https://api.arcee.ai/api/v1`, and is registered in sorted bundled-profile
  order alongside Alibaba. The upstream checkout has no dedicated plugin
  profile test; related `tests/hermes_cli/test_arcee_provider.py` and
  `tests/agent/test_arcee_trinity_overrides.py` cover future CLI/agent
  surfaces and remain separate inventory oracles. Added 2 `unit` parity tests
  for profile fields, aliases, hostname, and canonical list identity. Focused
  Arcee, Alibaba, and registry suites are green. Required
  `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green
  (`unit`/`mock`; 3 intentional delegation/schema doc tests ignored). Next
  dependency-safe production unit is `plugins.model-providers.kilocode.__init__`
  (14 LOC), the smallest remaining bundled provider profile.

- 2026-08-24 (session 4e): Ported `plugins/model-providers/alibaba/__init__.py`
  (@ b9aa928, 13 LOC) through a source-derived TDD pass because the pinned
  upstream checkout has no dedicated Alibaba test module. The Rust profile
  preserves the canonical name, all three aliases, `DASHSCOPE_API_KEY`, and
  the exact international DashScope-compatible base URL, then registers from
  the statically linked bundled-profile table during lazy discovery. Added 2
  `unit` parity tests covering fields, aliases, hostname, and canonical list
  identity. Focused `parity_alibaba` and `parity_registry` suites are green.
  Required `/home/mustbearnold/.cargo/bin/cargo build --workspace` and
  `/home/mustbearnold/.cargo/bin/cargo test --workspace` are green (unit/mock;
  3 intentional delegation/schema doc tests ignored). The bundled profile
  loader seam remains partial for the other provider plugin modules.
  Next dependency-safe production unit is the smallest remaining bundled
  provider profile after the metadata and ledger are synchronized.

- 2026-08-24 (session 4d): Ported `providers/__init__.py` (@ b9aa928, 198
  LOC) into `hermes-providers` through a test-first registry/discovery pass.
  The process-global registry preserves canonical insertion order, aliases,
  last-writer-wins replacement, stale-alias resolution, cache invalidation,
  and copy-safe list snapshots. Discovery marks itself before work, scans
  bundled then user plugin directories in sorted order, lets user profiles
  override bundled names, filters hidden/missing-init entries, guards module
  name collisions, scans legacy `.py` modules, and logs loader failures while
  continuing. The Rust filesystem behavior is exercised through an explicit
  loader callback because Python plugin execution and the bundled/user
  provider profile modules are not ported yet; this is recorded as the
  module's partial seam. Added 8 registry/discovery parity tests (`unit`/
  `mock`). Focused `cargo test -p hermes-providers --test parity_registry`,
  required `cargo build --workspace`, and required `cargo test --workspace`
  are green. Next dependency-safe production unit remains the provider
  profile/plugin surface, starting with the smallest bundled provider module.

- 2026-08-24 (session 4c): Opened the `hermes-providers` crate and ported
  `providers/base.py` (@ b9aa928, 238 LOC) through a TDD loop. The Rust
  `ProviderProfile` mirrors the declarative dataclass defaults, hostname and
  token-cap hooks, temperature-omit sentinel, and pass-through message/body
  hooks. `fetch_models` mirrors explicit-models URL precedence, caller
  `base_url` override behavior, list/object catalog shaping, strict UTF-8/JSON
  parsing, debug-only fail-open errors, and the five urllib redirect statuses.
  Its manual redirect path applies the upstream origin normalization and
  cross-origin `accept`/`user-agent` allowlist, preventing arbitrary provider
  credential headers from crossing origins. Added the reqwest/rustls blocking
  transport and 9 loopback parity tests in `parity_base.rs` (`unit`/`mock`).
  Focused `cargo test -p hermes-providers --test parity_base`, required
  `cargo build --workspace`, and required `cargo test --workspace` are green.
  `providers.base` is recorded `partial` because `_profile_user_agent` still
  uses the upstream fallback until the CLI crate can provide its runtime
  version, and Python's installed opener custom-handler/cookie/instrumentation
  policy has no Rust CLI owner yet. Next unit: `providers.__init__` registry
  and discovery.

- 2026-08-24 (session 4b): Resumed the support-wave handoff and completed the
  four pending code units. Committed `tools/tool_backend_helpers` as
  `358f639`, `tools/tool_output_limits` as `e563376`, `tools/working_diff` as
  `74c5286`, and the `file_state` parity temp-path isolation fix as `74e9fb8`.
  The working-diff review corrected empty-path-list handling, conditional
  `empty` result shaping, and untracked-probe error propagation against the
  upstream source. Focused evidence (`mock`/`live`) passed: backend helpers
  40, output limits 9, working diff 11, and file state 10 tests. Required
  workspace evidence (`unit`/`mock`) is green with the explicit stable
  toolchain commands:
  `PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo build --workspace`
  and
  `PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo test --workspace`.
  Three delegation/schema doc tests remain intentionally ignored. Inventory
  and conversion ledger were regenerated with
  `HERMES_UPSTREAM=/run/media/mustbearnold/Projects/Research/hermes-agent-repo tools/inventory.sh`
  and `python3 tools/conversion_ledger.py`, retaining 41 done / 7 partial /
  1,055 missing production modules. Because the local HTTPS Git client has no
  credentials, the connected GitHub API published the 24 code commits and
  metadata commits as a sequential remote mirror; each remote commit has the
  same tree snapshot and message as its local counterpart, but a different
  SHA because the API cannot preserve the local author/committer timestamps.
  Each tree was verified identical before its remote ref update.

- 2026-08-23 (session 4a): Hermes-tools support wave ported against upstream
  @ b9aa928. Added audio_container, computer_use/schema, credential_files,
  daemon_pool, debug_helpers, delegation_output_schema, desktop_ui, env_probe,
  fal_common, interrupt, mcp_schema_cache, read_preview_tool,
  read_terminal_tool, slash_confirm, terminal_hints, thread_context,
  threat_patterns, todo_tool, tool_backend_helpers, tool_output_limits, and
  working_diff, with parity tests and golden schema fixtures where needed.
  The ledger records 41 done / 7 partial production modules after
  `HERMES_UPSTREAM=/run/media/mustbearnold/Projects/Research/hermes-agent-repo tools/inventory.sh`.
  This count also reconciles the already-landed `agent.redact`,
  `agent.session_activity`, and top-level `model_tools` ports into the
  machine-readable overlay.
  Partial seams are explicit: real JSON-Schema validation, full NFKC, config
  and provider integrations, AIAgent/task context, and credential-file
  ContextVar semantics. Two full-workspace test-isolation fixes were also
  landed: a shared hermes-logging queue mutex and a shared hermes-constants
  environment/profile mutex. Evidence (`unit`/`mock`):
  `PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo build --workspace`
  and the same explicit-toolchain command with `cargo test --workspace` are
  green; 3 delegation/schema doc tests remain intentionally ignored.

- 2026-08-22 (session 3c): agent/file_safety.py ported — hermes-tools
  src/file_safety.rs (@ b9aa928, 693 LOC; homed in hermes-tools until the
  agent crate opens). Write denial: build_write_denied_paths/prefixes
  (ssh/aws/gnupg/kube/docker/azure/github/gcloud + sudoers/systemd + ENV
  store + .netrc/.pgpass/.npmrc/.pypirc/.git-credentials + hermes state.db
  / sessions / mcp-tokens / pairing via home+root), HERMES_WRITE_SAFE_ROOT
  gate, classify_write_denial returning credential/safe_root/None.
  Read denial: skills/.hub cache-blocking, credential-store exact files
  (auth.json/.lock/.env/.anthropic_oauth/webhook_subscriptions/google
  oauth/bws_cache), mcp-tokens prefix, project-local env-basename block
  (case-insensitive, .env.example allowed). Uses hermes-constants home/
  root resolution through the ported get_hermes_home /
  get_default_hermes_root. Oracle: tests/agent/test_file_safety.py +
  write-denial classification; 5 pure-path parity tests + 2 isolated
  env-binary tests (HERMES_HOME / HERMES_WRITE_SAFE_ROOT are process-
  global, isolated like the file-state kill switch); workspace 555 tests
  green; clippy clean. Evidence: `cargo test --workspace` (unit).
- 2026-08-22 (session 3d): tools/tts_text_normalize.py ported — hermes-tools
  src/tts_text_normalize.rs (@ b9aa928, 278 LOC). Full spoken-text pipeline:
  strip_nonspoken_blocks (think/</think> blocks #34213 + unterminated
  thinking stream, file-mutation verifier footer #40772), strip_markdown_for_tts
  (code/link/image/inline/bold/italic/strike/heading/blockquote/list/hr/table
  pipe; bold/italic/strike compiled with DOTALL like upstream re.DOTALL),
  normalize_symbols_for_tts (nbsp/minus/ellipsis, temp ranges 11–17°C → 11 to 17
  degrees Celsius, km/h-mm/cm/m, numeric rates, NZ/AU/US$/€/£/$ money, percent,
  & → and, bullets, arrows, variation selectors, emoji strip),
  smooth_whitespace_for_tts (heading fold + sentence pauses),
  flatten_newlines_for_payload (#9004 Kokoro), prepare_spoken_text (default
  max_chars caller-side Some(4000); truncation trims trailing whitespace).
  REGEX FIXES vs first draft: think-block closing tag is ` response`
  (no space) and verifier footer warns ⚠ U+26A0 + optional variation selector
  U+FE0F — both were corrupted to space-less variants by an HTML renderer;
  written back as \x3c/\u{fe0f} escapes so they cannot regress.
  html.unescape: replaced a 7-entity table with a faithful CPython
  html.unescape port (tools/gen_html5_entities.py → src/html5_entities.rs,
  2,231 HTML5 named refs incl. 93 multi-codepoint + legacy semicolon-less +
  _invalid_charrefs C1 map + _invalid_codepoints) — semicolon-less legacy
  (&lt, &copy, &amp), longest-match invalid refs (&notit; → ¬it;),
  &#x110000;/&#xD800; → U+FFFD, &#13; CR and &#0; ✓ byte-parity. Oracle:
  tests/tools/test_tts_text_normalize.py + test_tts_prepare_spoken.py (share-
  cleaner wiring cases deferred with gateway/tts_tool) → 14 parity tests in
  parity_tts_normalize.rs + upstream/golden_tts_text_normalize.json (36-case
  stage-by-stage byte corpus incl. entity edge classes and multi-line
  bold/italic/strike). FIXED pre-existing hermes-toolsets flake
  (resolve_special_all_alias intermittently raced the process-global registry
  vs registry_tools_merge_into_builtin_toolset; tests now serialize via a
  REGISTRY_TEST_LOCK). Workspace 569 tests green; clippy clean. Evidence:
  `cargo test --workspace` (unit).
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
