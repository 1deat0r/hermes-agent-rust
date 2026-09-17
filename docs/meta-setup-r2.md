# META setup r2 (2026-09-17): capped HANDOFF, kept — adjudicated R1

R1: S1 BUILD, S2 CONDITIONAL (hook points cited), S3 REJECT (math),
S4 BUILD (duplication proven), S6 HOLD (3 orphans). Ruling: deletion
rejected; the file stays under a hard cap with the orphans homed.

## The cap (binding)

HANDOFF.md keeps, in order: Binding rulings (≤10 lines) → Next unit
(≤10) → Resume point (pin/recipe/scope) → last 3 batch sections →
Next actions → Archive pointer. Everything older moves to
`docs/archive/handoff-<pin>.md` at re-pin. Hard ceiling: 400 lines
(~3,500 words). Current: 267 lines — compliant, no cut needed today.

## Orphan homes (S6 — all inside HANDOFF, no new files)

(a) Branch/working-tree state → new `## Tree state` block after Resume
point (branch, HEAD, `git status` shape, updated per commit).
(b) Board-gate records → stay as batch-adjacent sections (already the
pattern: "Board gate R2" sits beside its batches).
(c) Skills-install note → batch entry like any other (already is).

## S2 hook points (no hook change — protocol unchanged)

`tools/pre_commit_docs.py` L13/L150-155 + `docs/agents/issue-tracker.md`
L16-19 stay as-is: dual-staging retained, no count-check added (S2's
semantic-gap warning accepted — machine check would bless lying prose).

## S3 conceded math (recorded honestly)

Steady-state must-load ≈ AGENTS (1k, whole) + PLAN §5 slice + §7 tail +
HANDOFF (1.4k, whole) ≈ 9k words. The r1 "35k→10k" compared against
whole-file reads the protocol already forbids; the deletion's true saving
was ≈1.1k words (HANDOFF 1,375 minus ~300 moved into the NOSKIP hot
file) against a +30% hot-file tax and 26 broken cites. Real saving from
this r2: ~0 words today (already capped); structural win is the ceiling
holding as batches accumulate.

## Gate rule

Unanimous BUILD, closures against live sources.

## Board record

| Round | Verdicts | Tally |
|---|---|---|
| R1 | S1 BUILD, S2 COND, S3 REJECT, S4 BUILD/HOLD | deletion rejected on S3 math |
| R2 | S1/S2/S4/S5/S6 BUILD, S3 COND→ | 1 figure missing |
| R3 | S3 BUILD | **UNANIMOUS BUILD — GATE PASSED** |
