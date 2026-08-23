# Hermes Agent Rust — Next-session handoff

Date: 2026-08-24 (Pacific/Auckland), session 4g.

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

Latest synchronized units: local source `f04daa74` → GitHub `cf104bd`
(`plugins.model-providers.stepfun.__init__`), after local `6d1c89dc` → GitHub
`e14acc6` (`plugins.model-providers.kilocode.__init__`), local `fcf144a1` →
GitHub `11245d8` (`plugins.model-providers.arcee.__init__`), local `ec9db5aa`
→ GitHub `9f8f7f6` (`plugins.model-providers.alibaba.__init__`), local
`c121ae3` → GitHub `3996dcb6` (`providers.base`), and local `0fdafeea` →
GitHub `b1cb43a7` (`providers.__init__`), all at upstream `b9aa928`. `main`
is aligned to the fetched remote mirror; the API-authored SHA differs only
because it cannot preserve the local author/committer timestamps.

## What landed this session

Module-sized commits are complete through
`plugins.model-providers.stepfun.__init__`: `db23d7c`
hermes-logging test isolation;
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
profile crate unit (`c121ae3` locally, mirrored as `3996dcb6` remotely);
`providers.__init__` registry/discovery (`0fdafeea` locally, mirrored as
`b1cb43a7` remotely); and the Alibaba bundled profile
(`ec9db5aa` locally, mirrored as `9f8f7f6` remotely); and the Arcee bundled
profile (`fcf144a1` locally, mirrored as `11245d8` remotely).
The Kilo Code bundled profile is `6d1c89dc` locally, mirrored as `e14acc6`
remotely. The StepFun bundled profile is `f04daa74` locally, mirrored as
`cf104bd` remotely.

The new `hermes-providers` crate ports `providers/base.py` and
`providers/__init__.py` @ `b9aa928`: declarative profile defaults and hooks,
model endpoint precedence, strict fail-open catalog parsing,
credential-safe redirects, canonical/alias registry behavior, copy-safe
caching, and sorted bundled/user/legacy discovery. The focused suites contain
9 base, 8 registry, 2 Alibaba profile, 2 Arcee profile, 2 Kilo Code profile,
and 2 StepFun profile tests are green. The provider surface remains partial
for the future CLI version/opener integration and remaining Rust plugin profile
loaders. The next unit is the smallest remaining bundled profile,
`plugins.model-providers.openai-codex.__init__` (15 LOC).

The required workspace run was green before the commit split:

```text
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo build --workspace
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo test --workspace
```

All tests passed; only three intentional delegation/schema doc tests were ignored. Two full-suite flakes were fixed and committed: hermes-logging queue registry state and hermes-constants environment/profile state now have shared process-global test mutexes.

## Exact working-tree state

After the current StepFun commit is mirrored and `main` is aligned to its
remote mirror, the working tree is clean. The committed metadata
includes `PLAN.md`, `tools/port_status.json`, generated `tools/inventory.json`,
`CONVERSION-LEDGER.md`, and this handoff. No code or parity test is pending
for the StepFun unit.

## Next actions, in order

1. Start `plugins.model-providers.openai-codex.__init__` by reading its pinned
   source/tests and writing profile-registration parity tests first.
2. Keep the static bundled-profile registration order and user-loader seam
   explicit while adding the next provider profile.
3. For every future module, commit and publish each logical unit immediately;
   use the connected GitHub API until local GitHub CLI authentication exists.

## Current conversion ledger

`CONVERSION-LEDGER.md` is generated and contains one row for every 3,882 upstream inventory modules: 1,103 production tasks plus 2,779 oracle/test tasks. Only `done` counts toward completion.

- All tracked modules: 45 done / 9 partial / 3,828 missing = **1.16%**.
- Production modules: 45 done / 9 partial / 1,049 missing = **4.08%**.

The nine partial production rows are `hermes_constants`, `providers.base`,
`providers.__init__`, `tools.credential_files`,
`tools.delegation_output_schema`, `tools.threat_patterns`, `tools.todo_tool`,
`tools.tool_backend_helpers`, and `tools.tool_output_limits`. Their closure
seams are listed in the ledger and PLAN.md.

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

The focused provider parity suites passed 9 base, 8 registry, 2 Alibaba, 2
Arcee, 2 Kilo Code, and 2 StepFun profile tests. The
required workspace build and test also passed with the explicit cargo
toolchain; three
delegation/schema doc tests are intentionally ignored. Inventory and
conversion ledger were regenerated and now record 45 done / 9 partial / 1,049
missing production modules.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```
