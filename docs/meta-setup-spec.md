# META setup spec (r1, 2026-09-17 — REJECTED, see r2 below): kill HANDOFF.md

> R1 verdict: S1 BUILD, S2 CONDITIONAL, S3 REJECT, S4 BUILD/S6 HOLD.
> Adjudication: S3's math is correct — real steady-state saving ≈1.1k
> words (HANDOFF 1,375 minus ~300 moved into the NOSKIP hot file), not
> ~25k; the spec compared against whole-reads nobody performs, taxed the
> one unskippable file +30%, and broke 26 cites. DELETION REJECTED.
> Counter-proposal in `docs/meta-setup-r2.md` (capped HANDOFF, kept).

## Target file set (steady state)

| File | Words now | Words after | Role |
|---|---|---|---|
| AGENTS.md | ~1,000 | ~1,300 | Protocol + resume point + hot rulings + query guardrails |
| PLAN.md | ~31,800 | ~30,000 | Parity matrix (§5) + single session log (§7) + next-actions head |
| GATES.md | ~500 | ~500 | Unchanged |
| docs/fidelity-appendix.md | ~2,350 | ~2,350 | Unchanged (cap-exempt) |
| docs/archive/* | — | +HANDOFF.md | Full HANDOFF history preserved, stable path |
| HANDOFF.md | ~1,375 | DELETED (pointer stub) | — |

Must-load per turn after: AGENTS (~1.3k) + PLAN §5 (~6k) + PLAN §7 tail
(~2k) + GATES (~0.5k) ≈ 10k words, down from ~35k.

## Function mapping (nothing lost)

1. Resume point (pin, recipe, scope) → AGENTS.md new §1 block (30 lines).
2. Binding rulings hot list → AGENTS.md (already adjacent to protocol).
3. Next-actions queue → PLAN §7 head (`## Next` block, updated per batch).
4. Batch history → PLAN §7 only (already there; HANDOFF copies deleted).
5. Verification evidence per batch → PLAN §7 entries (already there).

## Splice-detector succession (answers board R1/S3-B2)

The dual-prose cross-check is succeeded, not deleted: the pre-commit hook
already regenerates `tools/inventory.json` + ledger + README snapshot and
fails on unstaged generated docs; REQUIRED_SOURCE_DOCS becomes
`("PLAN.md",)` plus a new check — PLAN §7's latest entry ledger-counts
must equal `tools/inventory.json` summary (machine-verifiable, stronger
than eyeballing two prose files).

## Hook delta (`tools/pre_commit_docs.py`)

- `REQUIRED_SOURCE_DOCS = ("PLAN.md",)`; error text updated.
- New: parse latest PLAN §7 `**N done / M partial**` vs inventory
  summary; fail on mismatch.
- `docs/agents/issue-tracker.md` updated to match (single log).

## Rollback

`git log --oneline -- docs/archive/HANDOFF.md` + `git show` per archive
README drill; pointer stub at `HANDOFF.md` preserves the path.

## Gate rule

Unanimous BUILD, closures against live sources.
