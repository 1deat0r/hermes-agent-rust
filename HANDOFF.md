# Hermes Agent Rust — Next-session handoff

Date: 2026-08-24 (Pacific/Auckland), session 4c.

## Resume point

Repository: `/run/media/mustbearnold/Projects/AI Agents/Hermes-Agent-Rust`

Pinned upstream commit: `b9aa928`. The AGENTS file names `/home/mustbearn/Projects/Research/hermes-agent-repo`, but that path is absent on this machine. The checkout actually used and validated is `/run/media/mustbearnold/Projects/Research/hermes-agent-repo`. Set `HERMES_UPSTREAM` to that path when regenerating inventory data.

Current branch/HEAD: `main` at the committed implementation and metadata
sequence. The connected GitHub API publishes each logical commit immediately
as a sequential remote mirror. Its commit SHAs differ from the local sequence
because the API cannot preserve local author/committer timestamps, but every
tree snapshot and commit message matches and is verified before each ref
update. The local HTTPS Git client still has no credentials; use the
connected GitHub API for future pushes until `gh auth login` or SSH is
configured.

Latest synchronized unit: local source commit `c121ae3` and GitHub mirror
commit `3996dcb6` (`providers.base: port profile and model catalog @ b9aa928`).

## What landed this session

Module-sized commits are complete through `tools/working_diff` and the
current `providers.base` unit: `db23d7c` hermes-logging test isolation;
`a1a5b1b` hermes-constants test isolation; `cd4d356` audio_container;
`12d704e` computer_use/schema plus generator/golden; `9303b4b`
credential_files; `3d18cce` daemon_pool; `f55a634` debug_helpers; `e616a7c`
delegation_output_schema; `cbe33b1` desktop_ui; `b9cd597` env_probe;
`4b6826f` fal_common; `33d3112` interrupt; `b19de16` mcp_schema_cache plus
dependencies; `59b87f1` read_preview_tool; `35a35e6` read_terminal_tool;
`fb9f503` slash_confirm; `fe0e198` terminal_hints; `6bae97e`
thread_context; `e422040` threat_patterns plus `examples/prof_scan.rs`;
`f7ce193` todo_tool; `358f639` tool_backend_helpers; `e563376`
tool_output_limits; `74c5286` working_diff; and the new `providers.base`
profile crate unit (`c121ae3` locally, mirrored as `3996dcb6` remotely).

The new `hermes-providers` crate ports `providers/base.py` @ `b9aa928`:
declarative profile defaults and hooks, model endpoint precedence, strict
fail-open catalog parsing, and credential-safe redirect behavior. Its 9
focused loopback parity tests are green. `providers.base` remains partial for
the future CLI version injection and installed-opener integration; the next
unit is `providers.__init__` registry/discovery.

The required workspace run was green before the commit split:

```text
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo build --workspace
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo test --workspace
```

All tests passed; only three intentional delegation/schema doc tests were ignored. Two full-suite flakes were fixed and committed: hermes-logging queue registry state and hermes-constants environment/profile state now have shared process-global test mutexes.

## Exact working-tree state

After the current `providers.base` commit is mirrored and `main` is aligned
to its remote mirror, the working tree must be clean. The committed metadata
includes `PLAN.md`, `tools/port_status.json`, generated `tools/inventory.json`,
`CONVERSION-LEDGER.md`, and this handoff. No code or parity test is pending
for this unit.

## Next actions, in order

1. Verify the local/remote `providers.base` commit trees and leave `main`
   aligned with `origin/main`.
2. Start `providers.__init__` by reading its pinned source/tests and writing
   registry/discovery parity tests first.
3. For every future module, commit and publish each logical unit immediately;
   use the connected GitHub API until local GitHub CLI authentication exists.

## Current conversion ledger

`CONVERSION-LEDGER.md` is generated and contains one row for every 3,882 upstream inventory modules: 1,103 production tasks plus 2,779 oracle/test tasks. Only `done` counts toward completion.

- All tracked modules: 41 done / 8 partial / 3,833 missing = **1.06%**.
- Production modules: 41 done / 8 partial / 1,054 missing = **3.72%**.

The eight partial production rows are `hermes_constants`, `providers.base`,
`tools.credential_files`, `tools.delegation_output_schema`,
`tools.threat_patterns`, `tools.todo_tool`, `tools.tool_backend_helpers`, and
`tools.tool_output_limits`. Their closure seams are listed in the ledger and
PLAN.md.

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

The focused provider parity suite passed 9 tests. The required workspace build
and test also passed with the explicit cargo toolchain; three
delegation/schema doc tests are intentionally ignored. Inventory and
conversion ledger were regenerated and now record 41 done / 8 partial / 1,054
missing production modules.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```
