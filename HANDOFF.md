# Hermes Agent Rust — Next-session handoff

Date: 2026-09-17 (Pacific/Auckland), doc-efficiency implementation.

## Binding rulings (hot — full records in archive)

1. Ledger honesty: AST re-proof must be comment-aware; "identical" claims
   carry classifier output, not prose (R1/S3-B1).
2. Cancellation never masks as 503: enum seams refuse the fold (R1/S3-B3).
3. Test suites pin behavior (~20 assertions, not test counts) (R1/S3-B4
   refuted).
4. Doc reads: JSON summaries only, caps 20/50 rows; never whole ledger or
   inventory (doc-efficiency r2).
5. Dual PLAN §7 + HANDOFF log stays — redundancy is the splice detector.

## Next unit (see § Next actions below for the full queue)

Wave B sweep per `docs/long-horizon-101.md` waves A–D; current ledger +
validation commands recorded after each batch section below.

## Tree state

Branch `main` @ `a7ad46e`, tree clean (update per commit). Pinned
worktree: `.upstream-pin/5d59366` (persistent disk; never /tmp).

## Resume point

Repository: `/run/media/its1deat0r/Projects/AI Agents/Hermes-Agent-Rust`

Pinned upstream commit: **`5d59366`**
(`5d59366010640c1d6b8f170d8a4ee109db2bbdef` — latest `origin/main` as of
2026-09-16 NZ, `feat(plugin-catalog): add 66 community plugins`). The previous
pin `b9aa928` is superseded. Always regenerate the inventory against the
pinned worktree, never HEAD:

