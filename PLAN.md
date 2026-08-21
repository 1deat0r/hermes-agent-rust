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
    hermes-state/          # hermes_state{,_schema,_common,_portability,_search}.py  (Phase 1)
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
| iter_hermes_node_dirs / _candidate_node_command_names | 🟡 deferred (node subsystem, P2) | — |
| node_tool_runnable / managed-node bootstrap/heal | ❌ deferred (node subsystem, P2) | — |
| agent_browser_runnable | ❌ deferred (P2) | — |
| display_hermes_home / secure_parent_dir | 🟡 deferred (P1, needs utilities) | — |
| profile-home helpers (_profile_home_path, get_real_home, …) | ❌ deferred (P1, profile subsystem) | — |
| resolve_per_model_reasoning_effort / resolve_reasoning_config | 🟡 deferred (P1, needs config crate) | — |
| apply_ipv4_preference | ❌ deferred (P3 networking) | — |
| partial_update_hint / update diagnostics | ❌ deferred (P3) | — |

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

Evidence format: every claim in this file must cite `unit` | `mock` | `live`
+ the exact command, e.g. `cargo test -p hermes-time (unit)`.

## 7. Session log

- 2026-08-22 (session 1): P0 scaffold. Inventory generated, governance set,
  workspace created, hermes-constants foundational subset + hermes-time full
  port landed with parity tests. Binary renamed/shaped; golden fixtures
  vendored from upstream `_canonical_model_variants` /
  `parse_reasoning_effort` (@ b9aa928); parity tests assert exact full-list
  equality. Evidence: `cargo test --workspace` (unit) — 74 ok.
- 2026-08-22 (session 1b): GitHub repo `1deat0r/hermes-agent-rust` created
  (public). Commit policy: local+remote together on every commit (standing
  rule).
