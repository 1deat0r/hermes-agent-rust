# Long-horizon goal: 100% overall parity (set 2026-09-16, board-gated)

**Goal: every one of the 8,895 tracked modules `done` — 3,481 production
(1,147,748 LOC) + 5,414 oracles — with `cargo build/test --workspace`
green, GATES G1–G7 closed, at pin `5d59366` plus disciplined re-pins.**

Board gate for this plan: UNANIMOUS BUILD across R1→R3 (record:
`docs/board-review-r1.md`). Standing orders below are binding until the
user countermands them.

## 1. The arithmetic (why leverage, not heroics)

- Missing production: 3,312 modules / 1,100,953 LOC. Partials: 142 / 45,978.
- Oracles: 5,414 rows / 1,320,900 LOC of upstream test surface.
- Demonstrated velocity: ~22 rows/session. Naive projection: ~150 sessions.
- Conclusion: the plan must (a) shrink rows-per-unit cost via batching and
  codegen, (b) convert whole test-dirs into oracles mechanically, and
  (c) never re-pay drift — re-pin on a cadence, not continuously.

## 2. Phase order (dependency-safe, largest-contract-first)

1. **P1/P2 re-cert** (142 partials, smallest-DIFF first): the
   AST-classifier worklist from 2026-09-16 (`IDENT`→promote,
   `DIFF`→line-by-line). Closes the honest-demotion debt before new ports.
2. **P2 agent core**: `run_agent` + `AIAgent` loop on a model stub (P2 exit
   criterion — the single highest-leverage unit; unblocks toolsets,
   delegation,batch).
3. **P3 CLI**: `cli.py` + `hermes_cli/main` entry points (unblocks gateway
   `code_skew`, provider UA wiring, config crate seams).
4. **P4 gateway/plugins/cron** against ≥1 live platform.
5. **P5 TUI/ACP/scripts**.
6. **P6 desktop**: `hermes-desktop` Tauri backend bottom-up
   (backend-child/lifecycle → IPC → updater → windows), renderer per-module;
   Linux AppImage/deb/rpm last.
7. **Oracle sweep**: mechanical conversion of remaining `tests.*` rows into
   parity suites or explicit resolved gaps.

## 3. Mattpocock-skills doctrine (Total TypeScript standards, every turn)

No mattpocock skill is installed in this harness (verified 2026-09-16:
341 skills, none by that name). The standing instruction is therefore a
doctrine, applied to every TS-touching unit, at the highest quality bar:

1. **Strictest types, no `any`**: every ported TS surface (and every
   Rust↔TS bridge type in `hermes-desktop`) is fully typed; `any` requires
   a tracked ticket, never a pass. (`react-ts-patterns` skill §5 is the
   local enforcement arm until a mattpocock skill is ingested.)
2. **Typecheck-green is a gate**: `tsc --noEmit` (renderer) +
   `tsc -p tsconfig.electron.json --noEmit` (main) must pass for any
   P6 unit, mirroring upstream's `check:lint`. A P6 commit without the
   typecheck log is red by definition.
3. **Narrow the unknown at every seam**: IPC channels enumerated with
   payload schemas (cf. `electron-side-app` §2 — no untyped `invoke`);
   Zod-or-equivalent runtime validation where the renderer meets the
   backend; `unknown` + narrowing, never silent casts.
4. **Derive, don't duplicate**: shared contract types come from
   `@hermes/shared` (or its Rust mirror), never re-declared per consumer —
   one source of truth per shape, compiler-checked.
5. **Error handling explicit**: `Result`-shaped returns across the bridge;
   no `catch {}` swallowing; every failure mode in the type signature.
6. **When a genuine mattpocock skill becomes available** (user-supplied
   source), ingest it via the harness skill mechanism and amend this
   section the same turn — doctrine becomes automation.

## 4. Operating cadence (binding)

- One logical unit per commit; TDD red-first with RED observed in the log;
  `cargo build/test --workspace` + `git diff --check` per unit.
- Ledger honesty from the R1 lesson: AST re-proof must be comment-aware
  (raw-AST or Pass-normalized with the normalizer's blind spot stated);
  "identical" claims carry the classifier output, not prose.
- Re-pin cadence: stay on `5d59366` until P2 core closes; re-pin only at
  phase boundaries with a full demote/re-cert cycle, never mid-phase.
- Board re-gate at each phase exit (5 seats + cold read, unanimous BUILD).
- Session throughput target: ≥20 rows/session via small-module batching;
  codegen (`tools/gen_*.py`) for schema/table-shaped modules.

## 5. Definition of done (100%)

GATES G1–G7 all green: 8,895/8,895 done (3,481 prod), workspace green,
`cargo fmt --check` clean, ledger/README/PLAN/HANDOFF agree, every unit
committed + mirrored, live-platform contracts reviewed. No ignored test
hides a gap; no seam is an injectable placeholder.
