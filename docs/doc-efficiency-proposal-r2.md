# Doc-efficiency proposal (spec r2, 2026-09-17 — adjudicated R1)

R1: 1 REJECT (S3) / 4 CONDITIONAL. Rulings: S1-number CONFIRMED (32→143,
trivial); S3-B1 (JSON bomb) CONFIRMED; S3-B2 (dual-log load-bearing)
CONFIRMED — fix 3 (single log) DROPPED; S3-B3/B4, S2, S4, S5 items
CONFIRMED with closures below. Honest steady-state math per S5: must-load
≈42.4k words; ledger is accident-risk, not recurring cost.

## Fix 1r — read guardrails with allowlisted queries (replaces fix 1)

- AGENTS.md turn protocol gains: "Query `tools/port_status.json` ONLY via
  these three shapes (r3-tightened, S3-B1): (a) `status_counts` /
  `prod_status_counts` dicts; (b) one module row by exact key (output ≤40
  lines); (c) top-N filtered lists, default `--limit 20`, hard max 50
  rows. Never `cat`/`read` `tools/inventory.json` (1.3MB/137k tokens) or
  CONVERSION-LEDGER.md (230k words) whole; never `print(open(...).read())`
  or `json.load` + full dump. Schema lives at the top of
  `tools/port_status.json` (keys: module → {status, retarget_note})."
- CONVERSION-LEDGER.md header gains "generated-only; agents query JSON
  summaries per AGENTS.md".
- Closure: S3-B1 (caps + schema + numeric limits, no unbounded reads),
  S5-B1/B3 (post-fix must-load list + wc before/after in the implementing
  commit).

## Fix 2r — HANDOFF cap with fidelity mining (replaces fix 2)

- Keep: resume point + Next actions + current ledger + working-tree
  pointers, explicitly (S4-B2).
- Before archiving: mine the `HANDOFF.md` "Fidelity notes" section
  bullets (r3: unpinned from line numbers — locate via `grep -n "^##\?- \?
  Session.*fidelity\|^## Fidelity"`, S3-N1) into code `PARITY:` comments
  or a kept `docs/fidelity-appendix.md` exempt from the cap (S4-B1).
- Archive: `docs/archive/handoff-<pin>.md` at re-pin only (forward-only,
  never retroactive — existing board cites keep working, S2-B3).
- Closure: S4-B1/B2, S2-B3 (stable paths + pointer lines).

## Fix 3r — hook + docs update shippED with the change (replaces fix 3's hole)

- DROPPED: single session log. Dual PLAN §7 + HANDOFF stays (S3-B2:
  redundancy is the splice-drift detector + §2B invariant).
- The implementing change ships the `tools/pre_commit_docs.py` delta (if
  any doc requirement text changes) + `docs/agents/issue-tracker.md`
  update in the same commit (S2-B1, S4-B3).
- Closure: dry-run hook exit 0 recorded (S2-B2).

## Fix 4r — archive with pointer integrity + rollback recipe

- Pointer lines preserve exact filename→archive mapping; post-move
  dangling-ref check enumerates EVERY archived basename (r3: build the
  alternation from `ls docs/archive/`, never a hardcoded pair — S3-N2,
  S4-B5): `grep -rn "$(ls docs/archive/ | sed 's/.md$//' | paste -sd'|')"
  --include="*.md" .` outside `docs/archive/` must show zero refs except
  the pointer lines themselves.
- Rollback drill (r3: S3-B4, S6-3): `docs/archive/README.md` records
  pin → path → commit range; drill = `git log --oneline -- docs/archive/`
  lists the move commit + `git show <move>:<archived-path>` renders the
  file; pass = both succeed. Dry-run receipt for the docs-only path
  (r3: S2-B2, S6-1): `git add docs/...` (docs-only) +
  `python3 tools/pre_commit_docs.py` → `documentation checkpoint passed`,
  exit 0 — recorded 2026-09-17 live.
- Rollback recipe recorded in the archive index (`docs/archive/README.md`:
  pin → path → restoring commit range) — tested once at creation (S3-B4).
- Binding rulings (`board rulings bind`, AGENTS.md:106) stay hot: the
  RULINGS SUMMARY (5 lines) remains in HANDOFF resume state, full records
  archive (S3-B3).

## Gate rule

Implement only on unanimous BUILD with all material blockers closed
against live sources.

## Board record

| Round | Verdicts | Tally |
|---|---|---|
| R1 | S1 COND, S2 COND, S3 REJECT, S4 COND, S5 COND | 1R/4C |
| R2 | S1 BUILD, S2 COND→, S3 REJECT→, S4 COND→, S5 BUILD, S6 COND | carries |
| R3 | S1 ✓, S2 BUILD, S3 BUILD, S4 BUILD, S5 ✓, S6 BUILD | **UNANIMOUS BUILD — GATE PASSED** |
