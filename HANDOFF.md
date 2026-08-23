# Hermes Agent Rust — Next-session handoff

Date: 2026-08-24 (Pacific/Auckland), paused overnight at the user's request.

## Resume point

Repository: `/run/media/mustbearnold/Projects/AI Agents/Hermes-Agent-Rust`

Pinned upstream commit: `b9aa928`. The AGENTS file names `/home/mustbearn/Projects/Research/hermes-agent-repo`, but that path is absent on this machine. The checkout actually used and validated is `/run/media/mustbearnold/Projects/Research/hermes-agent-repo`. Set `HERMES_UPSTREAM` to that path when regenerating inventory data.

Current branch/HEAD: `main` at the locally committed implementation and
metadata sequence. The connected GitHub API has published the complete code
and metadata sequence as a sequential remote mirror. Its commit SHAs differ
from the local sequence because the API cannot preserve local author/committer
timestamps, but every tree snapshot and commit message matches and was
verified before each ref update. The local HTTPS Git client still has no
credentials; use the connected GitHub API for future pushes until `gh auth
login` or SSH is configured.

## What landed this session

Module-sized commits are complete through `tools/todo_tool`: `db23d7c` hermes-logging test isolation; `a1a5b1b` hermes-constants test isolation; `cd4d356` audio_container; `12d704e` computer_use/schema plus generator/golden; `9303b4b` credential_files; `3d18cce` daemon_pool; `f55a634` debug_helpers; `e616a7c` delegation_output_schema; `cbe33b1` desktop_ui; `b9cd597` env_probe; `4b6826f` fal_common; `33d3112` interrupt; `b19de16` mcp_schema_cache plus dependencies; `59b87f1` read_preview_tool; `35a35e6` read_terminal_tool; `fb9f503` slash_confirm; `fe0e198` terminal_hints; `6bae97e` thread_context; `e422040` threat_patterns plus `examples/prof_scan.rs`; and `f7ce193` todo_tool.

The required workspace run was green before the commit split:

```text
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo build --workspace
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo test --workspace
```

All tests passed; only three intentional delegation/schema doc tests were ignored. Two full-suite flakes were fixed and committed: hermes-logging queue registry state and hermes-constants environment/profile state now have shared process-global test mutexes.

## Exact working-tree state

The code units are committed. The remaining working-tree changes are the
metadata/documentation unit: `PLAN.md`, `tools/port_status.json`, generated
`tools/inventory.json`, `CONVERSION-LEDGER.md`, `tools/conversion_ledger.py`,
  and this handoff. No code or parity test is pending; this state is clean
  after the metadata commit.

## Next actions, in order

1. Run `git diff --check`, confirm the workspace build/test evidence below,
   and leave `git status` clean.
2. For future module work, commit and publish each logical unit immediately;
   use the connected GitHub API until local GitHub CLI authentication exists.

## Current conversion ledger

`CONVERSION-LEDGER.md` is generated and contains one row for every 3,882 upstream inventory modules: 1,103 production tasks plus 2,779 oracle/test tasks. Only `done` counts toward completion.

- All tracked modules: 41 done / 7 partial / 3,834 missing = **1.06%**.
- Production modules: 41 done / 7 partial / 1,055 missing = **3.72%**.

The seven partial production rows are `hermes_constants`, `tools.credential_files`, `tools.delegation_output_schema`, `tools.threat_patterns`, `tools.todo_tool`, `tools.tool_backend_helpers`, and `tools.tool_output_limits`. Their closure seams are listed in the ledger and PLAN.md.

Regenerate with:

```bash
HERMES_UPSTREAM=/run/media/mustbearnold/Projects/Research/hermes-agent-repo tools/inventory.sh
python3 tools/conversion_ledger.py
```

The strict production formula is `done production modules / 1,103`; partial rows receive zero credit. The all-module percentage is also shown because every upstream test/oracle task remains explicit.

## Fidelity notes

- Desktop emitter is process-global like upstream; session-ID lookup remains a thread-local gateway seam.
- Credential registration uses an ordered vector for Python dict insertion order; it remains thread-local rather than async-task-local.
- Daemon pool rejects zero workers like Python; Rust Drop intentionally avoids joining wedged daemon workers.
- MCP cache loading reads/parses the file once and fails open on malformed content.
- `tools/gen_computer_use_schema.py` discovers the upstream root via `HERMES_UPSTREAM` and has path fallbacks for this machine.
- `cargo fmt --all -- --check` reports many pre-existing unformatted foundation files outside this wave. Do not mass-reformat unrelated crates; use targeted formatting only if needed.

## Verification evidence

The focused parity suites passed: backend helpers 40, output limits 9,
working diff 11, and file state 10. The required workspace build and test
also passed with the explicit stable toolchain; three delegation/schema doc
tests are intentionally ignored. Inventory and conversion ledger were
regenerated and remain at 41 done / 7 partial / 1,055 missing production
modules.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```
