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

## dashboard_auth close in progress (2026-09-21)

Unit 1/6: `token_auth` partial → done @ 5d59366 (96 LOC,
red-green-proven `catch_unwind`, 14/14 tests). Unit 2/6: `ws_tickets`
partial → done @ 5d59366 (97 LOC, 9 oracle tests incl. concurrency,
14/14). Unit 3/6: `registry` partial → done @ 5d59366 (full scoped
port, 3 oracle tests, 8+14 green). Unit 4/6: `prefix` partial →
done @ 5d59366 (130 LOC, urlparse-IPv6 fix red-green, 12/12).
Unit 5/8: `base` partial → done @ 5d59366 (162 LOC, 1 oracle test,
9/9). Unit 6/8: `cookies` partial → done @ 5d59366 (222 LOC, 3
divergence fixes + PKCE codec, 8 oracle tests, 17/17). Unit 7/8:
`native_flow` partial → done @ 5d59366 (165 LOC, 2 consume tests,
8/8). Unit 8/10: `request_utils` + `refresh_singleflight` missing →
done @ 5d59366 (89+107 LOC, 14 oracle tests incl. burst). **Ledger:
158/41/8696 (1.78% tracked, 4.54% prod).** Next: middleware + routes
+ login_page (FastAPI surfaces, decision-tables portable) → __init__
close.

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

## Next actions, in order

1. Wave B unit 6: next smallest-DIFF partials (`agent.ssl_guard`,
   `plugins.model-providers.alibaba.__init__` regional set,
   `hermes_cli.dashboard_auth.__init__` remainder).
2. Wave A: trivial sweep (`__init__` 0–3 LOC, eslint configs, barrels —
   acp/cron-scripts/tui roots need crate-open decision first).
3. Keep ownership disjoint; commit and publish each logical unit immediately.

## Archive pointer

History at pin `b9aa928` (sessions 4da/5a–5f, old review checkpoints):
`docs/archive/handoff-b9aa928.md` (stable path; board cites keep working).
Fidelity semantics mined to `docs/fidelity-appendix.md` (exempt from cap).
