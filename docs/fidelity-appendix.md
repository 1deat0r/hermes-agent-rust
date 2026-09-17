# Fidelity appendix (exempt from HANDOFF cap)

> Mined from archived HANDOFF history per doc-efficiency r2 (board-gated).
> Live parity semantics that must survive history archival. Each bullet stands
> unless its module is re-ported — then the code `PARITY:` comment wins.

## Fidelity notes (sessions 4da/5a-5f, pin b9aa928)

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
