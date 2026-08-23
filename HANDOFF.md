# Hermes Agent Rust — Next-session handoff

Date: 2026-08-24 (Pacific/Auckland), session 4t.

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

Latest synchronized units: local source `69c5f5c` → GitHub `5a4f884`
(`plugins.model-providers.ai-gateway.__init__`), after local source `56a92d6` →
GitHub `93d9edc` (`plugins.model-providers.copilot-acp.__init__`), after local
source `d39ca6a` → GitHub `3f03b65` (`plugins.model-providers.gmi.__init__`),
after local source `1d974f1` →
GitHub `3984386`
(`plugins.model-providers.bedrock.__init__`), after local source `47d5865` →
GitHub `8db5d2e`
(`plugins.model-providers.novita.__init__`), after local source `eb61ae7` →
GitHub `ae9eb9e`
(`plugins.model-providers.nvidia.__init__`), after local source `5abcd78` →
GitHub `31daa217`
(`plugins.model-providers.azure-foundry.__init__`), after local `b62208f` →
GitHub `148b205c`
(`plugins.model-providers.alibaba-coding-plan.__init__`), after local `66c12f6` →
GitHub `38e28b0` (`plugins.model-providers.huggingface.__init__`), local `7afbf40` → GitHub
`c965919` (`plugins.model-providers.xai.__init__`), local `47c7ead` → GitHub
`aa77b2a` (`plugins.model-providers.xiaomi.__init__`), local `c5e98e03` → GitHub
`30d491d` (`plugins.model-providers.openai-codex.__init__`), local `f04daa74` →
GitHub `cf104bd` (`plugins.model-providers.stepfun.__init__`), local
`6d1c89dc` → GitHub
`e14acc6` (`plugins.model-providers.kilocode.__init__`), local `fcf144a1` →
GitHub `11245d8` (`plugins.model-providers.arcee.__init__`), local `ec9db5aa`
→ GitHub `9f8f7f6` (`plugins.model-providers.alibaba.__init__`), local
`c121ae3` → GitHub `3996dcb6` (`providers.base`), and local `0fdafeea` →
GitHub `b1cb43a7` (`providers.__init__`), all at upstream `b9aa928`. `main`
is aligned to the fetched remote mirror; the API-authored SHA differs only
because it cannot preserve the local author/committer timestamps.

## What landed this session

Module-sized commits are complete through
`plugins.model-providers.ai-gateway.__init__`: `69c5f5c` locally, mirrored as
`5a4f884` remotely; `db23d7c`
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
`cf104bd` remotely. The OpenAI Codex bundled profile is `c5e98e03` locally,
mirrored as `30d491d` remotely. The Xiaomi bundled profile is `47c7ead` locally,
mirrored as `aa77b2a` remotely. The XAI bundled profile is `7afbf40` locally,
mirrored as `c965919` remotely. The Hugging Face bundled profile is `66c12f6`
locally, mirrored as `38e28b0` remotely. The Alibaba Coding Plan bundled
profile is `b62208f` locally, mirrored as `148b205c` remotely.

The Azure Foundry bundled profile is included in the new `5abcd78` source
commit and its `31daa217` GitHub mirror.

The NVIDIA bundled profile is included in the new `eb61ae7` source commit and
its `ae9eb9e` GitHub mirror.

The NovitaAI bundled profile is included in the new `47d5865` source commit
and its `8db5d2e` GitHub mirror.

The AWS Bedrock bundled profile is included in the new `1d974f1` source
commit and its `3984386` GitHub mirror.

The GMI Cloud bundled profile is included in the new `d39ca6a` source commit
and its `3f03b65` GitHub mirror.

The GitHub Copilot ACP bundled profile is included in the new `56a92d6` source
commit and its `93d9edc` GitHub mirror.

The Vercel AI Gateway bundled profile is included in the new `69c5f5c` source
commit and its `5a4f884` GitHub mirror.

The new `hermes-providers` crate ports `providers/base.py` and
`providers/__init__.py` @ `b9aa928`: declarative profile defaults and hooks,
model endpoint precedence, strict fail-open catalog parsing,
credential-safe redirects, canonical/alias registry behavior, copy-safe
caching, and sorted bundled/user/legacy discovery. The focused suites contain
9 base, 8 registry, 2 AI Gateway profile, 2 Alibaba profile, 2 Alibaba Coding
Plan profile, 2 Arcee profile, 2 Azure Foundry profile, 2 Bedrock profile, 2
Copilot ACP profile, 2 GMI profile, 2 Kilo Code profile, 2 NovitaAI profile, 2
NVIDIA profile, 2 StepFun profile, 2 OpenAI Codex profile, 2 Xiaomi profile, 2
XAI profile, and 2 Hugging Face profile tests are green.
The provider
surface remains partial
for the future CLI version/opener integration and remaining Rust plugin profile
loaders. The next unit is the smallest remaining bundled profile,
`plugins.model-providers.fireworks.__init__` (46 LOC).

