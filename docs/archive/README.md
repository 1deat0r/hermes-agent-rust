# Archive index (doc-efficiency r2, board-gated)

Forward-only: files move here at re-pin or supersession; live cites are
re-pointed to these stable paths, never left dangling.

| Pin / date | Archived file | Restoring commits |
|---|---|---|
| b9aa928 → 5d59366, 2026-09-16 | `handoff-b9aa928.md` (old-pin HANDOFF history) | `git log --oneline -- docs/archive/handoff-b9aa928.md` |
| 2026-09-17 | `board-review-r1.md` (gate record R1→R3) | `git log --oneline -- docs/archive/board-review-r1.md` |
| 2026-09-17 | `long-horizon-100.md` (superseded by 101) | `git log --oneline -- docs/archive/long-horizon-100.md` |
| 2026-09-17 | `doc-efficiency-proposal.md` (r1, superseded by r2) | `git log --oneline -- docs/archive/doc-efficiency-proposal.md` |

## Rollback drill (pass = both succeed)

```bash
git log --oneline -- docs/archive/  # lists the move commit
git show <move-commit>:docs/archive/<file>  # renders the archived file
```

Fidelity semantics mined before archiving live in
`docs/fidelity-appendix.md` (exempt from HANDOFF cap).
