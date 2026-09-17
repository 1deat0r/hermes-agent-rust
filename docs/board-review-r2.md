# Board review R2 — skills install + first skilled P6 unit (spec r1)

Date: 2026-09-16. Under review: commit `9007212` (mattpocock/skills
@ `959a8e9` snapshot install, `docs/agents/` repo config, `tdd`+`implement`+
`code-review` proof unit `ts:apps.desktop.electron.active-runtime-state`,
long-horizon §3 automation upgrade).

## Gate rule

Proceed to the next long-horizon goal only when every seat returns BUILD
**and** every material blocker has a verified closure against live sources.
Unanimity necessary, not sufficient.

## Claims under review

1. Snapshot is genuine mattpocock/skills @ `959a8e9` (33 skills, upstream
   selection, copy method as approved).
2. Install is clean: no collisions, no hostile content, complete dirs.
3. Repo config follows `setup-matt-pocock-skills` (issue-tracker local,
   default triage labels, single-context domain, AGENTS.md block).
4. Proof unit is faithful: 7 oracle cases + 2 labeled hardening tests;
   UTF-16 length parity correct; seam contracts documented; review
   findings actually closed in code.
5. Ledger/docs agree (28 done; 1,798/0 serial; diff clean).
6. Skill registry staleness (harness doesn't list new skills) is disclosed,
   not hidden.

## Board record

| Round | Seats | Verdicts | Tally |
|---|---|---|---|
| R1 | S1 supply-chain, S2 skills-fidelity, S3 adversarial, S4 P6-unit parity, S5 ledger/docs | 4×BUILD, S2 CONDITIONAL (1 disclosure line) | revise required |
| R2 | S2 + S3 verify-by-quote, S6 cold-read | S2 BUILD; S3 BUILD; S6 CONDITIONAL (2 record items) | pointers + live run |
| R3 | S6 re-confirm | 1 PASS + 1 FAIL (line pointers) | relabel cites |
| R4 | S6 final confirm | BUILD | **UNANIMOUS BUILD — GATE PASSED** |

## Adjudication log

- S2-B1 (registry-staleness undisclosed in docs) — CONFIRMED. Fixed: caveat
  line in `docs/long-horizon-101.md` §2 (2026-09-16).
- S3 kill 1 (staleness) — self-closed live: staging HEAD == `959a8e9`
  (command evidence `git -C /tmp/skill-stage rev-parse HEAD` →
  `959a8e9f1edc…`, no file:line exists for VCS state; still origin/main
  HEAD on 2026-09-16). No action.
- S3 kill 2 (masquerade) — self-closed live: hardening labeled
  (`crates/hermes-desktop/tests/parity_active_runtime_state.rs:97-99`,
  "labeled as mine, not upstream's"). No action.
- S3 kill 3 (UTF-16) — self-closed live: fix in code
  (`crates/hermes-desktop/src/active_runtime_state.rs:60`,
  `commit.encode_utf16().count() >= 7`). No action.
- S3 kill 5 (ledger) — self-closed live: entry exists
  (`tools/port_status.json:673`,
  `ts:apps.desktop.electron.active-runtime-state` done; 28 done-count
  holds). No action.
- S3 nit (scaffold marker/test co-edit, stale fn name + status comment) —
  confirmed non-blocking; fixed anyway (renamed test, corrected status).
- S1/S4/S5: no blockers. Claim 5 verified live at R2 close:
  `cargo test --workspace -- --test-threads=1` EXIT=0, 195 suites ok,
  **1,798 passed / 0 failed**; `git diff --check` clean.