The required workspace run was green before the commit split:

```text
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo build --workspace
PATH=/home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH /home/mustbearnold/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/cargo test --workspace
```

The required sequential workspace test passed; only three intentional
delegation/schema doc tests were ignored. A concurrent invocation had one
unrelated hermes-toolsets environment-isolation flake; its isolated test and
the sequential required run passed. Two earlier full-suite flakes were fixed
and committed: hermes-logging queue registry state and hermes-constants
environment/profile state now have shared process-global test mutexes.

## Exact working-tree state

After the current AI Gateway commit is mirrored and this handoff commit is aligned to
its remote mirror, the working tree is clean. The committed metadata
includes `PLAN.md`, `tools/port_status.json`, generated `tools/inventory.json`,
`CONVERSION-LEDGER.md`, and this handoff. No code or parity test is pending
for the AI Gateway unit.

## Next actions, in order

1. Start `plugins.model-providers.fireworks.__init__` by reading its pinned
   source/tests and writing profile-registration parity tests first.
2. Keep the static bundled-profile registration order and user-loader seam
   explicit while adding the next provider profile.
3. For every future module, commit and publish each logical unit immediately;
   use the connected GitHub API until local GitHub CLI authentication exists.

## Current conversion ledger

`CONVERSION-LEDGER.md` is generated and contains one row for every 3,882 upstream inventory modules: 1,103 production tasks plus 2,779 oracle/test tasks. Only `done` counts toward completion.

- All tracked modules: 57 done / 9 partial / 3,816 missing = **1.47%**.
- Production modules: 57 done / 9 partial / 1,037 missing = **5.17%**.

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
- XAI pins the upstream `Hermes-Agent/0.20.0` default header; dynamic
  `hermes_cli.__version__` injection remains an explicit future CLI seam.
- Azure Foundry retains upstream's empty base URL because its endpoint is
  per-resource and user-supplied through `AZURE_FOUNDRY_BASE_URL`.
- NovitaAI's pricing-cache helper remains deferred with the future hermes-cli
  models seam; the provider profile and all declared fallback metadata are
  ported.
- Bedrock's upstream subclass `fetch_models()` override is represented by the
  `models_fetch_disabled` profile capability; it returns `None` without REST
  probing, while AWS SDK discovery remains a future provider/agent seam.
- GMI pins the upstream `HermesAgent/0.20.0` default header; dynamic
  `hermes_cli.__version__` injection remains an explicit future CLI seam.
- Copilot ACP's upstream `fetch_models()` override is represented by the
  `models_fetch_disabled` profile capability; it returns `None` without REST
  endpoint validation because model listing belongs to the external ACP
  subprocess.
- AI Gateway's upstream `build_api_kwargs_extras` override is represented by
  the `reasoning_passthrough` profile capability; it copies supplied reasoning
  config into `extra_body.reasoning`, defaults to enabled/medium, and honors
  `supports_reasoning=False` by omitting the body.
- `tools/gen_computer_use_schema.py` discovers the upstream root via `HERMES_UPSTREAM` and has path fallbacks for this machine.
- `cargo fmt --all -- --check` reports many pre-existing unformatted foundation files outside this wave. Do not mass-reformat unrelated crates; use targeted formatting only if needed.

## Verification evidence

The focused provider parity suites passed 9 base, 8 registry, 2 AI Gateway, 2
Alibaba, 2 Alibaba Coding Plan, 2 Arcee, 2 Azure Foundry, 2 Bedrock, 2 Copilot
ACP, 2 GMI, 2 Kilo Code, 2 NovitaAI, 2 NVIDIA, 2 StepFun, 2 OpenAI Codex, 2
Xiaomi, 2 XAI, and 2 Hugging Face profile tests. The
required workspace build and test also passed with the explicit cargo
toolchain; three
delegation/schema doc tests are intentionally ignored. Inventory and
conversion ledger were regenerated and now record 57 done / 9 partial / 1,037
missing production modules.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```
