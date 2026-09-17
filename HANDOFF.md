# Hermes Agent Rust — Next-session handoff

Date: 2026-09-16 (Pacific/Auckland), retarget session.

## Resume point

Repository: `/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust`

Pinned upstream commit: **`5d59366`**
(`5d59366010640c1d6b8f170d8a4ee109db2bbdef` — latest `origin/main` as of
2026-09-16 NZ, `feat(plugin-catalog): add 66 community plugins`). The previous
pin `b9aa928` is superseded. Always regenerate the inventory against the
pinned worktree, never HEAD:

```bash
git -C /run/media/its1deat0r/Projects/Research/hermes-agent-repo \
    worktree add --detach /tmp/hermes-upstream-5d59366 5d59366
HERMES_UPSTREAM=/tmp/hermes-upstream-5d59366 bash tools/inventory.sh
python3 tools/conversion_ledger.py
git -C /run/media/its1deat0r/Projects/Research/hermes-agent-repo \
    worktree remove /tmp/hermes-upstream-5d59366
```

Scope now includes the **Hermes Agent Desktop Linux app**: `apps/desktop`
(Electron 40.10.2 main + React renderer, Linux AppImage/deb/rpm via
electron-builder, desktop `0.17.3`), `apps/shared` (`@hermes/shared`
contract lib), the Tauri `apps/bootstrap-installer` (`0.21.1`, upstream's
port precedent), and root `tests-js` desktop gates. First-party TS/JS is
tracked as `ts:` modules under P6: 2,539 total = 1,506 production
(~381k LOC) + 1,033 oracles; sibling `web/`, `ui-tui/`, e2e/diag/perf
harnesses, and fixtures are excluded (R2-reconciled; the earlier "2,525"
was the pre-oracle-split count). Port strategy (user-approved):
`hermes-desktop` crate via Tauri, renderer strategy per-module.

## What landed this session (retarget 5d59366)

- New pinned worktree `/tmp/hermes-upstream-5d59366` at `5d59366`.
- `tools/inventory.py` extended: walks `apps/` + `tests-js` for first-party
  TS/JS (`ts:` rows, test detection via `.test.`/`.spec.`/`__tests__`/`e2e`),
  scope exclusions (e2e, `__fixtures__`, pr-assets, public, diag/perf/repro
  harnesses, `.d.*`, `.e2e.*`), packages `desktop` /
  `ts:apps.shared+installer` / `ts:web+tui+tests`.
- `tools/conversion_ledger.py`: new P6 phase (`ts:` rows); PHASE_ORDER
  bumped (oracle → 7).
- `tools/port_status.json`: 162 drifted done/partial Python rows demoted to
  `partial` with `retarget_note` (LOC changed between pins — refactors incl.
  `hermes_state.py` → 20+ modules); 5 unchanged rows keep `done`
  (`agent.tool_result_classification`, `agent.trajectory`,
  `hermes_cli.lifecycle`, `tools.budget_config`, `tools.read_terminal_tool`).
  4 stale package-rollup keys folded to real `__init__` modules;
  `tools.mcp_stdio_watchdog` (deleted upstream) → successor
  `tools.mcp_death_supervisor` partial.
- Version literals bumped to the new pin: `hermes_cli::{VERSION,
  RELEASE_DATE}` = `0.21.3` / `2026.9.14`; provider UA strings, portal tags,
  run_agent routermint headers, gateway-health fixtures follow.
- Governance updated: AGENTS.md (pin + desktop scope), PLAN.md (target,
  §1 table, crate tree, P6 phase row), GATES.md (scope, P6 branch, G1/G4
  counts + pin), `tools/pre_commit_docs.py` pin gate → `5d59366`,
  README.md (scope, percentages, pin).
- **Ledger: 5 done / 164 partial / 8,726 missing tracked (0.06%); 5 / 164 /
  3,312 production (0.14%).** The percentage reset is honest demotion +
  added desktop scope, not lost work — prior ports remain the starting
  point for re-certification.

## Verification evidence (retarget session)

- `cargo build --workspace` — green.
- `cargo test --workspace -- --test-threads=1` — 1,781 passed, 0 failed.
- `git diff --check` — clean.

## Re-cert batch 1 (2026-09-16, @ 5d59366)

22 modules promoted (5 → 27 done): 18 AST-identical-modulo-docstring rows
re-run green, 4 refactor-only drifts re-certified green, plus a TDD port of
`classify_jwks_lookup_error` (#94558, 5 oracle tests in
`crates/hermes-cli/tests/parity_dashboard_auth_jwks.rs`). Full validation:
`cargo test --workspace -- --test-threads=1` — **1,786 passed, 0 failed**;
`git diff --check` clean. **Ledger: 27 done / 142 partial / 8,726 missing
tracked (0.30%); production 27 / 142 / 3,312 (0.78% — computed 0.7756%, ledger rounds display).**

## Board gate: PASSED UNANIMOUS (R1→R3, 2026-09-16)

R1 1B/3C/1R → adjudicated live (7 confirmed, 2 partly, 1 refuted) →
revisions (KeyMaterial variant, hardened jwks tests, desktop scaffold,
relabeled notes, reconciled counts) → R2 verify-by-quote → R3 S3+S6
confirm → **unanimous BUILD incl. cold-read seat**. Record:
`docs/board-review-r1.md`. Long-horizon 100% goal:
`docs/long-horizon-100.md`. Validation at gate: 1,789 passed / 0 failed
serial, `git diff --check` clean. Ledger: 27/142/8726 (0.30%).

## mattpocock skills installed + first skilled unit (2026-09-16)

`mattpocock/skills` @ `959a8e9` (33 skills, copy-snapshot,
`~/.config/opencode/skills/`); repo config in `docs/agents/` + AGENTS.md.
First unit under the skills: `ts:apps.desktop.electron.active-runtime-state`
→ done (9 tests, review-closed). **Ledger: 28/142/8725 (0.31% tracked,
0.80% prod).** Validation: serial 1,798/0, diff clean.

## Board gate R2: PASSED UNANIMOUS (R1→R4, 2026-09-16)

4B/1C → S2 line fixed → S3 kills died live → R2 quote + S6 cold-read →
R3/R4 pointer fixes → unanimous BUILD. Record: `docs/board-review-r2.md`.
Live validation at close: serial 1,798/0 (195 suites), diff clean.
Next goal: `docs/long-horizon-101.md` (waves A–D, M1–M7, all-33 lanes).

## Wave A batch 1 done (2026-09-16)

4 rows (`agent.jiter_preload` + 3 package surfaces), review-closed.
**Ledger: 32/142/8721 (0.36% tracked, 0.92% prod).** Serial 1,805/0.

## Wave A batch 2 done (2026-09-16)

`hermes-platforms` crate + `plugins/__init__` + slack entry (PluginCtx seam,
review-clean). **Ledger: 34/142/8719 (0.38% tracked, 0.98% prod).**
Serial 1,807/0.

## Wave A batch 3 done (2026-09-16)

21 platform entries (all `plugins.platforms.*/__init__` complete).
**Ledger: 55/142/8698 (0.62% tracked, 1.58% prod).** Serial 1,828/0.
Next: Wave A remaining trivials (acp/cron-scripts/tui roots need crates;
evals/skills sweep) or Wave B partials.

## Wave B unit 1 done (2026-09-16)

lmstudio_reasoning + reactions + message_content re-certified.
**Ledger: 58/139/8698 (0.65% tracked, 1.67% prod).** Serial 1,828/0.

## Wave B unit 2 done (2026-09-16)

model_search (data) + path_security (missing fn) + skill_provenance +
qqbot.utils (cosmetic). **Ledger: 62/135/8698 (0.70% tracked, 1.78%
prod).** Serial 1,831/0.

## Wave B unit 3 done (2026-09-16)

bedrock rename + ai-gateway/gemini recerts; redaction scoped own unit.
**Ledger: 65/132/8698 (0.73% tracked, 1.87% prod).** Serial 1,831/0.

## Wave B redaction unit done (2026-09-16)

egress refactor ported; 5 stale expectations fixed live.
**Ledger: 67/130/8698 (0.75% tracked, 1.92% prod).** Serial 1,841/0.

## Wave B unit 4 done (2026-09-16)

binary_extensions (opaque set) + verify.environment + timeouts.
Flaky bitwarden assertion fixed deterministically.
**Ledger: 70/127/8698 (0.79% tracked, 2.01% prod).** Serial 1,844/0.

## Summaries batch done (2026-09-16)

Any-delta + append/backfill ported; 6 recerts; partials <100.
**Ledger: 98/99/8698 (1.10% tracked, 2.82% prod).** Serial 1,868/0.

## CLI batch done (2026-09-16)

line_input + soul template ports; 3 recerts.
**Ledger: 103/94/8698 (1.16% tracked, 2.96% prod).** Serial 1,873/0.

## Affinity batch done (2026-09-16)

affinity triple + kimi UA; 4 recerts; aux-model deferred.
**Ledger: 109/88/8698 (1.23% tracked, 3.13% prod).** Serial 1,874/0.

