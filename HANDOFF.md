# Hermes Agent Rust — Next-session handoff

Date: 2026-08-31 (Pacific/Auckland), session 4da.

## Resume point

Repository: `/run/media/mustbearnold/Projects/AI Agents/Hermes-Agent-Rust`

Pinned upstream commit: `b9aa928`. The checkout actually used and validated is
`/run/media/mustbearnold/Projects/Research/hermes-agent-repo` (AGENTS.md and
`tools/inventory.sh` were corrected to this path this session). **Upstream
HEAD has advanced far past the pin** — always regenerate the inventory against
a pinned worktree:

```bash
git -C /run/media/mustbearnold/Projects/Research/hermes-agent-repo \
    worktree add --detach /tmp/hermes-upstream-b9aa928 b9aa928
HERMES_UPSTREAM=/tmp/hermes-upstream-b9aa928 bash tools/inventory.sh
python3 tools/conversion_ledger.py
git -C /run/media/mustbearnold/Projects/Research/hermes-agent-repo \
    worktree remove /tmp/hermes-upstream-b9aa928
```

Current branch/HEAD: `main` carries this session's four units committed with
this documentation checkpoint. Previous: `d138528` (4d9's
`browser_camofox_state`/`focus_pane_tool`/`cwd_placeholder`), `e6079b9`,
`a06d8fb`, `aedb1f2`, `2468dc7`.

## What landed this session (4da)

Forty-four units across four crates, all red-first, 268 new parity tests:

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

`hermes-gateway` also gains `hermes-cli`, `hermes-constants`, `hermes-time`,
`serde_json`, `libc` — all still below the agent/tools layers.
`tools.close_terminal_tool` was skipped: blocked on the 2,937-LOC
`tools.process_registry`.

## Exact working-tree state

Session 4da **changed the ledger**: 99 done / 15 partial / 3,768 missing
tracked modules and 99 done / 15 partial / 989 missing production modules —
**3.40%** tracked and **11.97%** production strict completion — regenerated
against the pinned `b9aa928` worktree (commands above), with
`cargo build --workspace` and the serialized workspace run green at 1,594
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
132 done / 21 partial / 950 missing production (**11.97%**). Regenerated via
`tools/inventory.sh` (pinned worktree) + `python3 tools/conversion_ledger.py`.

## Fidelity notes

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
  hermes-agent` — 268 new tests green.
- Serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace --
  --test-threads=1` — 1,594 passed, 0 failed, 6 intentional ignores.
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