```bash
git -C /run/media/its1deat0r/Projects/Research/hermes-agent-repo \
    worktree add --detach .upstream-pin/5d59366 5d59366
HERMES_UPSTREAM=.upstream-pin/5d59366 bash tools/inventory.sh
python3 tools/conversion_ledger.py
git -C /run/media/its1deat0r/Projects/Research/hermes-agent-repo \
    worktree remove .upstream-pin/5d59366
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

- New pinned worktree `.upstream-pin/5d59366` at `5d59366`.
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
`docs/archive/board-review-r1.md`. Long-horizon goal: `docs/long-horizon-101.md`. Validation at gate: 1,789 passed / 0 failed
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

## EOF batch done (2026-09-16)

secret_prompt EOF-fold + 4 recerts.
**Ledger: 126/71/8698 (1.42% tracked, 3.62% prod).** Serial 1,879/0.

## SQLite unit done (2026-09-16)

open_db + transaction ported; 3 recerts.
**Ledger: 130/67/8698 (1.46% tracked, 3.73% prod).** Serial 1,884/0.

## Cache batch done (2026-09-16)

MCP TTL expiry ported; 3 recerts.
**Ledger: 134/63/8698 (1.51% tracked, 3.85% prod).** Serial 1,886/0.

## Patterns batch done (2026-09-16)

_SECRET_VAR ported; 3 refactors verified.
**Ledger: 138/59/8698 (1.55% tracked, 3.96% prod).** Serial 1,887/0.

## Scrubber batch done (2026-09-16)

THINK tag tables ported; 4 recerts.
**Ledger: 143/54/8698 (1.61% tracked, 4.11% prod).** Serial 1,888/0.

## Cron batch + pin infra done (2026-09-16)

Outcomes set + delegation ported; 4 recerts. Pinned worktree now
persistent (`.upstream-pin/5d59366`; /tmp is tmpfs).
**Ledger: 148/49/8698 (1.66% tracked, 4.25% prod).** Serial 1,889/0.

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

## JEV optimization pass done (2026-09-18)

Jev = TypeSafe System One decision model (2026-09-15). No
TYPESAFE_API_KEY in env → pattern-applied locally (measured state,
atomic noul/score decisions, confidence-gated). Landed, uncommitted:
13 manifests (8 deps centralized, 49 pins → workspace=true, dev
debug=1, release strip=true), reasoning.rs regex hoist, fmt sweep
(126 files, G3 `cargo fmt --check` now exits 0). Validation:
`cargo build --workspace` green (0 errors),
`cargo test --workspace -- --test-threads=1` green — 211 suites,
1,889 passed, 0 failed; metadata no-deps + full graph OK, Cargo.lock
untouched, `git diff --check` clean.
**Ledger: no status change — 148/49/8698 (1.66% tracked, 4.25% prod).**
Pre-existing drift noted (not mine): tail batch below says 77/120 but
live inventory.json + port_status.json + CONVERSION-LEDGER.md agree on
148/49. Next: optional `cargo clean`; set TYPESAFE_API_KEY for
live-Jev decision calls.

## Wave B6 audit done (2026-09-19)

`hermes_cli.dashboard_auth.audit` partial → done @ 5d59366
(line-by-line identical; compat no-op shim unported + noted; 2 oracle
tests added). File serial: 13 passed / 0 failed (`cargo test
-p hermes-cli --test parity_dashboard_auth_audit_token --
--test-threads=1`); fmt + diff-check clean. KNOWN pre-existing flake:
token_auth half races ~1-in-3 in parallel mode (reproduced at HEAD,
unfixed — out of scope).
**Ledger: 149/48/8698 (1.68% tracked, 4.28% prod).**
Next: Wave B6 remainder in smallest-LOC order — `token_auth` (96),
`ws_tickets` (97), `registry` (125), `prefix` (130) re-certs; then the
missing HTTP siblings (middleware/routes/login_page/request_utils/
refresh_singleflight) to close `dashboard_auth.__init__`.

## tools.credential_files done (2026-09-21)

Partial → done @ 5d59366 (366 LOC; per-home cache + mounts +
exclusions + ssh mapping, 33/33).
**Ledger: 181/21/8693 (2.03% tracked, 5.20% prod).**

## tools.tool_result_storage done (2026-09-21)

Partial → done @ 5d59366 (321 LOC; spillover layer + verify +
SHA-256 + Recovery, 26/26).
**Ledger: 180/22/8693 (2.02% tracked, 5.17% prod).**

## tools.clarify_tool done (2026-09-21)

Partial → done @ 5d59366 (311 LOC; batch system + labels +
sentinel + schema, 25/25).
**Ledger: 179/23/8693 (2.01% tracked, 5.14% prod).**

## tools.todo_tool done (2026-09-21)

Partial → done @ 5d59366 (284 LOC; full re-port with parents +
revision + todo_list, 34/34).
**Ledger: 178/24/8693 (2.00% tracked, 5.11% prod).**

## providers root package CLOSED (2026-09-21)

base (176) → __init__ (177): registry fallback, routed rejects,
installed-dir scan, 3 oracle tests, 11/11.
**Ledger: 177/25/8693 (1.99% tracked, 5.08% prod).**
Next: next Wave B partial in smallest-LOC order.

## providers.base done (2026-09-21)

Partial → done @ 5d59366 (344 LOC; endpoint order fixed per
oracle, 10/10).
**Ledger: 176/26/8693 (1.98% tracked, 5.06% prod).**

## agent.monitoring package CLOSED (2026-09-21)

otlp_exporter (173) → gateway_health (174) → gateway_health_export
(175): all 3 siblings done @ 5d59366. Export runtime with ordered
shutdown, 5 oracle tests, 14/14.
**Ledger: 175/27/8693 (1.97% tracked, 5.03% prod).**
Next: next Wave B partial in smallest-LOC order.

## agent.monitoring.gateway_health done (2026-09-21)

Partial → done @ 5d59366 (352 LOC; falsy/truncation/truthiness
fixes, fail-open emit, 3 oracle tests, 14/14).
**Ledger: 174/28/8693 (1.96% tracked, 5.00% prod).**

## agent.monitoring.otlp_exporter done (2026-09-21)

Partial → done @ 5d59366 (283 LOC; panic-count fix, signal +
streamer lifecycle, 4 oracle tests, 11/11).
**Ledger: 173/29/8693 (1.94% tracked, 4.97% prod).**

## agent.secret_sources package CLOSED (2026-09-21)

onepassword (169) → command (170) → registry (171) → bitwarden
(172): all 5 siblings done @ 5d59366 (base landed earlier).
bitwarden: shared-engine classifier, AUTH enrichment, apply_*
port, symlink-aware zip-slip, L1-clearing clear_caches, 4 oracle
tests, 15/15.
**Ledger: 172/30/8693 (1.93% tracked, 4.94% prod).**
Next: next Wave B partial in smallest-LOC order.

## agent.secret_sources.registry done (2026-09-21)

Partial → done @ 5d59366 (419 LOC; scoped port + 3 bug fixes,
3 oracle tests, 9/9; 62 green package-wide).
**Ledger: 171/31/8693 (1.92% tracked, 4.91% prod).**

## agent.secret_sources.command done (2026-09-21)

Partial → done @ 5d59366 (383 LOC; env-scope leak fixed, buffer
split, apply_* port, 2 oracle tests, 16/16).
**Ledger: 170/32/8693 (1.91% tracked, 4.88% prod).**

## agent.secret_sources.onepassword done (2026-09-21)

Partial → done @ 5d59366 (360 LOC; shared-engine classifier,
hints migration, apply_* port, 2 oracle tests, 11/11).
**Ledger: 169/33/8693 (1.90% tracked, 4.85% prod).**

## tools.tool_backend_helpers done (2026-09-21)

Partial → done @ 5d59366 (259 LOC; selection subsystem restored,
whitespace credential fix, 5 oracle tests, 45/45).
**Ledger: 168/34/8693 (1.89% tracked, 4.83% prod).**

NOTE (infra): the Research upstream checkout had been externally
deleted mid-session (pre-commit hook refused with "not at the pinned
5d59366"). Recovered per recipe: re-cloned NousResearch/hermes-agent,
fetched the pin SHA, byte-verified the fresh tree against the intact
`.upstream-pin` oracle files (all identical — session work was sound),
re-anchored the canonical worktree. Commit `211a0fe`, pushed,
mirror-identical.

## tools.env_probe done (2026-09-21)

Partial → done @ 5d59366 (255 LOC; caller-context backend +
cache-bypassing remote short-circuit, 1 oracle test, 14/14).
**Ledger: 167/35/8693 (1.88% tracked, 4.80% prod).**

## agent.verify.runner done (2026-09-21)

Partial → done @ 5d59366 (255 LOC; compose guard restored, stderr
wedge fixed, 3 oracle tests, 12/12).
**Ledger: 166/36/8693 (1.87% tracked, 4.77% prod).**

## agent.secret_sources.base done (2026-09-21)

Partial → done @ 5d59366 (255 LOC; 7 contract pieces restored,
backends migrated, 6 oracle tests, 15/15 + 43 backend green).
**Ledger: 165/37/8693 (1.85% tracked, 4.74% prod).**

## tools.file_state done (2026-09-21)

Partial → done @ 5d59366 (254 LOC; real lock_path, forget_task,
guard_disabled, IndexMap eviction, exact wording; 4 oracle tests
incl. contention proof, 14/14 ×3 stable).
**Ledger: 164/38/8693 (1.84% tracked, 4.72% prod).**

## Git + GitHub fully synced (2026-09-21)

Genius-agent pass: 32 commits pushed (`72efd57..4357aa0`),
ahead 0/behind 0, tree SHAs identical (`00c3ef6…`), description
synced, stale `local-sha-sequence-20260824` archived as tag + deleted.
Gates `.unlazy/git-hygiene/GATES.md` 5/5 met. Only `main` remains.
**Ledger: 163/39/8693 (1.83% tracked, 4.69% prod).**

## tools.mcp_dashboard_oauth done (2026-09-21)

Partial → done @ 5d59366 (140 LOC; iss carry-through + parse_qs-exact
state parse, 3 oracle tests, 14/14).
**Ledger: 163/39/8693 (1.83% tracked, 4.69% prod).**

## dashboard_auth package CLOSED (2026-09-21)

Unit 1: `token_auth` partial → done (96 LOC, red-green `catch_unwind`,
14/14). Unit 2: `ws_tickets` partial → done (97 LOC, 9 oracle tests,
14/14). Unit 3: `registry` partial → done (full scoped port, 8+14).
Unit 4: `prefix` partial → done (130 LOC, IPv6 fix, 12/12). Unit 5:
`base` partial → done (162 LOC, 9/9). Unit 6: `cookies` partial →
done (222 LOC, 3 fixes + codec, 17/17). Unit 7: `native_flow`
partial → done (165 LOC, 8/8). Unit 8: `request_utils` +
`refresh_singleflight` missing → done (89+107 LOC, 14 tests). Unit 9:
`middleware` + `route_logic` + `login_page` decision layers (20 gate
tests); all 13 siblings done. Full workspace: 218 suites, 2,036
passed, 0 failed.
**Ledger: 162/40/8693 (1.82% tracked, 4.66% prod).**
Files: `PLAN.md`, `HANDOFF.md` updated; ledger regenerated.
Next: next Wave B partial in smallest-LOC order.

## Jev god-tier pass landed (2026-09-20)

Five new `hermes-jev` modules covering every remaining live Jev use:
`compaction` (scored prune), `memory_nudge`, `review_gate`
(spawn+skill), `skill_route` (two-stage cookbook router), `triage`
(label+flags) — plus `hermes-agent::jev_review` seam. All default OFF,
keyless-safe, never throwing. Live-proven @ jev-1.13.0 (fixtures
locked). Validation: `cargo build --workspace` green;
`cargo test --workspace -- --test-threads=1` green — 215 suites,
1,973 passed, 0 failed; fmt + diff-check clean. Gates
`.unlazy/jev-godtier/GATES.md` 5/5 met.
**Ledger: no status change — 149/48/8698 (1.68% tracked, 4.28% prod).**
Files: `PLAN.md`, `HANDOFF.md` updated/verified; `tools/inventory.json`
+ `CONVERSION-LEDGER.md` verified unchanged (additive layer).

## Jev System-One layer landed (2026-09-20)

Additive `hermes-jev` crate (Choice/Noul/Score transport + router /
tool-select / stop-hook / guardrail + scrubber) with
`hermes-agent::jev_router` and `hermes-toolsets::jev_tool_route` seams —
all default OFF, keyless-safe, never throwing. Live-proven against
`jev-1.13.0` with the Hermes TYPESAFE_API_KEY (sourced at call time,
never printed): router→coding_agent @1.0, tool→read_file @0.9, stop
0.84, guardrail 0.55/0.17; payloads locked as test fixtures.
Validation: `cargo build --workspace` green; `cargo test --workspace
-- --test-threads=1` green — 214 suites, 1,935 passed, 0 failed;
`cargo fmt --all`, `git diff --check` clean. Gates
`.unlazy/jev-systemone/GATES.md` 5/5 met.
**Ledger: no status change — 149/48/8698 (1.68% tracked, 4.28% prod).**
Files: `PLAN.md`, `HANDOFF.md` updated/verified; `tools/inventory.json`
+ `CONVERSION-LEDGER.md` verified unchanged (additive layer, no
tracked-module change).

## agent.verify.recipes done (2026-09-22)

Partial → done @ 5d59366 (296 LOC; every oracle case mirrored,
2 real bugs fixed — nextjs default 3000, bool-port truthiness —
1 documented fail-open divergence, 26 oracle tests, 26/26).
Full workspace: 2,156 passed, 0 failed.
**Ledger: 183/19/8693 (2.06% tracked, 5.26% prod).**
Next: next Wave B partial in smallest-LOC order
(`tools.schema_sanitizer` 386, `toolsets` 485).

## gateway.platforms.qqbot.keyboards done (2026-09-22)

Partial → done @ 5d59366 (287 LOC; full surface incl ApprovalSender,
int()/str() coercions, 27 oracle tests, 27/27; 2 documented fail-open
divergences). Full workspace: 218 suites, 2,036→2,139 passed
(+103 across the session window incl. concurrent dashboard-auth
closure), 0 failed.
**Ledger: 182/20/8693 (2.05% tracked, 5.23% prod).**
Next: next Wave B partial in smallest-LOC order
(`agent.verify.recipes` 296, `tools.schema_sanitizer` 386).

## Next actions, in order

1. Wave B: next smallest-LOC partials (`agent.verify.recipes` 296,
   `tools.schema_sanitizer` 386, `toolsets` 485 — `agent.ssl_guard`
   and `hermes_cli.dashboard_auth.__init__` are already done).
2. Wave A: trivial sweep (`__init__` 0–3 LOC, eslint configs, barrels —
   acp/cron-scripts/tui roots need crate-open decision first).
3. Keep ownership disjoint; commit and publish each logical unit immediately.

## Archive pointer

History at pin `b9aa928` (sessions 4da/5a–5f, old review checkpoints):
`docs/archive/handoff-b9aa928.md` (stable path; board cites keep working).
Fidelity semantics mined to `docs/fidelity-appendix.md` (exempt from cap).
