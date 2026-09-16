# Long-horizon goal 101: from 28 to 100% — the skilled assault
(date: 2026-09-16, board-gated R2/R3/R4 unanimous BUILD, record:
`docs/board-review-r2.md`)

**Goal: 3,481/3,481 production rows `done` (1,147,748 LOC) + 5,414 oracles
resolved, GATES G1–G7 green, at pin `5d59366` with phase-boundary re-pins.
Start: 28 done / 142 partial / 3,311 missing prod. Mountain: 1,100,895
missing LOC + 45,978 partial LOC.**

## 1. Battle plan (4 waves, dependency order)

- **Wave A — empty/trivial sweep** (~60 rows: `__init__` 0–3 LOC, eslint
  configs, barrel `index` files): batch 10–15/commit with surface tests.
  Days, not weeks. Uses `implement` batch mode.
- **Wave B — partials closure** (142 rows: agent 38, tools 38, hermes_cli 22,
  plugins 21): smallest-DIFF first with the AST classifier; each closes its
  `retarget_note` seam, promotes, or (if genuinely blocked) records the
  owner crate. The 20–50 LOC band goes 5–8/session.
- **Wave C — agent core + CLI** (`run_agent` loop, `AIAgent`, `cli.py`,
  `hermes_cli/main`): the deep modules; `codebase-design` seams first,
  `to-spec`/`to-tickets` decomposition, tracer-bullet ports. Unblocks P3/P4.
- **Wave D — gateway/platforms/cron → TUI/ACP → P6 desktop → oracle sweep**:
  P6 Electron modules bottom-up (`api-transport` next); renderer per-module
  with `setup-ts-deep-modules` boundaries; root `tests-js` gates as oracles.

## 2. Mattpocock skill lanes (as many as quality demands — all 33 assigned)

| Lane | Skills | Applies to |
|---|---|---|
| Build loop (every unit) | `tdd`, `implement`, `code-review` | RED→GREEN→two-axis review→commit; proven 2026-09-16 |
| Deep-module design | `codebase-design`, `domain-modeling` | Wave C seams; P6 bridge types; glossary discipline |
| Decomposition | `to-spec`, `to-tickets`, `wayfinder` | Wave C/D breakdowns; map = ledger, frontier = HANDOFF next |
| Debugging | `diagnosing-bugs`, `systematic-debugging` (local) | Failing parity, flaky tests, drift mysteries |
| Research | `research`, `ask-matt` | Upstream rationale, PyJWT-style taxonomy mapping |
| Hardening | `implement-spec`, `improve-codebase-architecture` | Seams, adapters, depth audits per phase exit |
| Process | `triage`, `handoff`, `retro`, `teach` | Session checkpoints, retrospectives, handoffs |
| TS boundaries | `setup-ts-deep-modules` | P6 renderer: deep modules + entry-point discipline |
| Conflict safety | `resolving-merge-conflicts` | Re-pin merges, upstream restructuring |
| Elicitation | `grill-me`, `grilling`, `grill-with-docs`, `wizard`, `to-questionnaire`, `loop-me`, `prototype`, `wait-what`, `claude-handoff`, `writing-*` | Requirements, ADRs, specs on demand |

Quality bar (binding): no `any` on TS units; typecheck-green commit gate;
IPC schemas enumerated; contract types derived; two-axis review per unit;
RED observed in log; ≥20 rows/session; board re-gate per wave exit.

## 3. Milestones & exit

- M1: Wave A complete (trivial rows → ~90 done).
- M2: Wave B partials halved (<70 partial).
- M3: `run_agent` loop + `AIAgent` on model stub (P2 exit unblocked).
- M4: CLI surface + config crate (P3 exit).
- M5: Gateway live on ≥1 platform + cron (P4 exit).
- M6: TUI/ACP + desktop backend complete, Linux bundles built (P5/P6 exit).
- M7: Oracle sweep — zero unresolved rows; G1–G7 green; 100%.