## Sweep batch done (2026-09-16)

6 recerts (events/ssl/kanban verified, not just greened).
**Ledger: 115/82/8698 (1.29% tracked, 3.30% prod).** Serial 1,874/0.

## Actual unit done (2026-09-16)

thinking-toggle reasoning + metadata; above-crate seams deferred.
**Ledger: 116/81/8698 (1.30% tracked, 3.33% prod).** Serial 1,875/0.

## Tags batch done (2026-09-16)

unicode-tag strip ported; 4 recerts.
**Ledger: 121/76/8698 (1.36% tracked, 3.48% prod).** Serial 1,879/0.

## Wave B provider batch done (2026-09-16)

alibaba×3, ollama clamp, nvidia/vertex recerts, registry 38→41.
**Ledger: 81/116/8698 (0.91% tracked, 2.33% prod).** Serial 1,856/0.

## timefmt/budget unit done (2026-09-16)

coerce_epoch + warning-ratio ports; 2 recerts; daemon flake noted.
**Ledger: 92/105/8698 (1.03% tracked, 2.64% prod).** Serial 1,866/0.

## Anthropic pagination done (2026-09-16)

Cursor loop + shared page fetcher; deepseek/upstage recerts.
**Ledger: 88/109/8698 (0.99% tracked, 2.53% prod).** Serial 1,858/0.

## Wave B unit 5 done (2026-09-16)

7 smallest-DIFF partials closed: alibaba-coding-plan +CN profile, nvidia
(4 aliases + prepare_messages strip), vertex (+vertexai), interrupt_compat
(tool_reason gate), ssl_verify (context-cache seam), verify_hooks
(docstring-only), hermes_cli.__init__ (new utf8 module, 5 tests). 11 new
tests. **Ledger: 77/120/8698 (0.87% tracked, 2.21% prod).** Serial 1,855/0.

## Next actions, in order

1. Wave B unit 6: next smallest-DIFF partials (`agent.ssl_guard`,
   `plugins.model-providers.alibaba.__init__` regional set,
   `hermes_cli.dashboard_auth.__init__` remainder).
2. Wave A: trivial sweep (`__init__` 0–3 LOC, eslint configs, barrels —
   acp/cron-scripts/tui roots need crate-open decision first).
3. Keep ownership disjoint; commit and publish each logical unit immediately.

## Archive: session 4da (2026-08-31, pin b9aa928 — superseded)

Current branch/HEAD: `main` carries this session's four units committed with
this documentation checkpoint. Previous: `d138528` (4d9's
`browser_camofox_state`/`focus_pane_tool`/`cwd_placeholder`), `e6079b9`,
`a06d8fb`, `aedb1f2`, `2468dc7`.

## What landed this session (4da)

Fifty-five units across four crates, all red-first, 366 new parity tests:

- `hermes_cli/main.py` `_read_packed_ref` + `_read_git_revision_fingerprint`
  → `hermes-cli::git_revision` (8 tests, source-derived — no dedicated
  upstream test file; gap noted). Real `.git` fixtures: detached HEAD, loose
  refs, packed-refs (comments/peel lines skipped), the `unresolved` marker,
  worktree `.git`-file indirection with `commondir`, OSError fail-open.
  `hermes_cli.main` is marked `partial` (two private helpers of a ~5k-line
  module).
- `gateway/code_skew.py` → `hermes-gateway::code_skew` (10 tests mirroring
  `tests/test_code_skew.py`; `TestModelSwitchSkewGuard` stays PENDING with
  the unported `gateway.slash_commands`). Test seams
  (`set_fingerprint_override_for_tests`/`reset_for_tests`) stand in for the
  upstream `monkeypatch` fixture.
- `gateway/cgroup_cleanup.py` → `hermes-gateway::cgroup_cleanup` (8 tests
  mirroring `tests/gateway/test_cgroup_cleanup.py`, incl. one live SIGKILL of
  a spawned `sleep` child). Only successful kills count toward the tally;
  `reap_cgroup(None)` is never exercised in tests (it would kill the test
  process's own cgroup).
- `gateway/rich_sent_store.py` → `hermes-gateway::rich_sent_store` (8 tests,
  source-derived; gap noted). Python-falsiness guards, fail-open lookup
  (missing/corrupt/non-dict/falsy-`t`), 1000-entry trim by oldest `ts` with
  a stable sort (same-second timestamps trim in insertion order).

Batch 2: `tools/skill_provenance.py` → `hermes-tools::skill_provenance`
stub filled (5 tests; scoped thread = `copy_context().run`; token carries
the prior value like `ContextVar.reset`) and `gateway/session_stall.py` →
`hermes-gateway::session_stall` (8 tests; numeric-string `float()` accepts,
non-finite fall-through, negative-idle clamp; the `GatewayRunner`
notify-once loop stays PENDING with `gateway.run`).

Batch 3: `gateway/readiness.py` → `hermes-gateway::readiness` (9 tests
mirroring both cases in `tests/gateway/test_readiness.py` plus
source-derived probe cases: corrupt db degrades without repair, empty YAML
= defaults, draining-is-ok gateway states, `state or status` connected
counting, negative-counter clamps). The crate gains `rusqlite` and
`serde_yaml` for the probes; `hermes-state` remains dev-only.

Batch 4: `tools/open_preview_tool.py` → `hermes-tools::open_preview_tool`
(7 tests; registry entry oracle `desktop_ui`/no check_fn, emitter-panic
except-arm reproduction, no-emitter desktop-only arm, and the
`_normalize_target` grammar — bare-domain https, localhost/loopback http
with re.I and `[::1]`, sub-2-char TLDs bare, path/scheme/`file:`/`~`
pass-through, backtick/whitespace stripping).

Batch 5: `hermes_cli/timefmt.py` → `hermes-cli::timefmt` (6 source-derived
tests, gap noted; falsy-ts → "?", truncating minute/hour/day divisions,
24–48h "yesterday" band, future timestamps read "just now", local
`%Y-%m-%d` past a week; `chrono` added to the crate) and
`agent/iteration_budget.py` → `hermes-agent::iteration_budget` (6
source-derived tests, gap noted; consume/refund contract, remaining clamp
under a shrunk public `max_total`, independent per-agent budgets, 8-thread
race stays exactly at the cap).

Batch 6: `agent/reactions.py` → `hermes-agent::reactions` (6 source-derived
tests, gap noted; the vibe lexicon pinned arm-by-arm, with boundary cases
verified against the Python oracle: "goodbot" fires because `good\s*bot`
has no internal boundary, "empty"/"tyty" stay silent via `\bty\b`) and
`gateway/platforms/_http_client_limits.py` →
`hermes-gateway::platform_http_limits` (5 tests mirroring the applicable
oracle cases; ported as a transport-agnostic `HttpPoolLimits` struct — the
`httpx is None → None` arm has no Rust analog, noted in the module doc).

Batch 7: `agent/trajectory.py` → `hermes-agent::trajectory` (7
source-derived tests, gap noted; scratchpad→think conversion,
incomplete-scratchpad detection, JSONL append semantics, the
completed→default-filename decision, naive-local ISO timestamps,
fail-open IO). `save_trajectory` returns the written path for callers;
upstream returns None implicitly. The AIAgent method
`_convert_to_trajectory_format` stays pending with run_agent.

Batch 8: `gateway/platforms/qqbot` package opened — `crypto`
(AES-256-GCM `generate_bind_key`/`decrypt_secret`; test-side encryptor
reproduces the portal layout; the crate gains `aes-gcm`, `base64`, `rand`),
`constants` (all values pinned incl. `QQ_PORTAL_HOST` Lazy read-once), and
`utils` (`build_user_agent` grammar with a Python-version slot reporting
`unknown` — a Rust runtime has no interpreter, documented in the module;
`coerce_list` keeps Python `str()` semantics incl. `True`/`False`).

Batch 9: `tools/mcp_stdio_watchdog.py` → `hermes-tools::mcp_stdio_watchdog`
lib module + native binary `src/bin/mcp_stdio_watchdog.rs` (9
source-derived tests, gap noted). The child runs in its own process group
(`Command::process_group(0)` = `start_new_session`); SIGTERM/SIGINT are
forwarded to the child's group async-signal-safely, exiting 128+sig; the
watchdog thread polls getppid at 2s and SIGTERMs then SIGKILLs the child's
group after the 3s grace. hermes-tools gains `libc` in [dependencies].

Batch 10: `agent/monitoring/` package — `events.py` (three typed
content-free event dataclasses with `to_dict()` discriminator-first
dicts), `emitter.py` (bounded 10k ring queue, drop-oldest-when-full,
lazy 256-batch dispatcher thread, fail-isolated subscribers,
opt-in singleton starting disabled, flush/stats/close, and the
`TelemetryEmitter` back-compat alias), and the package `__init__`
re-exports. The hot-path invariant (`emit` never blocks on
disk/network, never raises) is preserved via a bounded Mutex+Condvar
queue with panic-isolated subscriber fan-out.

Batch 11: `agent/monitoring/redaction.py` → `monitoring::redaction` done
(force-redactor wrap + bearer/token/literal/residue shapes + PII →
`[email]`/`[id]`/`[phone]`; the `regex` crate has no lookbehind so the
phone pattern's `(?<!\w)`/`(?!\w)` guards became boundary capture groups
that are restored on replacement) and `cron_health.py` → **partial** (pure
projections ported: classify/job-key/duration/projection/terminal-emit;
`build_cron_health_snapshot`/`_is_overdue` PENDING with
`cron.jobs`/`cron.scheduler`/`GatewayMetric`).

Batch 12: `agent/monitoring/gateway_health.py` →
`monitoring::gateway_health` done (11 source-derived tests, gap noted).
`GatewayMetric`/`GatewayHealthSnapshot` now exist, unblocking the
cron_health snapshot builder. The singleton-touching tests serialize
behind a mutex — parallel tests closing the shared emitter race each
other's dispatch.

Batch 13: `agent/monitoring/otlp_exporter.py` → **partial**. Ported: the
config probe, `is_enabled`, `resolve_headers` (env-name indirection,
missing vars skipped, values never logged), `span_attrs` per-kind keep
lists with redaction+500-char bounds, and `export_batch` over a
caller-wired `SpanSink` trait (panic fail-isolated). PENDING: the
opentelemetry-sdk transport (`_require_sdk`/`build_exporter`/
`_make_provider`/`OTLPStreamer`/`start_streaming`/`is_available`) — no
Rust optional-SDK analog; the SDK seam is the `SpanSink` the operator
plane wires — and `_resource_attributes` (needs `policy.ensure_install_id`).

Batch 14: `agent/monitoring/gateway_health_export.py` → **partial**.
Ported: `_safe_resource_attributes` (allowlist + `^[A-Za-z0-9._:/-]{1,128}$`
grammar + redaction-changed rejection + sha256 instance-id routing),
`_diagnostic_log_attributes`, both config probes and `_enabled`,
`_metric_endpoint`/`_logs_endpoint`, `_supervision_mode` env probe,
`_severity_number` (OTel enum values: FATAL=24/ERROR=17/WARN=13/INFO=9/
DEBUG=5), `_gateway_health_event` plane filter, `_redact_string`, and
`_DEFAULT_DIAGNOSTIC_SCOPE`. PENDING: `GatewayHealthExportRuntime`/
`start_gateway_health_export` and the SDK streamer/provider classes (OTel
transport, snapshot thread, root-logger attachment) and `_install_id`
(`policy.ensure_install_id`).

Batch 15: `gateway/platforms/qqbot/keyboards.py` → **partial**. Ported:
the keyboard dataclass tree (`InlineKeyboard`→`KeyboardContent`→rows→
buttons with render/action/permission), approval + update-prompt
builders (mutually-exclusive `group_id="approval"`, allow-always toggle,
grey deny styling), the greedy-session-key button_data parsers
(`approve:<key>:(allow-once|allow-always|deny)` — key may contain
colons; needs ≥1 char), `ApprovalRequest` + exec/plugin markdown
renderers (300-char preview bound, title suppressed when equal to the
preview, severity icons), and `InteractionEvent` parsing with the
group→user→resolver `or`-chain operator openid. PENDING:
`ApprovalSender` (async HTTP over adapter callables) — adapter
transport.

Batch 16: `hermes_cli/colors.py` → `hermes-cli::colors` done (6
source-derived tests, gap noted). The `Colors` ANSI constant table;
`should_use_color` with NO_COLOR-any-value (Python `is not None` — even
empty disables, per no-color.org), TERM=dumb, and non-TTY stdout disable
arms; `color` joins the codes as a prefix and appends RESET, passing text
verbatim when disabled. Tests pin the grammar under both TTY outcomes.

Batch 17: `hermes_cli/sqlite_util.py` → `hermes-cli::sqlite_util` done
(6 source-derived tests, gap noted). `add_column_if_missing` keeps the
`ddl` parameter carrying the full definition incl. the column name (e.g.
"note TEXT") exactly as upstream call sites pass it — the first red run
caught my test passing a bare type. `write_txn` becomes a closure form
(BEGIN IMMEDIATE → body → COMMIT; guarded ROLLBACK that never shadows
the original error). The crate gains `rusqlite` (bundled).

Batch 18: `hermes_cli/secret_prompt.py` → `hermes-cli::secret_prompt`
done (7 source-derived tests, gap noted; `_collect_masked_input` is pure
over injected read/write closures — enter commits, Ctrl-C interrupts,
Ctrl-D/Z are EOF, backspace erases only non-empty, bare ESC ignored,
`if mask:` arm; POSIX raw-mode termios session with TCSADRAIN restore is
the runtime form; the msvcrt Windows arm is cfg'd out) and
`hermes_cli/cli_output.py` → `hermes-cli::cli_output` done (4 tests; the
print_* helpers route through colors; `prompt`/`prompt_yes_no` take an
injected line reader as the input() seam — EOF returns "" not the
default, matching upstream's except arm).

Batch 19: `hermes_cli/default_soul.py` → `hermes-cli::default_soul` done
(4 in-crate tests, source-derived, gap noted). The legacy-template table
is a safety guarantee: the two comment-only scaffolds carry zero user
intent so a match is safe to upgrade in place; `_normalize_soul` unifies
CRLF/CR, strips a leading BOM, and trims; any deviation (even edits inside
the comment) defeats the match.

Batch 20: `agent/verify/` package opened — `recipes.py` (Recipe
to_dict/from_dict with grok alias tolerance, lockfile package-manager
priority, Node framework+script+port inference, Python/Go/Rust/Java/
Make/compose fallbacks in grok's order) and `environment.py`
(manifest save/load with corrupt-file degradation; manifest wins over
detection) — 8 source-derived tests, gap noted. PENDING:
`verify/runner.py` (smoke-test runner) and the `__init__` re-exports.
Test-oracle notes: `.PHONY` DOES match the make-target grammar (dots in
the character class); pnpm/yarn script runners omit `run`
(`pnpm dev`, `yarn dev`) while npm/bun keep it.

Batch 21: `agent/verify/runner.py` → `verify::runner` done (9
source-derived tests, gap noted). `shell=True` → `sh -c`; readiness
probing is a raw HTTP/1.0 GET over TcpStream where ANY response (even
404) proves up — the urllib HTTPError arm; the child spawns with
`process_group(0)` at spawn time and teardown SIGTERMs then SIGKILLs the
group (10s/5s deadlines). Start-phase tests use distinct loopback ports —
parallel tests sharing a port cross-talk via the other's server.

Batch 22: `hermes_cli/dashboard_auth/` package opened — `base.py`
(`DashboardAuthProvider` trait with capability flags and loud unimplemented
defaults; Session/TokenPrincipal/LoginStart; ProviderError→503,
InvalidCodeError→400, InvalidCredentialsError→401-generic,
RefreshExpiredError semantics; assert_protocol_compliance pins
non-empty name/display_name) and `registry.py` (registration order,
duplicate rejection, token/session capability subsets, fail-closed token
plane, clear_providers) — 5 source-derived tests, gap noted. The crate
gains `async-trait` + `futures`(executor). PENDING: middleware, cookies,
routes, login_page, native_flow, token_auth, ws_tickets, prefix, audit
(web-server surface).

Batch 23: `hermes_cli/dashboard_auth/public_paths.py` → done (the shared
allowlist both auth middlewares import — the drift fix for the portal's
wildcard liveness probe; minimal by contract: uptime-probe/SPA/curl safe)
and `prefix.py` → **partial** (X-Forwarded-Prefix normalisation with the
256-char budget and `..`/`//`/quote/whitespace rejection; public_url
resolution env→config with malformed fall-through; env-only resolution
until the hermes_cli.config seam ports — the config leg is parameterised).

Batch 24: `hermes_cli/dashboard_auth/audit.py` → done (JSONL audit at
$HERMES_HOME/logs/dashboard-auth.log; REDACTED_FIELDS stripped before
serialisation; fail-open writes) and `token_auth.py` → **partial**
(token-route registry, bearer extraction, stacked
`(principal, unreachable)` authenticate over the token providers; the
FastAPI middleware stays PENDING — its 401/503 decision table rides the
returned values; audit_token_failure covers both TOKEN_AUTH_FAILURE
shapes). Registry-touching tests serialize behind a mutex (the provider
registry is process-global).

Batch 25: `hermes_cli/dashboard_auth/ws_tickets.py` →
`dashboard_auth::ws_tickets` done (5 source-derived tests, gap noted).
Single-use browser tickets (43-char base64url, TTL 30s, `expires_at <
now` expiry with expired-ticket GC, truncated unknown-ticket error) and
the process-lifetime internal credential (multi-use, never expires,
constant-time compare, rejected before first mint). Explicit-clock
`*_at` forms are the `time.time` patch seam. Tests serialize over the
process-global store.

Batch 22: `hermes_cli/dashboard_auth/cookies.py` →
`dashboard_auth::cookies` done (10 source-derived tests, gap noted).
`resolved_name` three-prefix rules, SameSite=Lax/HttpOnly/
Secure-only-over-HTTPS, provider-reported AT TTL, 30-day RT upper bound
with empty-RT degradation to access-token-only, Max-Age=0 deletions across
all name variants, most-strict-first read fallback, PKCE + SSO
loop-guard marker cookies. The FastAPI Response/Request seams are a
SetCookie directive list + cookie-lookup closure; `detect_https` takes
the request scheme (honours X-Forwarded-Proto upstream of the seam).

Batch 27: `hermes_cli/dashboard_auth/native_flow.py` →
`dashboard_auth::native_flow` done (6 source-derived tests, gap noted).
Gateway-brokered RFC 8252 store: pending authorizations (600s TTL, 8/IP
cap exempting empty IPs, 256 global cap) and one-time gateway codes
(120s TTL) bound to the desktop's S256 challenge; redemption pops before
the PKCE check (no retry/oracle/replay) with constant-time comparison.
S256 pinned against the RFC 7636 appendix-B sample. Fidelity nuance: the
CodeExpired arm is dead in practice — GC inside redeem_code drops expired
entries before lookup, so expiry reports CodeInvalid; verified against
the Python oracle. The crate gains `sha2`.

Batch 28: `agent/ssl_guard.py` → `hermes-agent::ssl_guard` done (7 tests
mirroring `tests/agent/test_ssl_ca_guard.py`). Certifi's leg is
parameterised as the platform bundle path; the
`ssl.create_default_context`/`get_ca_certs()` check degrades to PEM
certificate-block presence (true x509 parsing left to the TLS stack —
documented divergence). Guard order: skip env → 4 env vars → platform
bundle (substantial ≥1024 bytes + certificate blocks). Repair hint joins
the message with a newline, verbatim.

Batch 29: `tools/mcp_dashboard_oauth.py` ->
`hermes-tools::mcp_dashboard_oauth` done (11 source-derived tests, gap
noted). Flow state machine (starting -> authorization_required ->
approved/error) with condvar waits standing in for
asyncio.to_thread(event.wait); state pinning from the authorization URL,
constant-time callback state compare, duplicate-callback rejection,
mark_error waking waiters and no-op after approval; the ContextVar
current-flow becomes a thread-local slot with restore-on-drop guard.

Batch 30: `tools/browser_dialog_tool.py` ->
`hermes-tools::browser_dialog_tool` done (6 source-derived tests, gap
noted). The SUPERVISOR_REGISTRY lookup is injected as a settable closure
returning a `DialogResponder` trait object (result dict carries
ok/dialog/error exactly as the supervisor's respond_to_dialog);
task_id defaults to "default"; responder errors pass through verbatim;
registry entry in the browser-cdp toolset. The CDP supervisor itself
(1,518 LOC) stays PENDING.

Batch 31: `tools/close_terminal_tool.py` ->
`hermes-tools::close_terminal_tool` done (7 source-derived tests, gap
noted) — previously flagged as blocked on process_registry, now ported
via the seam pattern: the registry's `on_close` sink injects as a settable
closure and the `request_close_terminal` contract (desktop-only error
dict, error-string passthrough, ok + closed + note) is implemented
locally. The registry mega-module is still PENDING for the remaining
surface (spawn/poll/read/kill), whose close_stdin etc. wait the same way.

Batch 32: `agent/think_scrubber.py` ->
`hermes-agent::think_scrubber` done (13 source-derived tests, gap noted).
The streaming tag-suppression state machine: in_block/buf/
last_emitted_ended_newline, closed-pair priority over boundary-gated
opens, partial-tag hold-back with char-boundary-safe byte slicing,
orphan-close stripping with trailing whitespace, flush discarding
unterminated blocks and resetting the boundary flag for intra-turn
retries. All five tag variants (think/thinking/reasoning/thought/
REASONING_SCRATCHPAD) case-insensitive.

Batch 32: `agent/think_scrubber.py` ->
`hermes-agent::think_scrubber` done (13 source-derived tests, gap noted).
The streaming tag-suppression state machine: closed-pair priority over
boundary-gated opens, partial-tag hold-back with char-boundary-safe byte
slicing, orphan-close stripping with trailing whitespace, flush
discarding unterminated blocks and resetting the boundary flag for
intra-turn retries. All five tag variants case-insensitive. PLAN.md
corruption (duplicated session-log copies from a scripted splice,
introduced in batch 31's commit) was found and repaired by restoring the
last clean copy (7efd75b) and consolidating the batch 29-32 entries.

Batch 33: `agent/bounded_response.py` ->
`hermes-agent::bounded_response` done (8 source-derived tests, gap
noted). The httpx response abstracts as a blocking chunk iterator owned
by the drain worker; the caller waits with the hard wall-clock deadline
(Condvar) and on timeout abandons the worker (mem::forget = upstream's
daemon thread), keeping partial bytes. Byte cap truncates mid-chunk;
invalid UTF-8 replaces; `read_error_body_or_default` returns None on
empty. Constants: 64KiB cap / 10s deadline.

Batch 34: `agent/markdown_tables.py` ->
`hermes-agent::markdown_tables` done (11 source-derived tests, gap
noted). CJK/wide-char table re-alignment via the `unicode-width` crate
(wcswidth analog, clamped non-negative); divider grammar with the 3-dash
minimum; conservative pass-through for non-tables and mid-stream
fragments; vertical key-value fallback (Column N default labels, thin
row dividers, word-wrap with hard-break) when the rebuilt table exceeds
available_width. The crate gains `unicode-width`.

Batch 34: `agent/markdown_tables.py` ->
`hermes-agent::markdown_tables` done (11 source-derived tests, gap
noted). CJK/wide-char table re-alignment via the `unicode-width` crate
(wcswidth analog, clamped non-negative); divider grammar with the 3-dash
minimum; conservative pass-through for non-tables and mid-stream
fragments; vertical key-value fallback (Column N default labels, thin
row dividers, word-wrap with hard-break) when the rebuilt table exceeds
available_width. The crate gains `unicode-width`.

Batch 34: `agent/markdown_tables.py` ->
`hermes-agent::markdown_tables` done (11 source-derived tests, gap
noted). CJK/wide-char table re-alignment via the `unicode-width` crate
(wcswidth analog, clamped non-negative); divider grammar with the 3-dash
minimum; conservative pass-through for non-tables and mid-stream
fragments; vertical key-value fallback (Column N default labels, thin
row dividers, word-wrap with hard-break) when the rebuilt table exceeds
available_width. The crate gains `unicode-width`.

Batch 35: `agent/verify/__init__.py` re-export surface done — the
package's parent entry flips to done (recipes/environment/runner all
ported; the mod.rs re-export list matches the upstream __init__ names,
pinned by a surface test). Verify package complete (parent done).

Batch 36: `agent/secret_sources/base.py` ->
`hermes-agent::secret_sources::base` done (9 source-derived tests, gap
noted). The `SecretSource` contract (fetch MUST NOT raise or prompt;
mapped/bulk precedence flags; protected_env_vars so a vault can't clobber
its own bootstrap credential), ErrorKind taxonomy, FetchResult, the
per-fetch environment view with token reset, `scrub_ansi` (unterminated
OSC arm that ansi_strip lacks), and `run_secret_cli` (allowlisted env +
NO_COLOR + stdin null + timeout error). PENDING: registry.py
orchestrator, bitwarden/onepassword/command backends, _cache.

Batch 37: `agent/secret_sources/registry.py` ->
`hermes-agent::secret_sources::registry` done (6 source-derived tests,
gap noted). Registration gates (invalid lowercase names, API-version
mismatch, shape, scheme collision across names, duplicate unless
replace); secrets.sources ordering with unknown-name warnings;
mapped-beats-bulk precedence; first-claim-wins with conflict warnings;
preserve_existing > env > override ladder; per-source wall-clock timeout
reporting TIMEOUT and continuing startup; profile aliasing
(FOO_<PROFILE> -> FOO, credential-suffix guard, supplied-directly
exemption). Bundled backends (bitwarden/onepassword/command) remain
PENDING, so the lazy builtin registration is a no-op.

Batch 38: `agent/secret_sources/command.py` ->
`hermes-agent::secret_sources::command` done (14 source-derived tests,
gap noted). Security model preserved line-for-line: the helper runs via
/bin/sh -c (user's own config trust level); the key travels ONLY as
HERMES_SECRET_KEY env data (never interpolated); 3s hard timeout with
whole-group SIGKILL; 1MiB output cap; stderr captured and DISCARDED with
structured fields only (code/signal) in any log; parse_secret_output's
exact-dotenv-match > multi-key-dump-None > bare-value cascade with the
base64-padding and cross-key misroute guards; CommandSource adapter with
NOT_CONFIGURED/Internal remediation hints. POSIX-only (Windows degrades
to an empty result with a warning).

Batch 39: `agent/secret_sources/_cache.py` ->
`hermes-agent::secret_sources::cache` done (11 source-derived tests, gap
noted). DiskCache: `<hermes_home>/cache/<basename>` JSON with
serialized-key matching, str-to-str coercion, atomic
staging-file + chmod-0600 + rename writes, cache-dir chmod 0700,
ttl<=0 disabling both cache layers symmetrically, idempotent clear;
explicit-clock read_at/write_at seams. rand + base64 already in the
crate's tree.

Batches 37-39 (session-log entries for 29-36 are in PLAN.md §7):

- Batch 37: `agent/secret_sources/registry.py` ->
`secret_sources::registry` done (6 tests). Registration gates, mapped-
beats-bulk precedence, first-claim-wins with conflict warnings,
preserve/override ladder, per-source wall-clock timeout, profile
aliasing (FOO_<PROFILE> -> FOO).
- Batch 38: `agent/secret_sources/command.py` ->
`secret_sources::command` done (14 tests). /bin/sh helper with
HERMES_SECRET_KEY-as-data, group-kill timeout, 1MiB cap, stderr
discarded with structured fields only; parse cascade with base64 and
cross-key misroute guards.
- Batch 39: `agent/secret_sources/_cache.py` ->
`secret_sources::cache` done (11 tests). Atomic 0600 staging writes,
cache-dir 0700, ttl<=0 disables both layers, key matching with
str-str coercion, explicit-clock read_at/write_at seams.

Batches 39-40 (completing the _cache substrate and the 1Password
backend): `_cache` -> `secret_sources::cache` done (11 tests: atomic
0600 staging writes, cache-dir 0700, ttl<=0 disables both layers,
explicit-clock seams) and `onepassword.py` ->
`secret_sources::onepassword` done (9 tests: reference validation,
pinned-binary discovery, live fake-op fetch with L1/L2 caching and
refs-fingerprint misses, per-reference failure warnings, op error
taxonomy, mapped adapter with override_existing default true and the
token env protected). PENDING siblings: bitwarden.py (1,048 LOC).

Batch 41: `agent/secret_sources/bitwarden.py` ->
`secret_sources::bitwarden` **partial** (7 source-derived tests, gap
noted). Ported: the color-eyre stderr summarizer (Location/Backtrace
cutoff, numbered-cause stripping, stripped-raw fallback), the bws error
taxonomy (incl. invalid_client/invalid_grant/400-bad-request ->
AUTH_FAILED), pipe-joined cache-key serialization, sha256 token
fingerprints, and the BitwardenSource adapter (bulk, scheme bws,
override_existing default TRUE, BWS_ACCESS_TOKEN protected,
NOT_CONFIGURED pre-flight arms). PENDING: find_bws/install_bws
(checksum-verified download + zip extraction), _run_bws_list +
fetch orchestration, and the HKDF+AESGCM encrypted last-good cache.

Bitwarden runtime follow-up (same partial): `_run_bws_list` +
`fetch_bitwarden_secrets` + `find_bws`/`hermes_bin_dir`/
`_platform_binary_name` ported with 8 more tests against a fake bws
binary. Live-fetch failure falls back to the STALE plaintext disk cache
only for NETWORK/TIMEOUT classifications (never AUTH/INTERNAL);
encrypted-cache tier PENDING. Cross-test L1 cache state was a real test
bug: the network test now uses its own token. The installer
(checksum-verified download + zip-slip-safe extraction) remains the
main PENDING piece.

Bitwarden installer follow-up (same partial): `platform_asset_name`
(target-triple grammar, Linux musl probe via ldd), `expected_sha256`
(sha256sum format), `sha256_file`, `pick_zip_member` (shortest path),
`safe_extract_member` (zip-slip refusal), `install_urls`, and
`install_bws_at` — checksum-verified staged install (chmod 0755 + atomic
rename) with the HTTPS layer abstracted behind an injectable
`Downloader` seam; existing-target short-circuits without download;
checksum mismatch aborts without installing. 7 installer tests. The
crate gains `zip`.

Bitwarden encrypted-cache follow-up (same partial): `_derive_encrypted_cache_key` (HKDF-SHA256, info `hermes-bws-encrypted-cache-v1`, 32-byte AES-256 key from the bootstrap token), `write_encrypted_disk_cache` (random 16B salt + 12B nonce, AES-256-GCM over compact JSON with the serialized cache key as AAD, 0600 staging + atomic rename, legacy plaintext file removed on success), and `read_encrypted_disk_cache` (version/key gates, str-str coercion, 0<=age<=max_stale window). 5 more tests. The crate gains `hkdf` + `aes-gcm`.

Batch 45: `agent/billing_links.py` -> `hermes-agent::billing_links` done
(6 source-derived tests, gap noted). The 14-provider billing-link table,
two-pass resolution (slug dict first, then base_url host), Nous in-app
routing bit with the portal fallback URL, unknown-provider
readable-label degradation without an invented URL, and to_dict.
`_nous_billing_url`'s hermes_cli import is PENDING with that surface
(falls back to the same constant).

`hermes-gateway` also gains `hermes-cli`, `hermes-constants`, `hermes-time`,
`serde_json`, `libc` — all still below the agent/tools layers.
`tools.close_terminal_tool` was skipped: blocked on the 2,937-LOC
`tools.process_registry`.

## Exact working-tree state

Session 4da **changed the ledger**: 99 done / 15 partial / 3,768 missing
tracked modules and 99 done / 15 partial / 989 missing production modules —
**3.71%** tracked and **13.06%** production strict completion — regenerated
against the pinned `b9aa928` worktree (commands above), with
`cargo build --workspace` and the serialized workspace run green at 1,666
tests / 6 ignored (the 6th is skill_provenance's intentional `ignore` doc
example).

## Next actions, in order

1. Keep taking the small-module batch. Same-shape candidates: the
   remaining small oracle-backed `tools/`/`gateway/`/`hermes_cli` leaves
   (`tools.close_terminal_tool`'s siblings). The big
   `tools.process_registry` (2,937 LOC) unblocks `tools.close_terminal_tool`
   and `tools.read_terminal_tool` callers when taken.
2. Continue the remaining `agent.auxiliary_client` request/response lifecycle
   and the deferred higher-layer seams (desktop/gateway emitter wiring,
   plugin registry).
3. Keep ownership disjoint; commit and publish each logical unit immediately.

## Current conversion ledger

99 done / 15 partial / 3,768 missing tracked (**2.55%** strict completion);
144 done / 22 partial / 936 missing production (**13.06%**). Regenerated via
`tools/inventory.sh` (pinned worktree) + `python3 tools/conversion_ledger.py`.

## Fidelity notes

- Session 4da (batch 39): the cache's is_fresh keeps upstream's strict
  `<` (a fetched_at exactly TTL old is stale) and ttl<=0 disables both
  layers. Explicit-clock read_at/write_at forms are the time.time patch
  seam; rand+base64 supply the mkstemp random staging suffix.
- Session 4da (batch 44): the encrypted cache uses HKDF-SHA256 with the
  bootstrap token as IKM, a random 16-byte salt, and
  hermes-bws-encrypted-cache-v1 as info (32-byte AES-256 key); AES-256-GCM
  over compact JSON with the serialized cache key as AAD; 0600 staging +
  atomic rename; a successful write removes the legacy plaintext file;
  reads gate on version/key and enforce 0<=age<=max_stale.
- Session 4da (batch 38): command source tests exercise real /bin/sh
  children (env-key-as-data proven by interpolating HERMES_SECRET_KEY in
  the helper). The unquote requires length >= 2 (a lone quote survives);
  whitespace-only quoted values are 'no value' (would 401 an
  Authorization header). PENDING siblings: bitwarden.py (1,048),
  onepassword.py (682), _cache.py (215).
- Session 4da (batch 37): the apply chain order is invalid-name ->
  protected -> claimed(conflict warning) -> preserve_existing ->
  env-without-override -> apply; profile aliasing additionally requires
  the alias not be supplied directly by any source and a credential-shaped
  suffix; per-source timeout uses a worker thread (TIMEOUT result,
  partials discarded).
- Session 4da (batch 41): the summarize rule strips ANSI FIRST (so a
  colorized 'Error:' line is not recognized as the skip-marker - faithful
  to upstream's replace-then-parse order); cause lines are numbered
  '0: ...' with the index stripped per line. Redaction-failure subtlety:
  the test fixture was corrected to the non-ANSI oracle input.
- Session 4da (batches 39-40): the onepassword disk key omits home_path
  (the file already lives under <home>/cache/), L1 keys fold it in; only
  complete error-free pulls are cached (a transient auth failure isn't
  frozen for the TTL); op_child_env is a 15-var allowlist + OP_SESSION_*
  + normalized OP_SERVICE_ACCOUNT_TOKEN + NO_COLOR; `--` terminates op
  option parsing before the reference.
- Session 4da (batch 36): run_secret_cli's env allowlist keeps only the
  named basics + allow_env + extra_env (never the full post-dotenv
  environ); timeout kill discards partial output and raises the
  actionable timed-out error; the ANSI regex deliberately also strips
  unterminated OSC sequences (not a superset of ansi_strip).
- Session 4da (batch 35): verify package complete. __init__ re-export
  surfaces are 'done' once every upstream-exported name resolves through
  the package root - a surface test pins the list.
- Session 4da (batch 34): markdown_tables tests cross-checked alignment
  with a minimal CJK=2-cells width helper; the vertical fallback emits
  row dividers only between (not before) body rows, and Column N labels
  apply only to empty HEADER cells (empty body values render as 'h2:').
- Session 4da (batch 33): the bounded read's deadline cannot be enforced
  between chunks (a stall mid-chunk never yields control), so the drain
  runs on a worker the caller abandons on timeout (mem::forget stands in
  for the upstream daemon thread). Callers own response close; the
  iterator's error path is panic-isolated like the upstream `except`.
- Session 4da (batch 34): markdown_tables tests cross-checked the
  alignment assertions with a minimal CJK=2-cells width helper; the
  vertical fallback only emits row dividers between (not before) body
  rows, and Column N labels apply only to empty HEADER cells.
- Session 4da (batch 34): markdown_tables tests cross-checked alignment
  with a minimal CJK=2-cells width helper; vertical fallback emits row
  dividers only between (not before) body rows, and Column N labels apply
  only to empty HEADER cells (empty body values render as 'h2:').
- Session 4da (batch 32): the scrubber's byte-slicing is char-boundary
  guarded (max_partial_suffix skips non-boundary splits, feed re-checks
  before slicing) so multi-byte content cannot panic mid-tag-hold-back.
  Priority semantics preserved: a closed pair at or before a boundary-open
  index wins; the earliest boundary-legal open per tag variant is enough.
- Session 4da (batch 32): think_scrubber's byte-slicing is char-boundary
  guarded (max_partial_suffix skips non-boundary splits; feed re-checks
  before slicing) so multi-byte content cannot panic mid-hold-back.
  Priority semantics: a closed pair at or before a boundary-open index
  wins; the earliest boundary-legal open per tag variant suffices.
  PLAN.md duplication lesson: scripted splices must assert match counts
  before writing.
- Session 4da (batch 31): close_terminal's seam drops the ProcessSession
  argument the upstream sink receives (the desktop handler keys off the
  session id); the required-process_id error fires before the sink
  lookup so CLI callers get the argument error; sink panics are caught
  and surfaced as the error string (the upstream except arm).
- Session 4da (batch 30): browser_dialog_tool completes with a seam rather
  than the full supervisor: set_supervisor_lookup injects the registry
  lookup; the missing-supervisor arm yields the desktop-only error object
  and responder errors pass through verbatim (catch_unwind not needed —
  the seam is typed).
- Session 4da (batch 29): mcp_dashboard_oauth's wait_for_callback wraps the
  stored callback error in the authorization-failed raise — including the
  payload-less did-not-include-code arm (stored as a callback_error, not
  a separate variant). mark_error is a no-op after approval. The crate
  gains thiserror.
- Session 4da (batch 28): ssl_guard validates in upstream's order (skip
  env → env vars → platform bundle) with the check ladder exists →
  is-file → substantial → loads-certificates; the "loads" leg counts PEM
  certificate blocks rather than x509-parsing (the Rust TLS stack owns
  real parsing at client construction). Path.expanduser mapped to $HOME
  prefix substitution.
- Session 4da (batch 27): native_flow is the security-critical cut of the
  dashboard_auth package — pop-before-PKCE (no retry), per-IP pending cap
  on the public pre-auth route, GC-before-lookup (so CodeExpired is
  effectively dead — CodeInvalid covers expiry), constant-time
  comparisons, and test-only reset. hermes-tools' sha2 dep was restored
  after an accidental removal.
- Session 4da (batch 22-cookies): the reader tries every name variant
  (most-strict first) because the reading request may not share the
  setting request's shape; deletions emit Max-Age=0 for every variant
  under the active Path. The transparent AT-rotation flow
  (`middleware._attempt_refresh`) remains PENDING with middleware.py.
- Session 4da (batch 25): the ws-ticket store is process-global — tests
  serialize behind a mutex (parallel resets were clearing each other's
  tickets). `expires_at < now` (not <=) means a ticket is valid AT its
  exact TTL second. The internal credential's constant-time compare keeps
  length/prefix leakage off mismatch errors.
- Session 4da (batch 24): authenticate_token's stack contract: unrecognised
  tokens are Ok(None) (fall through), ProviderError is remembered and
  only surfaces as 503 when NO provider accepts; empty token
  short-circuits without consulting providers. audit_log strips
  REDACTED_FIELDS at the kwarg boundary — values never reach
  serialization. The async verify_token is bridged with
  futures::executor::block_on in the sync seam.
- Session 4da (batch 23): prefix normalisation strips ALL trailing
  slashes (Python `.rstrip("/"))` and rejects `..`/`//`/quote/whitespace
  before the length budget; public_url uses a hard "no" on hostile
  characters rather than a soft parse — the caller must treat "" as
  "reconstruct from request", never "explicitly no public URL". The
  once-per-(source,value) warning dedup is preserved (resolve runs per
  request).
- Session 4da (batch 22): the auth trait's default
  `complete_password_login`/`verify_token` arms keep upstream's fail-loud
  contract (NotImplementedError, never silent acceptance); unrecognised
  tokens are Ok(None) — never errors — so the seam falls through providers
  in registration order; the registry's token plane fails closed (401)
  when no token provider is registered. `assert_protocol_compliance` can
  only dynamically pin the name/display_name attributes (Rust traits
  enforce the method set at compile time).
- Session 4da (batch 21): the verify runner's most important find: a
  post-spawn `setpgid` loses the race against the child's exec, and the
  subsequent `killpg` then signals the RUNNER's own group. Rust's
  `Command::process_group(0)` performs the setsid at spawn — the only
  correct analog of `start_new_session=True`. Start-phase tests must use
  distinct loopback ports when running in parallel.
- Session 4da (batch 20): verify recipes were red-checked against the
  Python oracle twice — `.PHONY: build` matches `_MAKE_TARGET_RE` (dots
  are in the class) and `_script_runner` gives pnpm/yarn bare `pnpm dev`/
  `yarn dev` while npm/bun use `run`. `Recipe::from_dict` keeps upstream's
  tolerant normalization: grok aliases, blank-entry dropping, 0<port<65536
  with numeric-string coercion, readiness path must start with "/".
- Session 4da (batch 19): default_soul's match is exact-equality over
  normalized text (not substring), so a scaffold with an extra sentence —
  even more comment text — is user content. Tests live in-crate this once
  (cfg(test)); the workspace convention remains tests/parity_*.rs for new
  modules.
- Session 4da (batch 18): secret_prompt's `\b \b` erase sequence is
  written per real erase (Python writes the same three bytes); the
  getpass fallback in non-TTY environments reads plainly (no /dev/tty
  redirect — documented divergence). cli_output's prompt returns "" on
  EOF, NOT the default (upstream's except arm is a bare `return ""`,
  distinct from the empty-input arm that takes the default).
- Session 4da (batch 17): sqlite_util's duplicate-column swallow is
  string-matched on "duplicate column name" (case-insensitive) per the
  upstream `str(exc).lower()` check; every other OperationalError
  re-raises. write_txn's guarded rollback swallows only the rollback
  failure — the body's original error always surfaces. The live
  two-connection IMMEDIATE serialization test pins the writer-wins
  contract without sleeping.
- Session 4da (batch 16): `should_use_color` uses `var_os().is_some()` for
  NO_COLOR (Python `is not None` — an empty NO_COLOR still disables, per
  no-color.org) and `std::io::IsTerminal` for the TTY check. Note the
  percentage nuance: done-count gains on the tracked denominator moved
  tracked % (2.96→2.99) while production % dipped (10.70→10.52) because
  the module counts changed denominators differently — production % is
  116/1103.
- Session 4da (batch 15): the greedy session-key parse reproduces
  Python's `^approve:(.+):(allow-once|allow-always|deny)$` — the key runs
  to the LAST valid decision suffix, so keys containing colons (and even
  the word "allow-once") parse; `.+` still demands at least one char.
  `operator_openid` mirrors Python `or`-falsiness (first non-empty of
  group_member/user/resolver).
- Session 4da (batch 14): `safe_resource_attributes` rejects values that
  redaction would CHANGE (a defense-in-depth arm unique to this plane —
  an e-mail-shaped label passes the grammar but must not export verbatim).
  `_supervision_mode` env precedence: INVOCATION_ID → S6_* →
  container/.dockerenv → LAUNCHD_SOCKET → manual. `gateway_health`'s
  `safe_instance_id` went pub for this module (upstream imports the
  private helper across the module boundary the same way).
- Session 4da (batch 13): the OTLP partial keeps the module's security
  notes: header *names* in config, values read from env at export time
  and never logged; the `event_filter` plane-scoping seam becomes the
  sink contract; `OtlpUnavailable` survives as the no-sink error shape.
- Session 4da (batch 12): upstream's `gateway.status` try/except fallbacks
  are the ported code paths (no gateway.status module exists yet);
  `_safe_profile` falls back to "default" and `_safe_version` to
  `hermes_cli::VERSION`; `_base_attrs` never exports the profile, so the
  parameter is dropped. `GatewayDiagnosticLogHandler` becomes the
  per-record `diagnostic_event_for_log` function. Transition tests taught
  the singleton-mutex lesson (parallel tests closing the shared emitter
  race dispatch).
- Session 4da (batch 11): redaction is defense-in-depth stacking — the
  hermes-logging force-redactor masks tokens FIRST (possibly to partial
  masks the shape regexes then pass through), so the egress contract is
  "the raw secret never survives", not "the shape regex always fires".
  `cron_health` project/emit parity pins the branch-order nuance that an
  unknown non-empty source becomes "external" while the literal "unknown"
  stays.
- Session 4da (batch 10): monitoring crossed the 10% production mark.
  `emit`'s setdefault runs twice by design (once per payload insert like
  upstream's emit/put pair); subscriber identity dedup uses `Arc::ptr_eq`
  (Python uses list identity); the OTLP exporter plane itself remains
  PENDING with `agent/monitoring/otlp_exporter.py` and siblings.
- Session 4da (batch 9): the in-process `run` test must record the test
  process's *real* parent (`libc::getppid()`), not its own pid — the
  watchdog compares against the live getppid, so a self-referential record
  reads as instantly orphaned (that's the live-kill test's premise, not a
  defect). Signal handlers run async-signal-safely (killpg + _exit only).
- Session 4da (batch 8): qqbot `decrypt_secret` maps Python's raising
  `AESGCM.decrypt` to `Result<String, String>` (upstream lets the
  exception propagate; the Rust error strings carry no secrets). The
  User-Agent's Python slot is `unknown` by design — the pinned grammar
  lives in `build_user_agent_with`. `coerce_list` renders JSON booleans
  as Python `str()` would (`True`/`False`).
- Session 4da (batch 7): `trajectory` timestamps use naive local
  microsecond ISO (Python `datetime.now().isoformat()` — no offset
  suffix); the fail-open IO arm logs via the `log` facade and returns
  None, and the default-filename decision is a public pure function so
  the tests pin it without touching the process cwd.
- Session 4da (batch 6): the reactions regex is a verbatim arm-for-arm pin
  of the upstream lexicon; boundary assertions were checked against the
  Python oracle first (`goodbot` DOES fire — `good\s*bot` has no boundary
  between the words; `\bty\b` keeps "empty" and "tyty" silent). The httpx
  limits port keeps the env-var grammar byte-comparable (blank/non-numeric/
  non-positive → default) with the httpx-specific None arm documented away.
- Session 4da (batch 5): `timefmt.relative_time` reproduces Python falsiness
  (`None`/`0.0` → "?"), truncating `int(delta/x)` divisions, the
  `delta < 60` arm that makes future timestamps read "just now", and the
  local-zone `%Y-%m-%d` date branch. `IterationBudget.max_total` stays a
  plain public field (upstream mutates it), so `remaining`'s
  `max(0, ...)` clamp is observable.
- Session 4da (batch 4): `open_preview_tool`'s `_normalize_target` pins both
  upstream regexes verbatim (LazyLock) — re.I matters (LOCALHOST:3000 gains
  http://), `\w` stays unicode-aware, and the strip order is
  whitespace → backticks → whitespace, matching
  `raw.strip().strip("`").strip()`. The emitter except arm is reproduced
  via catch_unwind with the panic message surfaced, per read_preview_tool.
- Session 4da (batch 3): `readiness` opens the state db read-only via URI
  with a 1s busy timeout so probes never compete with writers; empty YAML
  documents count as defaults (`safe_load` → None); `disk_usage` reproduces
  `shutil.disk_usage` (`free` = `f_bavail`); error details carry short type
  labels only, never messages.
- Session 4da (batch 2): `session_stall` idle resolution mirrors Python
  `float()` semantics — numeric strings parse, non-numeric/non-finite
  `seconds_since_activity` falls through to `last_activity_at`/`_ts`,
  negative values clamp to 0.0, and an empty mapping is falsy like
  `if not activity`. `skill_provenance`'s set-boundary coercion is
  `origin or "foreground"`.
- Session 4da: `code_skew._short` keeps Python's `sha or fingerprint`
  empty-string fallback; `record_boot_fingerprint` is idempotent like the
  upstream module global. `cgroup_cleanup` per-PID kills map
  ProcessLookupError/PermissionError to skip-without-counting;
  `parse_own_cgroup_path` reproduces the `^0::(.+)$` anchor exactly (bare
  `0::` is not a match, whitespace-only is). `rich_sent_store`'s
  `record` guards run before any disk touch, and `lookup`'s non-dict
  document arm reproduces upstream's `AttributeError`. `git_revision`
  uses lossy reads for upstream's `errors="replace"` and canonicalizes
  non-strictly like `Path.resolve()`.
- Layering: `hermes-gateway` importing `hermes-cli` matches upstream's
  `gateway.code_skew` importing `hermes_cli.main`; both stay below the
  agent/tools layers.

## Verification evidence

- `/home/mustbearnold/.cargo/bin/cargo build --workspace` — green.
- `cargo test -p hermes-gateway -p hermes-cli -p hermes-tools -p
  hermes-agent` — 358 new tests green.
- Serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace --
  --test-threads=1` — 1,666 passed, 0 failed, 6 intentional ignores.
- `cargo clippy -p hermes-gateway -p hermes-cli --all-targets` — clean on
  all new code.
- `rustfmt --edition 2021 --check` — clean on all changed files.
- `git diff --check` — clean.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```

## Review checkpoint — expert board R1 (2026-09-14, corrected after pin regen)

Five-seat isolated board reviewed the working tree. Round 1: S1
CONDITIONAL, S2 CONDITIONAL, S3 REJECT, S4 REJECT, S5 REJECT.
Adjudication against live sources CONFIRMED the no-loop finding (no
`run_agent`/`AIAgent`/conversation loop — P2 exit unmet), the
`mustbearnold` path staleness, and `main` ahead 45 + untracked IDEA.md
(since removed: 1-line content-free stub).

CORRECTION to the R1 report: regenerating against the true pin
(`HERMES_UPSTREAM=/tmp/hermes-upstream-b9aa928`) yields **3,882 tracked
modules / 1,103 prod / 843,792 prod LOC** — exactly PLAN §1 and GATES
G1. The 4,740/1,229 figures came from an inventory generated against
upstream HEAD (`b51c055a`), not the pin. S4 sub-blockers on PLAN/GATES
counts are therefore REFUTED; the true defect was the regen protocol
(not a pinned worktree), now fixed: `git config hermes.upstream`
points at the pinned worktree, GATES G4 asserts the pin SHA before
refreshing, and the hook resolves via that config. Current ledger:
**3.74% all-tracked (145/3882), 13.15% prod (145/1103)**, 22 partial.
The 4 `port_status.json` keys absent from the inventory
(`agent.monitoring`, `agent.secret_sources`, `agent.verify`,
`hermes_cli.dashboard_auth`) are package-rollup keys with no direct
`.py` counterpart — excluded from strict counts by design, not drift.

Validation this session: `cargo build --workspace` green;
`cargo test --workspace -- --test-threads=1` green (exit 0, serialized;
must run with a clean env — `env -u HERMES_REAL_HOME -u HERMES_HOME
-u TERMINAL_MODAL_MODE -u TERMINAL_MODAL_IMAGE` — because
`HERMES_REAL_HOME` outranks `HOME` in `get_real_home` trust order and
otherwise breaks the `parity_tool_backend_helpers` HOME-override tests
with a mutex-poison cascade: 1 root failure + 14 `PoisonError`
follow-ons); `git diff --check` clean. Pre-existing nit left open:
`cargo fmt --check` wants `markdown_tables` module-ordering in
`hermes-agent/src/lib.rs:13`.

Next dependency-safe unit: `run_agent` (+ `AIAgent` turn loop on a
model stub).

Commit `22228d6` pushed: `d138528..22228d6 main -> main`, mirror in
sync. Note: local `git config hermes.upstream` points at the pinned
worktree (removed after commit) — recreate per the recipe above before
the next source commit, or GATES G4 fails loud. Pinned worktree
removed.

## R2 checkpoint (2026-09-14) — board re-verification + closures

R2 rule: verify-by-quote; new blockers only for revision defects.
S2 CONDITIONAL (items settled PASS, no new blockers; `crates/` untouched
by the revision). S3 REJECT sustained (4/4 FAILs quoted — no loop, no
transport, no executor, no provider entrypoint; ledger numbers PASS:
145/3882 = 3.74%, 145/1103 = 13.15%). S4: all 7 R1 items PASS
(conceded the count inversion with live numbers) + 1 NEW blocker,
independently confirmed by S5: dead `hermes.upstream` made GATES G4
unexecutable. S1 R2 output noise twice running ("8784") —
operator-adjudicated CONDITIONAL: exception note, deferred-crate note,
pkg drift all absent (FAIL), build/tests PASS (run by operator).
S5 REJECT (4 FAILs quoted, item 5 PASS conceded with clean
`## main...origin/main`).

Closures landed below this checkpoint: PLAN §3 gateway→cli exception +
P4/P5-deferred crate note; hermes-tools/toolsets workspace package
keys (redundant libc dev-dep dropped — watchdog suite 9/9 green);
pre-commit pin guard (refuses refresh off-pin); GATES G4 self-ensuring
pinned worktree + SHA assert (executed: recreated worktree, refreshed,
timestamp-only diff as designed); dead `hermes.upstream` config unset.

## Session 5b — run_agent section 2 (2026-09-14)

Session-establishment helpers + stream error type (TDD):
`launch_cwd_for_session`, `session_source_for_agent` (gateway context
as explicit arg — unported layer seam), `StreamErrorEvent`
(thiserror, SDK-shaped body). Oracles: `test_session_source`
(full mirror), `test_codex_xai_oauth_recovery` body assertions
(harness pending loop); `_launch_cwd` source-derived. Caught live:
rebinding an env-guard deadlocks (std Mutex non-reentrant) — scope
each phase. Next: TBD from pin map.

## Session 5a — run_agent section 1 (2026-09-14)

`hermes-agent::run_agent` opened (TDD, now 48 parity tests green):
scaffolding flags/table, truthiness filter, worker/marker/version
consts, Qwen + RouterMint header builders, traversal-safe session
filename with oracle-exact digests, pool-recovery predicate.
Validation: `cargo build --workspace` + `cargo test -p hermes-agent
--test parity_run_agent` green; full serial clean-env suite re-run
before commit. Next slice: TBD from the pin map (the ~1334-1613
provider predicates are `AIAgent` self-methods — board S4 correction).

## R5 checkpoint (2026-09-14) — section-2 board + closures

5-seat R1 on f919372: S1 CONDITIONAL (1 real bug), S2 CONDITIONAL
(2 test gaps), S3 BUILD, S4 CONDITIONAL (3 docs-only), S5 REJECT.
Adjudicated CONFIRMED + fixed: explicit-empty context MUST mask env
(`get_session_env` returns set-"" with no fallback — verified at
`gateway/session_context.py:363-386`; was leaking stale env); Some
ctor arms, TERMINAL_ENV ""/"   " arms, null/blank masking tests;
module-doc range/tiers/first-section; lossy-cwd + string-domain notes.
Accepted standing: u16 precedent, readonly-availability, fresh-now
contract, fallback-gate wiring (loop slice). S1/S5 items closed or
filed; S5's TOCTOU/truthiness attacks conceded by the seat itself.

## R4 checkpoint (2026-09-14) — pool-slice board + closures

5-seat R1 on c100b71: S1 CONDITIONAL (arms match; 2 standing
conditions), S2 BUILD, S3 BUILD, S4 CONDITIONAL (3 docs-only),
S5 REJECT (4 attacks, 2 conceded). Adjudicated: S4 docs fixed (oracle
names, 5-file sentence, stdlib+pool reword, count touch-up); S5's
sibling-method fork CONFIRMED at pin (~6007, no len gate; live path
`conversation_loop.py:4627` uses the module fn) and recorded in the
predicate docs as a loop-slice constraint; readonly-availability and
fresh-`now` caller contract documented (zero callers exist yet —
standing conditions, not slice defects); fallback-gate wiring belongs
to the loop slice. S1 upgrades to BUILD on those notes; S5's remaining
items are pool-module/loop-slice debt, correctly filed elsewhere.

## R3 checkpoint (2026-09-14) — section-1 board + closures

5-seat R1 on f27acce: S1 CONDITIONAL (2 blockers), S2 BUILD, S3
CONDITIONAL (5 items), S4 BUILD, S5 REJECT (5 kill-attempts). Zero
noise this round. Adjudicated CONFIRMED + fixed: `\w`
Join_Control/Mn divergence → calibrated class `[\p{L}\p{N}_]`
(char-by-char vs live oracle); `trim` vs `strip` (`\x1c-\x1f`) →
`trim_py_spaces`; unconditional `aarch64→arm64` → OS-gated table
(+ Windows AMD64/ARM64, system `.lower()`); wrapper smoke test;
null/number truthiness asserts; flag-order source label;
surrogate/coercion contracts documented. S5's kill-verdict flips:
implementation output byte-matches the oracle on every vector.
Operator-verified R2 (tests green + transit-proof oracle digests).

Transit lesson: zero-width chars (ZWNJ etc.) are eaten in tool-call
text — never paste them literally. Use `\u{...}` escapes, verify with
hexdump, and compute oracle digests via `chr()` construction.

## Session 5c — run_agent section 3 (2026-09-14)

Provider/URL predicates + token-cap helpers (TDD, explicit-argument
forms of pin ~1334-1680 methods): responses-API routing (Copilot arm
takes upstream's own except-fallback), 7 URL/provider predicates,
codex hang hint (Python-quote interpolation), max-tokens key,
output-cap `int()` coercion table. Oracles: direct-URL detection,
hang-hint positives, max-tokens/routing arms; self-reading no-arg
forms deferred to the loop slice. Next: TBD from pin map.

## R6 checkpoint (2026-09-14) — section-3 board + closures

5-seat R1 on 247a3e2: S1 CONDITIONAL, S2 CONDITIONAL (3), S3 BUILD,
S4 BUILD, S5 REJECT. Adjudicated: Copilot rule fully ported from
hermes_cli/models.py (pure re — kills S1-conditional + S5-1, no layer
violation); hang-regex kill REFUTED by hexdump (byte-identical;
adversarial tested a display-doubled phantom); hostname derived from
base URL (kills split-brain); Python repr() + underscore-int parity
added with oracle vectors; S2-1/S2-2 refuted (unreachable domain /
would-be divergence); float-bignum documented out-of-domain.
Operator-verified R2 = green suite + oracle recomputation.

## Session 5d — run_agent section 4 (2026-09-14)

Error-text helpers (TDD): entitlement classifier + WKE disambiguator,
xAI hint decorator, recursive detail coercer (sorted-JSON fallback),
key masker, message cleaner. Oracles: Fix-D parametrize + status gate,
mask-key intent (pin key redacted — substituted same-shape key, noted).
`_summarize_api_error` deferred to section 5 (exception-shape design).
Next: `_summarize_api_error` + `_flatten_exception_chain` slice.

## R7 checkpoint (2026-09-14) — section-4 board + closures

5-seat R1 on fe17b1b: S1/S2 CONDITIONAL, S3 CONDITIONAL, S4
CONDITIONAL, S5 REJECT. Adjudicated CONFIRMED + fixed: dumps
separators (`, `/`: ` live-verified), haystack list-join over-match
(now Python-repr containers via py_repr/py_float_repr), extended
decorate pins, honest labels, module-doc §4 range. Refuted or filed:
call-site audits (zero callers — standing), inf-saturation
(unreachable), u16 precedent, fallback-gate wiring (loop slice).
Operator-verified R2 = green suite + oracle recomputation.

## Session 5e — run_agent section 5 (2026-09-14)

`_summarize_api_error` via `ApiErrorShape` (settled verdicts at raise
site): ValueError/HTML/Gemini/body/response-text/fallback arms, redact
wired with upstream defaults. Oracles: summarize-empty-body (2),
challenge collapse; rest source-derived. `_flatten_exception_chain` is
an `agent.stream_diag` forwarder — different unit, not this slice.
Next: TBD from pin map (stream-diagnostic/self-state methods need the
agent struct decision).

## R8 checkpoint (2026-09-14) — section-5 board + closures

5-seat R1 on 65242d2: S1/S2/S3/S4 CONDITIONAL, S5 REJECT. Adjudicated
CONFIRMED + fixed: dumps separators, haystack list-join (py_repr
containers), whitespace-title placeholder drop, NBSP Ray class,
is_value_error doc misattribution, S3/S4 label+header fixes,
representative challenge fixture, empty-body shape, S5's executed
routing vectors as tests. Refuted: S2's DNS arm (no such file or
marker at pin — 3rd HEAD-vs-pin catch), inf-saturation (unreachable),
u16 precedent. Standing: zero-caller contracts, redact/parse
equivalence (noted). Operator-verified R2 = green suite.

Post-R8 correction: the adversarial seat's "empty error.message falls
through to top message" vector was mis-executed — live oracle shows a
dict `error` with falsy `message` skips the whole body arm (no second
lookup); only a non-dict `error` routes to the top `message`. Test
corrected to the true semantics; implementation was already faithful.
Reviewers are fallible — live source wins, including against seats.

## Session 5f — run_agent section 6 (2026-09-14)

Response-ending heuristic + Ollama-GLM detector (TDD, both
source-derived — no oracle files at pin): punct set transcribed
verbatim incl backslash/CJK, gate order preserved (caught 2 of my own
vectors wrong on re-read before running). Forwarder siblings and
think-block consumers deferred to their owner modules. Agent-struct
scaffold still deferred — free-function slices continue until shared
mutable state forces it.
