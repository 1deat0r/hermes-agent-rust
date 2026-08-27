# Hermes Agent Rust — Next-session handoff

Date: 2026-08-28 (Pacific/Auckland), session 4d3.

## Resume point

Repository: `/run/media/mustbearnold/Projects/AI Agents/Hermes-Agent-Rust`

Pinned upstream commit: `b9aa928`. The AGENTS file names `/home/mustbearn/Projects/Research/hermes-agent-repo`, but that path is absent on this machine. The checkout actually used and validated is `/run/media/mustbearnold/Projects/Research/hermes-agent-repo`. Set `HERMES_UPSTREAM` to that path when regenerating inventory data.

Current branch/HEAD: `main` carries the agent-utility batch committed with
this documentation checkpoint; the previous units are published as `8a9e8f3`
(+ `0aea517` handoff), `448a4f6` (+ `a284322` handoff). Push and verify with
`git ls-remote origin refs/heads/main` after this checkpoint lands.

Latest synchronized source unit: six small `agent/` utility modules —
`errors`, `message_content`, `tool_result_classification`,
`lmstudio_reasoning`, `reasoning_summaries` (all `done`) and
`interrupt_compat` (`partial`) — ported TDD-first against their dedicated
upstream test modules into `hermes-agent`, moving production strict
completion from 6.71% to 7.16%. The previous unit
— Portal attribution and managed configuration — is published as `448a4f6`
plus the `a284322` handoff commit. Logical units continue to publish with
`git push origin main`.

Latest synchronized source unit: Portal ambient propagation —
`ConversationContext` capture/run plus
`propagate_conversation_context_to_thread` in `hermes-agent::portal_tags`,
closing the ContextVar-propagation seam recorded by session 4d1. No
module-status change; `agent.portal_tags` remains `partial` for the pinned
CLI version, the unported `AIAgent._compress_context` turn wrapper, and the
unported fan-out call sites that must adopt the capture.
Previous synchronized source unit: Portal attribution and managed
configuration — `agent.portal_tags`, `hermes_cli.managed_scope`, the merged
managed-directory fold, and the auxiliary-client header/`extra_body` chain at
upstream `b9aa928` (`448a4f6`, published to `origin/main`).
Previous working source unit: pure auxiliary model-policy predicates plus
OpenCode Zen/Go profiles, registry wiring, and reasoning/max-token seams
(`8a30854`).
The work this session inherited — the previous session's uncommitted and
partly red tree — is now landed and published: `agent.portal_tags` (partial),
the whole `hermes_cli/managed_scope.py` module (done), the merged-loader
managed-directory fold, and the auxiliary-client
client-header/`extra_body` chain. Three upstream-fidelity divergences found
in that uncommitted work were repaired before commit (the invented
`*.yaml` drop-in merge, the blank-key-trimming/`null`-rendering user-header
overlay, and the substring OpenRouter host gate).
Earlier config/Z.AI discovery work is recorded below; the current unit
preserves that source tree's dependency direction and extends it without a
CLI crate.
The next older source
`e568a282a692d65ee574ce2ca25db10741b95515` → GitHub
`e568a282a692d65ee574ce2ca25db10741b95515`
(`feat(agent): compose full credential pool loader @ b9aa928`), with the
previous local source `2c7798500593d6ac290c235181da28a7ef81e8d2` → GitHub
`dadb41d8b1e8cf4da277d6b133a81e3314122ad1`
(`feat(agent): seed custom pool config @ b9aa928`), with the previous local
source `19d0adb6efd8dbaf52d92c053d625a56993c2ea9` → GitHub
`33187eafa6a98fd0e7c22e582449e41bf102bb96`
(`feat(agent): seed Copilot credentials @ b9aa928`), with the previous local
source `4659f291b9bdc3637d3ebe5a2109f2cb8ee33f65` → GitHub
`502491d1a790c369613d717b64620c98c82b94fa`
(`feat(agent): seed Anthropic OAuth credentials @ b9aa928`), with the previous
local source `3d6ae211cdddfd859c5d86487b5811c8b8d9afd3` → GitHub
`f6bc402553af0c828fca84fc382e060dadef326a`
(`feat(agent): seed xAI OAuth credentials @ b9aa928`), with the previous
local source `aedda102d13da1515aa0ea7724702514a7d6a63d` → GitHub
`970720d0aba3a104423fb6b06b141e326d25854e`
(`feat(agent): seed OpenAI Codex credentials @ b9aa928`), with the previous
local source `5abcc34da3bca6ac682931743ce85023cef183eb` → GitHub
`f9f4c457df15052546da0cf498f847e4d7d53e26`
(`feat(agent): seed MiniMax OAuth credentials @ b9aa928`), with the previous
local source `c471092747601784ee50b4d9503b6877c379bb25` → GitHub
`e4bd1f2199e3290a5adf381888c06f1bd0f3337d`
(`feat(agent): seed Qwen OAuth singleton credentials @ b9aa928`), with the
previous local source
`e0d804b3b851b49ccc7688ee5e044ccdef5e7f26` → GitHub
`a8e152495fe343c6f793c7e1980add0ccda466ce`
(`feat(agent): seed Nous singleton credentials @ b9aa928`), with the previous
local source `d4322dded66b9ef9340212116514f6db63ee565a` → GitHub
`038a61c9f78b34426c07dcdf487df5fbd86ba808`
(`feat(agent): compose environment pool loader @ b9aa928`), with the previous
local source `608f5d409848a35b9e10c8971269cbea662d7a74` → GitHub
`628c0ee13a14ed5a77e59e88da6accfb083e4ff9`
`628c0ee13a14ed5a77e59e88da6accfb083e4ff9`
(`feat(agent): add environment credential seeding @ b9aa928`), with the
previous local source `197c14819ebc37739d7a501aa1d94a2133ec4d32` → GitHub
`b98265b02c4b65fdb7aae8ace265a5cd5d925efc`
(`feat(agent): add auth store locking @ b9aa928`), with the previous local
source `fd7e26d07e1efbab67885f56ca8d9eae2ce9b4a9` → GitHub
`e022d323070e2672017e26e71fb9c24678412b4d`
(`feat(agent): merge credential cooldown state @ b9aa928`), with the previous
local source `43b4baf` → GitHub `72976e0748ed6c1b708cc35465e463594806c6f1`
(`feat(agent): port credential store persistence @ b9aa928`), with the
previous local source `4ccaa7e` → GitHub `c2ecc258`
(`feat(agent): port credential pool orchestration helpers @ b9aa928`), with the
previous local source `1011551` → GitHub `6858357`
(`feat(agent): port credential pool row model @ b9aa928`), with the previous
local handoff `ffcdf32` → GitHub `2dec2f0`
(`handoff: record final auxiliary docs refs @ b9aa928`), with the previous
local handoff `743fbcf` → GitHub `539b7c0`
(`handoff: finalize auxiliary reqwest checkpoint @ b9aa928`), the earlier
local handoff `5762c3c` → GitHub `2cd0262`
(`handoff: record auxiliary reqwest mirror refs @ b9aa928`), source local
`56871db` → GitHub `7984aaa`
(`feat(agent): construct auxiliary reqwest clients @ b9aa928`) and the
earlier local handoff `7fa7cdd` → GitHub `bdfd901`
(`handoff: record credential pool mirror refs @ b9aa928`) and source local
`6fcc72f` → GitHub `1d46bab` (`feat(agent): port credential pool selection
core @ b9aa928`) immediately before it. The exact tree and all tracked blobs
were verified before aligning local `main` to the API-authored remote ref, with
the older synchronized history following. The preceding documentation/GitHub
metadata hook workflow and its handoff checkpoints were mirrored immediately
after each local commit. The last auxiliary-client source unit before that
workflow was local `9a0bc98` → GitHub `1550e03`; the older synchronized history
follows. The
`agent.auxiliary_client` Codex token-selection unit, including current
PLAN/inventory/ledger metadata), after local handoff source `116bb97` →
GitHub `9388f11` (`HANDOFF.md` for the Codex-header unit), after local source
`4d229c2` → GitHub `40fd571`
(`agent.auxiliary_client` Codex credential-header wire helper, including
current PLAN/inventory/ledger metadata), after local handoff source `8850d73`
→ GitHub `a8e9a5e` (`HANDOFF.md` for the proxy/TLS unit), after local source
`a4a29c3` → GitHub `cc22417`
(`agent.auxiliary_client` proxy/TLS policy boundary, including current
PLAN/inventory/ledger metadata), after local handoff source `6f2ef3b` →
GitHub `55bae88` (`HANDOFF.md` for the client-option unit), after local source
`557b301` → GitHub `590625a`
(`agent.auxiliary_client` OpenAI client-option retry boundary, including
current PLAN/inventory/ledger metadata), after local handoff source `fa943bd`
→ GitHub `4dc65b9` (`HANDOFF.md` for the endpoint-normalization unit), after
local source `1a375bb` → GitHub `51e0d2b`
(`agent.auxiliary_client` endpoint normalization and Anthropic host guard,
including current PLAN/inventory/ledger metadata), after local handoff source
`378fe35` → GitHub `63cb9f0` (`HANDOFF.md` for the pool-runtime unit), after
local source `37a31b4` → GitHub `241974f`
(`agent.auxiliary_client` pool-runtime credential/base-URL projection,
including current PLAN/inventory/ledger metadata), after local handoff source
`6173619` → GitHub `cf9a083` (`HANDOFF.md` for the task-provider routing
unit), after local source `ae1eb70` → GitHub `ac22bde`
(`agent.auxiliary_client` task-provider routing extension, including current
PLAN/inventory/ledger metadata), after local handoff source `008472a` →
GitHub `38d93dd` (`HANDOFF.md` for the auxiliary predicate/wire unit), after
local source `b119001` → GitHub `dfc21ed`
(`agent.auxiliary_client` predicate/wire partial, including current
plan/inventory/README metadata), after local handoff source `a224ff3` →
GitHub `4a7020b` (`HANDOFF.md` for the ZAI unit), after local source
`0dfa448` → GitHub `df894bb` (`plugins.model-providers.zai.__init__`,
including current plan/inventory metadata), after local handoff source
`c7f93e5` → GitHub `db3df00`
(`HANDOFF.md` for the Kimi Coding unit), after local source `7fe4f19` →
GitHub `a85e7a2` (`plugins.model-providers.kimi-coding.__init__`, including
current plan/inventory metadata), after local handoff source `edd9b4a` →
GitHub `b2e1286`
(`HANDOFF.md` for the Upstage unit), after local handoff source `137d7aa` →
GitHub `23d5ee9`
(`HANDOFF.md` for the Qwen OAuth unit), after local source `4e3894e` → GitHub
`41482fc` (`plugins.model-providers.qwen-oauth.__init__`, including current
plan/inventory metadata), after local handoff source `4a1881e` → GitHub
`c8ad87a` (`HANDOFF.md` and corrected PLAN metadata for the Custom unit),
after local source `adf7332` → GitHub `6e88a75`
(`plugins.model-providers.custom.__init__`, including current plan/inventory
metadata), after local handoff source `fa2a85a` → GitHub `b58e0aa`
(`HANDOFF.md` for the Minimax unit), after local source `54d7a3d` → GitHub
`9d44d76` (`plugins.model-providers.minimax.__init__`, including current
plan/inventory metadata), after local handoff source `90d52cd` → GitHub
`d880d1a` (`HANDOFF.md` for the Ollama Cloud unit), after local source
`63db0cf` → GitHub `1262f1f`
(`plugins.model-providers.ollama-cloud.__init__`, including current
plan/inventory metadata), after local handoff source `18c9e03` → GitHub
`420ec76` (`HANDOFF.md` for the Actual unit), after local source `163edce` →
GitHub `5b7efa2` (`plugins.model-providers.actual.__init__`, including current
plan/inventory metadata), after local handoff source `15db2cd` → GitHub
`2de863e1` (`HANDOFF.md` for the Nous unit), after local source `956fa19` →
GitHub `6a3259f`
(`plugins.model-providers.nous.__init__`, including current plan/inventory
metadata), after local docs source `2523860` → GitHub `a6c7179` (`AGENTS.md`
documentation checkpoint), after local handoff source `e69136b` → GitHub
`87a5dd8` (`HANDOFF.md` for the DeepSeek unit), after local source `fd80752` → GitHub `372dc02`
(`plugins.model-providers.deepseek.__init__`, including current plan/inventory
metadata), after local handoff source `f9b2994` → GitHub `9baf32d`
(`HANDOFF.md` for the DeepInfra unit), after local source `a7f45d2` → GitHub
`984bf1e` (`plugins.model-providers.deepinfra.__init__`, including current
plan/inventory metadata), after local handoff source `6310352` → GitHub `abf4720`
(`HANDOFF.md` for the Vertex unit), after local source `1dca197` → GitHub `bb20257`
(`plugins.model-providers.vertex.__init__`, including current plan/inventory
metadata), after local test-hardening source `b2c1f4f` → GitHub `1682e21`
(`hermes-constants` platform-cache test serialization), after local handoff
source `2a31667` → GitHub `cb3c1e8` (`HANDOFF.md` for the Copilot unit), after
local source `9740ea9` → GitHub `67a40e3`
(`plugins.model-providers.copilot.__init__`), after local source `49fc714` →
GitHub `30f58e1` (`plugins.model-providers.gemini.__init__`), after local source `9006786` →
GitHub `58f6c0a` (`plugins.model-providers.anthropic.__init__`), after local source `2d9f1fd` →
GitHub `a643a07` (`plugins.model-providers.fireworks.__init__`), after local source `69c5f5c` →
GitHub `5a4f884` (`plugins.model-providers.ai-gateway.__init__`), after local source `56a92d6` →
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
`providers.__init__`, all at upstream `b9aa928`; the current `main` alignment is
recorded above with the config/Z.AI source commit and its verified tree.

## What landed this session

Session 4d3 opened the small-module batch: six self-contained `agent/`
utilities with real upstream oracles, each written test-first (all six suites
were red before their implementations). Five reached `done`; `agent.
interrupt_compat` is recorded `partial` because two of its six upstream cases
need the unported `run_agent.AIAgent` and `tools.delegate_tool`.

Two review corrections worth carrying forward: the tool-result oracle contract
is a JSON **string** payload (`isinstance(result, str)` then `json.loads`), not
a parsed mapping — the first test draft asserted the wrong shape and was fixed
rather than the implementation; and LM Studio's non-string `effort` /
`allowed_options` shapes raise in Python but fail open here, which is now
written into the module and PLAN.


Session 4d2 took the one portal seam session 4d1 had recorded as pending: the
ambient conversation id now propagates to worker threads. Upstream gets this
for free because `tools.thread_context.propagate_context_to_thread` copies the
whole `contextvars.Context`; the Rust port reproduces that contract for this
module's own variable with `ConversationContext::capture`/`run` and
`propagate_conversation_context_to_thread`, keeping the capture on the parent
thread, the re-invocable wrapper, the reused-`Context` write-back semantics,
and an RAII restore so a panic inside the target cannot leak the id into a
recycled thread. CPython's recursive/cross-thread `RuntimeError` guards are
documented as deliberately serialized instead of reproduced, and
`hermes_tools::thread_context`'s single snapshot-factory slot stays reserved
for the approval/gateway contextvars layer that will register a composite
context. Tests were written first and were red before the implementation.


Session 4d1 finished the wave a previous turn left uncommitted and red, then
closed it with the review, tests, and ledger. Three upstream-fidelity defects
were found by reading the pinned sources line by line:

1. `hermes-agent::config::load_merged_config_snapshot_with_managed_scope_at`
   merged every sorted `*.yaml` inside the managed directory. Upstream
   `managed_scope.load_managed_config` reads exactly one file,
   `<managed_dir>/config.yaml`, and fails open when it is absent, malformed, or
   not a mapping. The whole `hermes_cli/managed_scope.py` module is now ported
   as `hermes-agent::managed_scope` (resolver tiers, `(mtime_ns, size)` cache
   and invalidation hook, config/`.env` loaders with explicit-directory forms,
   dotted leaf-key helpers, and `apply_managed_overlay`), and the merged
   snapshot records the real `managed_dir` instead of the config path's parent.
2. `_apply_user_default_headers` had invented blank-key trimming and rendered
   `null` as the string `"None"`. Upstream skips only explicit `None` values,
   keeps keys verbatim, and merges the non-empty `model.extra_headers` alias so
   it wins over `model.default_headers`.
3. The OpenRouter response-cache gate used `base_url.contains("openrouter.ai")`
   with a provider-only predicate. Upstream applies the headers at two real
   sites — `_try_openrouter` (route gate, host-independent) and
   `_to_async_client` (`base_url_host_matches(..., "openrouter.ai")`) — so the
   port now unions those two gates through the shared host matcher, and the
   `openrouter` section (not the whole config) is the explicit input.

The Portal surface itself (`agent.portal_tags`) is a new module: base
`product=`/`client=` tags, `conversation=` helper, the ambient
`ContextVar`-equivalent publish/reset/get trio, and the ambient-over-explicit
precedence rule, with the auxiliary-client `_nous_extra_body`,
`get_auxiliary_extra_body`, and `_create_transport_client` tag/sticky-key
fallback living in `hermes-agent::auxiliary_client` as upstream does.


This source unit was developed through two parallel dependency-safe leaves.
The auxiliary leaf adds pure model-policy predicates for Kimi, Arcee Trinity,
OpenAI Codex gpt-5.4/5.5/5.6, and Codex Spark, with explicit temperature
directives and compression-threshold precedence. The provider leaf adds
OpenCode Zen and Go profiles, aliases, registry/loader wiring, the Go
Kimi/DeepSeek/GLM reasoning mappings, mutually exclusive wire shapes, and the
`mimo-v2.5-pro` 131,072 max-token cap. Neither leaf adds network, auth,
configuration, CLI, or shared mutable state.

Focused parity suites pass 46 auxiliary, 5 OpenCode, and 8 registry tests
(`unit`). Exact focused commands:
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-agent --test
parity_auxiliary_client`,
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-providers --test
parity_opencode_zen`,
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-providers --test
parity_registry`,
`/home/mustbearnold/.cargo/bin/cargo check -p hermes-agent`, and
`/home/mustbearnold/.cargo/bin/cargo check -p hermes-providers`.
Targeted `/home/mustbearnold/.cargo/bin/rustfmt --edition 2021 --check`
passes on all changed Rust files.

Workspace validation passes:
`/home/mustbearnold/.cargo/bin/cargo build --workspace` and
`/home/mustbearnold/.cargo/bin/cargo test --workspace --
--test-threads=1` (1,155 passed, 5 ignored, 20 warnings). `git diff --check`
passes. The currently scoped hermes-conversion gates are all met (36/36).
There is no module-status or ledger change: `tools/port_status.json` remains
unchanged, and `tools/inventory.sh` plus
`python3 tools/conversion_ledger.py` report 73 done / 11 partial / 3,798
missing tracked modules and 73 done / 11 partial / 1,019 missing production
modules, or 1.88% tracked strict completion and 6.62% production.
`PLAN.md`, `.unlazy/hermes-conversion/PLAN.md`,
`.unlazy/hermes-conversion/status.log`, `tools/inventory.json`, and
`CONVERSION-LEDGER.md` are updated or verified in this checkpoint.

The next dependency-safe unit is higher-layer provider/auth transport or
auxiliary-client integration. Full hermes-cli managed overlay/default-catalog
wiring remains blocked because the hermes-cli crate is absent.
The preceding source unit extends `hermes-agent::credential_pool` through the
provider-singleton seeding boundary. It adds the explicit `seed_from_singletons`
seam for the upstream Anthropic, Nous, Copilot, Qwen, MiniMax, OpenAI Codex, and
xAI branches: Anthropic provider opt-in/API-key-path gates, resolved
`hermes_pkce`/`claude_code` credential-file access/refresh/expiry/label mapping,
source suppression, and stale autodiscovered-row pruning; Nous
device-code source
suppression, stale device-code removal when singleton state has no runtime
material, invoke-JWT agent-key/runtime selection, custom labels, and
direct/extra metadata preservation for access/refresh expiry, obtained-at,
agent-key, endpoint, scope, and TLS fields; Copilot resolved exchanged-token
source classification (`gh_cli` versus `env:<VAR>`), all-source/per-source
suppression, enterprise/default endpoint mapping, API-key persistence, and
source labels; Qwen CLI source/auth type, access
token, expiry milliseconds, base URL, auth-file label, suppression, and
absent-token fail-open behavior; MiniMax OAuth source/auth type, access and
refresh tokens, ISO expiry milliseconds, trailing-slash-stripped base URL,
suppression, and custom-or-token-derived labels. Three source-derived `mock`
tests were added first; the focused credential-pool wave now has 38 pool plus
15 persistence tests. The upstream MiniMax branch has no dedicated
singleton-seeding test, so its implementation is the code oracle and that
gap is recorded in PLAN.md. OpenAI Codex adds nested tokens, the fixed Codex
backend URL, `last_refresh`, suppression, and custom-or-token-derived labels;
three additional source-derived `mock` tests bring the focused wave to 41
pool plus 15 persistence tests. Existing upstream Codex auth-provider tests
cover the nested auth-store shape, but no direct singleton-seeding test exists.
The xAI OAuth branch adds nested tokens, the fixed xAI endpoint,
`last_refresh`, device-code suppression, and token-derived labels; three
additional source-derived `mock` tests bring the focused wave to 44 pool plus
15 persistence tests. Dedicated upstream xAI pool-seeding tests cover
materialization and suppression. Four additional Anthropic source-derived
`mock` tests bring the focused wave to 48 pool plus 15 persistence tests; five
additional Copilot source-derived `mock` tests bring the focused wave to 53
pool plus 15 persistence tests (68 total). Copilot's higher-layer `gh auth
token` resolver and network exchange are represented by explicit resolved
token/source/endpoint inputs in this crate. Two additional custom-pool source-
derived `mock` tests bring the focused wave to 55 pool plus 15 persistence
tests (70 total). Custom-pool seeding accepts explicit provider/model config
and mirrors config/model API-key rows with normalized endpoint identity and
suppression. The current source checkpoint adds `PoolLoadInputs` and
`load_pool_with_inputs_at`, preserving the upstream `custom:*` versus
non-custom branch, singleton-then-environment ordering, non-destructive
environment pruning, custom stale-row pruning, Anthropic priority
normalization, strategy selection, and borrowed-safe persistence. Two more
source-derived `mock` tests bring the focused wave to 57 pool plus 15
persistence tests (72 total). One persistence assertion now allows the
sub-microsecond precision loss introduced by JSON floating-point round trips;
source behavior is unchanged. Outer configuration discovery/loading, Z.AI
endpoint probing, OAuth refresh, leases, and logging throttles remain pending.
Local source commit
`f6a266e6e5ef9417c9b49cf9f2d53563e0ac9ae4` was mirrored as GitHub
`940ead9f102e6aba3ea0d2c308d7cb9c732cbd5e`; both refs resolve to tree
`450e2757900fb773f3295e593782ec1a153b077f` with 273 matching tracked blobs.

The preceding synchronized source unit extended `hermes-agent::credential_pool`
through the lower environment-aware `load_pool` transaction. It added the
explicit profile/global pool paths and pool configuration inputs, mirrored
source profile/global fallback reads, borrowed-secret and auth-type healing
ownership, environment seeding, non-destructive env-row pruning, priority
normalization, sorted borrowed-safe persistence, and configured strategy
selection. Local source commit
`d4322dded66b9ef9340212116514f6db63ee565a` was mirrored as GitHub
`038a61c9f78b34426c07dcdf487df5fbd86ba808`; both refs resolve to tree
`d3765b69460cc5c11e066f1e2268cb9b2354ec46` with 273 matching tracked blobs.

The latest synchronized unit adds `hermes-agent::credential_store` around the
upstream auth-store boundary. It mirrors versioned empty-store defaults,
legacy `systems` migration, stale Nous Portal URL migration, corruption
quarantine versus active read-error propagation, atomic `0600` auth writes
with `0700` parents, per-provider profile/global fallback reads, and the
final borrowed-secret sanitizer at the pool write boundary. The follow-on
cooldown-recency merge now adopts newer live `EXHAUSTED` cooldowns and `DEAD`
quarantines for the same token while rejecting re-auth token changes and
expired cooldowns. The auth-store extension now adds platform-native
exclusive `.lock` files with 15-second minimum timeouts, same-thread
reentrancy, independent per-path holders, and locking across the full pool
write transaction. Fifteen source-derived `unit`/`mock` tests pass in the
persistence suite, bringing the credential-pool wave to 36 focused tests. The
approved credential-lifecycle leaf gates, workspace build, and serialized
workspace test pass. Full-workspace formatting and targeted Clippy remain
affected only by pre-existing issues outside this unit; environment/config
discovery, provider seeding, OAuth refresh, leases, logging throttles, and
cross-process orchestration beyond the auth-store lock remain pending.

The current unit adds `hermes-agent::credential_pool`'s source-upsert and
provider-boundary orchestration helpers. It mirrors source-scoped upserts,
duplicate-source collapse, changed-key failure-state clearing, Anthropic
manual/seeded priority normalization, configured fill-first/round-robin/
least-used/random strategy parsing and random selection, custom provider
name/endpoint lookup, non-empty custom-pool listing, and provider-boundary
matching. Seven source-derived `unit` tests were added; the focused suite now
has 21 passing pool tests. Auth-store persistence and env/config seeding,
provider OAuth refresh, leases, logging throttles, and cross-process pool
locking remain pending. The required workspace build, default test, and
serialized workspace test passed. Workspace Clippy was killed by the
environment with exit 137; targeted `hermes-agent` Clippy reports only
pre-existing auxiliary-client lint failures.
Local source commit `4ccaa7e` was mirrored as GitHub `c2ecc258`; both refs
resolve to tree `3877949d1ac4d13aa75246459134f86bb8775724` with 270 matching
tracked blobs.

The preceding unit added `hermes-agent::credential_pool`'s deterministic
in-memory core. `CredentialPool` mirrors source priority fill-first,
least-used, and round-robin selection; explicit reset timestamps and
status-based cooldowns; terminal OAuth `DEAD` transitions; failed-key identity
matching; duplicate-key quarantine; and unmatched-identity fail-open rotation.
Eight source-derived `unit` tests were added. Persistence/auth-store and env
seeding, serialization, OAuth refresh, lease locking, random selection,
logging throttles, and cross-process locking remain pending. The required
`/home/mustbearnold/.cargo/bin/cargo build --workspace` and
`/home/mustbearnold/.cargo/bin/cargo test --workspace` both passed; local
`6fcc72f` was mirrored as GitHub `1d46bab`, and the remote/local tree is
`b69eb818bc34145186f7432c8ebe8910e3f461da` with 270 matching tracked blobs.

The preceding credential-pool model unit extends `PooledCredential` with the
source's OAuth/provider metadata, JSON-only `extra` fields, defaulting and ISO
timestamp rehydration, persisted `last_status`, Anthropic `sk-ant-oat` auth
normalization, token labels, runtime base URLs, and Nous invoke-JWT scope/
expiry filtering. `to_dict`/`to_json` applies the upstream borrowed-secret
sanitizer and short SHA-256 fingerprint while preserving manual and
provider-owned device-code state. Six source-derived `unit` tests were added
first; the focused suite now has 14 passing pool tests. The workspace build
passed. The default parallel workspace test reproduced the existing
process-global `hermes-tools::parity_credential_files` race twice; the exact
isolated test and serialized workspace test passed, with only the three
intentional delegation/schema doc tests ignored. Auth-store orchestration,
environment/config seeding, OAuth refresh, leases, random selection, logging
throttles, and cross-process pool locking remain pending. Local source commit
`1011551` was mirrored as GitHub `6858357`; both refs resolve to the verified
tree `39b2fe2271e125baa2d27dc9be769ccb00d7085e` with 270 matching tracked
blobs.

The current auxiliary-client transport/pool unit adds
`AuxiliaryHttpClientConfig` and `openai_client_config_with_transport` in
`hermes-agent::auxiliary_client`. They mirror the pinned upstream keepalive
client's 20/100 limits, 20-second expiry, `(15, None, 15, 10)` timeout shape,
env-only proxy/no-proxy mount policy, sync/async mode, TLS choice, and
explicit-client-over-default precedence. The same unit adds fail-open pool
selection-state projection and pool-first runtime credential fallback for the
Nous/xAI-style auxiliary paths. Five source-derived `unit` tests were added;
the focused suite now has 37 passing tests. These remain transport-neutral and
injected-input adapters: concrete SDK/network client construction, pool
persistence/rotation/refresh, cancellation, and provider fallback chains are
still pending. The required workspace build and test commands passed. Local
`895fbcf` was mirrored as GitHub `ead6b5f`; the remote and local trees both
resolve to `d9720eddbe0198216912d7c3de6c8fb3693a45b1`, with 268 tracked blobs
matching by path, mode, and SHA.

The auxiliary-client construction unit adds
`build_auxiliary_http_client`, which selects concrete blocking or async reqwest
clients, disables ambient proxy lookup, forwards explicit proxy settings,
applies connect/idle-pool settings, supports insecure TLS, and loads explicit
PEM bundles as the certificate roots. Four source-derived `unit` tests were
added first; the focused auxiliary suite now has 41 passing tests. The
transport-neutral config retains the source's total-connection, write-timeout,
and pool-acquisition values because reqwest's public builder does not expose
those exact controls. Full SDK request/response integration, credential-pool
persistence/refresh, cancellation, and provider fallback remain pending.
The required workspace build and test commands passed. Local source commit
`56871db` was mirrored as GitHub `7984aaa`, followed by local handoff
`5762c3c` mirrored as GitHub `2cd0262`; the remote/local tree is
`cabc30ddb12626e265c4a2ec186c97e06b686815` with 270 matching tracked blobs.
The final handoff tree is
`50d8e1644e1a59c8c241cce083e583f4f51427f4` with the same 270 matching blobs.

The tracked documentation/GitHub metadata hook workflow is included in local
commit `0d2bcd4` and its `1af2de6` GitHub mirror. It is the current non-ledger
unit. Its `pre-commit` hook refreshes the generated inventory and
conversion ledger, updates the README status snapshot, stages those generated
files, and requires `PLAN.md` plus `HANDOFF.md` for source/parity/tooling
changes. Its `post-commit` hook synchronizes the repository description through
the GitHub API and verifies README parity without creating a second README
commit. No conversion-ledger status changed in this unit: 73 done / 10 partial
/ 3,799 missing tracked modules and 73 done / 10 partial / 1,020 missing
production modules.

The synchronized auxiliary-client predicate/wire slice is `b119001` locally,
mirrored as `dfc21ed` remotely; its task-provider routing extension is
`ae1eb70` locally, mirrored as `ac22bde` remotely; the ZAI profile is
`0dfa448` locally, mirrored as
`df894bb` remotely; the Kimi Coding profile is `7fe4f19` locally, mirrored as
`a85e7a2` remotely; the Upstage profile is `034b2ea` locally, mirrored as
`d1bff84` remotely; the Qwen OAuth profile is `4e3894e` locally, mirrored as
`41482fc` remotely; the Custom profile is `adf7332` locally, mirrored as
`6e88a75` remotely; the Minimax profiles are `54d7a3d` locally, mirrored as
`9d44d76` remotely; the Ollama Cloud profile is `63db0cf` locally, mirrored as
`1262f1f` remotely; the Actual profile is `163edce` locally, mirrored as
`5b7efa2` remotely; the Nous profile is `956fa19` locally, mirrored as
`6a3259f` remotely; the DeepSeek profile is `fd80752` locally, mirrored as
`372dc02` remotely; the DeepInfra profile is `a7f45d2` locally, mirrored as
`984bf1e` remotely; the Vertex profile is `1dca197` locally, mirrored as
`bb20257` remotely; the required platform-cache test hardening is
`b2c1f4f` locally, mirrored as `1682e21` remotely; `db23d7c`
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

The Fireworks AI bundled profile is included in the new `2d9f1fd` source
commit and its `a643a07` GitHub mirror.

The native Anthropic bundled profile is included in the new `9006786` source
commit and its `58f6c0a` GitHub mirror.

The Google Gemini bundled profile is included in the new `49fc714` source
commit and its `30f58e1` GitHub mirror.

The GitHub Copilot bundled profile is included in the new `9740ea9` source
commit and its `67a40e3` GitHub mirror.

The Google Vertex AI bundled profile is included in the new `1dca197` source
commit and its `bb20257` GitHub mirror. It mirrors Vertex's OAuth metadata,
aliases, base URL, auxiliary model, nested snake_case Gemini thinking body,
and unconditional no-REST model discovery behavior. The runtime OAuth token
adapter remains a future seam; `vertex_thinking` and `models_fetch_disabled`
make the current profile contract explicit.

The DeepInfra bundled profile is included in the new `a7f45d2` source commit
and its `984bf1e` GitHub mirror. It mirrors DeepInfra's metadata, aliases,
credentials, auxiliary model, empty fallback list, and key-gated live vision
default. The `deepinfra_vision` capability implements the tagged chat+vision
selection, base-URL catalog cache, negative-cache fail-open path, and Bearer
request; profile-scoped secret resolution and the CLI opener remain future
higher-layer seams.

The DeepSeek bundled profile is included in the new `fd80752` source commit
and its `372dc02` GitHub mirror. It mirrors DeepSeek's metadata, alias,
fallbacks, auxiliary model, and V4+ reasoning wire hook. The
`deepseek_reasoning` capability emits the explicit thinking body, preserves
disabled/no-op behavior, and maps effort levels to the top-level
`reasoning_effort`; broader reasoning-content history repair remains a future
agent/transport seam.

The Nous bundled profile is included in the new `956fa19` source commit and
its `6a3259f` GitHub mirror. It mirrors Nous's metadata, aliases, fallback
models, OAuth device-code auth, Portal product/client/conversation tags,
cron-stable sticky routing, provider preferences, and reasoning omission when
disabled. The `nous_portal` capability uses the pinned b9aa928 client version
and a `conversation_context` adapter; runtime CLI-version injection and
ContextVar propagation remain future higher-layer seams.

The Actual Computer bundled profile is included in the new `163edce` source
commit and its `5b7efa2` GitHub mirror. It mirrors Actual's metadata, aliases,
`ACTUAL_API_KEY`/`ACTUAL_BASE_URL` environment contract, hosted/local root URL
normalization, optional Bearer catalog auth, JSON/Accept/User-Agent headers,
list and `{data: [...]}` model payloads, ID filtering, and fail-open behavior.
The `actual_catalog` capability isolates the custom `fetch_models` hook;
runtime credential resolution, model-picker integration, and application
transport/opener wiring remain future hermes-cli seams.

The MiniMax bundled profiles are included in the new `54d7a3d` source commit
and its `9d44d76` GitHub mirror. The three registrations mirror the direct,
China, and OAuth profile metadata, aliases, API modes, credentials, base
URLs, OAuth description/signup fields, and auxiliary defaults. The
`minimax_reasoning` capability preserves the shared M3 hook's exact
`api.minimax.io/v1` and model/slug gating, mandatory `reasoning_split`,
adaptive thinking for any supplied config, explicit disabled thinking, and
no thinking body when config is absent. Auxiliary-client, OAuth runtime,
and broader agent/transport integration remain future higher-layer seams.

The Upstage Solar bundled profile is included in the new `034b2ea` source
commit and its `d1bff84` GitHub mirror. The `upstage_reasoning` capability
mirrors the deny-listed `solar-mini`/`syn-pro` substring families, default
medium reasoning for unset/empty effort, low/medium/high passthrough, minimal
omission, high clamping for xhigh/max/ultra and unknown efforts, and explicit
disabled omission. The profile mirrors Upstage's metadata, `solar` alias,
ordered API-key/base-URL environment variables, Solar endpoint, and
`solar-pro3` fallback; CLI provider overlay and full transport integration
remain future higher-layer seams.

The Kimi Coding bundled profiles are included in the new `7fe4f19` source
commit and its `a85e7a2` GitHub mirror. The `kimi_coding` capability mirrors
the exact HTTPS api.kimi.com /coding and /coding/v1 confirmation predicate,
normalizes confirmed /coding to /coding/v1, filters whitespace/case-insensitive
k3 IDs from unconfirmed catalogs, preserves fail-open probe errors, and emits
either thinking enabled/disabled or top-level low/medium/high
reasoning_effort. The global and China profiles mirror their aliases,
environment-variable order, Moonshot endpoints, omitted temperature, 32,000
max-token cap, hermes-agent/1.0 header, and auxiliary model. CLI credential
auto-detection and full transport integration remain future higher-layer
seams.

The Z.AI bundled profile is included in the new `0dfa448` source commit and
its `df894bb` GitHub mirror. The `zai_reasoning` capability mirrors the
source's GLM version predicate for GLM 4.5+ thinking, explicit enabled/
disabled `thinking.type`, GLM-5.2 aliases (`glm-5.2`, `glm-5-2`, and
`glm-5p2`), and the native top-level `reasoning_effort` mapping: standard
efforts normalize to `high`, xhigh/max/ultra normalize to `max`, and
empty/none/disabled configurations omit the effort field. The profile
mirrors Z.AI's aliases, ordered API-key variables, endpoint, fallback models,
and GLM-4.5 Flash auxiliary model; CLI credential/model-picker and full
transport integration remain future higher-layer seams.

The first `hermes-agent` checkpoint is included in the new `b119001` source
commit and its `dfc21ed` GitHub mirror. The partial
`hermes_agent::auxiliary_client` slice mirrors the source's provider alias
normalization (`custom:`, `codex`, `main`, and the bundled alias table),
OpenAI-compatible output-token keyword selection, and payment/quota,
rate-limit, stale-model, and model-capability error predicates. Hidden
Python config and exception introspection are explicit Rust adapter inputs;
client construction, credential pools, async transport, cancellation and
progress handling, provider fallback, and the remaining call path are still
pending.

The task-provider routing extension is included in the new `ae1eb70` source
commit and its `ac22bde` GitHub mirror. It mirrors explicit-over-config
precedence, matching task endpoint/key adoption, first-class provider identity
with an explicit base URL, direct `openai` → `custom` expansion, MoA
aggregator unwrapping with virtual credential removal, unresolved-MoA
fail-through, and explicit/configured `model: auto` normalization. Config
maps, MoA preset results, and provider-registry membership are explicit Rust
adapter inputs; secret-scope/key-env lookup and client construction remain
pending higher-layer seams.

The pool-runtime credential/base-URL projection is included in the new
`37a31b4` source commit and its `241974f` GitHub mirror. It mirrors the
projected runtime-key/access-token fallback, runtime/inference/base/fallback
URL precedence, whitespace/trailing-slash normalization, and the Nous-only
inference base-URL override. Pool JWT validation, secret lookup, and actual
SDK/client construction remain explicit auth/transport seams.

The OpenAI-compatible endpoint normalization and Anthropic host guard are
included in the new `1a375bb` source commit and its `51e0d2b` GitHub mirror.
They mirror MiniMax `/anthropic` → `/v1`, Z.AI `bigmodel` → `/paas/v4`, Kimi
Coding `/coding` → `/coding/v1`, unchanged endpoint normalization, and exact
`api.anthropic.com` acceptance including case, trailing-dot, and
protocol-relative forms while failing closed for foreign, malformed, and bare
host values. SDK construction, proxy/TLS bootstrap, and request transport
remain pending.

The transport-independent OpenAI client-option retry boundary is included in
the new `557b301` source commit and its `590625a` GitHub mirror. It preserves
the source's API-key/base-URL inputs, defaults `max_retries` to zero so Hermes
owns retry and fallback policy, and honors an explicit retry override. Actual
OpenAI SDK/httpx construction, proxy/TLS bootstrap, and async transport remain
pending.

The transport-independent auxiliary proxy/TLS policy is included in the new
`a4a29c3` source commit and its `cc22417` GitHub mirror. It mirrors the source
proxy environment precedence, SOCKS normalization, lowercase-aware `NO_PROXY`
suffix bypass, TLS insecure-setting precedence, explicit/provider and CA-env
bundle ordering, user expansion, existing-file check, and default-certificate
fail-open behavior. Rust exposes these choices as `AuxiliaryTlsVerify`; actual
httpx client, `ssl.SSLContext`, SDK credential selection, and async transport
construction remain pending.

The Codex OAuth/Cloudflare credential-header helper is included in the new
`4d229c2` source commit and its `40fd571` GitHub mirror. It preserves the fixed
`codex_cli_rs` originator and User-Agent, URL-safe JWT account-ID extraction,
exact `ChatGPT-Account-ID` casing, and fail-open handling for empty, malformed,
or claim-less tokens. Concrete SDK client construction and credential-pool
selection remain pending.

The Codex access-token selection helper is included in the new `9a0bc98` source
commit and its `1550e03` GitHub mirror. It mirrors pool-first runtime-key
selection, trimmed Hermes auth-store fallback, strict JWT expiry filtering, and
fail-open use of malformed/non-JWT tokens. Pool selection and auth-store reads
are explicit Rust inputs; auth-file locking, credential-pool rotation, and
concrete SDK client construction remain pending.

The Qwen Portal bundled profile is included in the new `4e3894e` source
commit and its `41482fc` GitHub mirror. The `qwen_portal` capability mirrors
the source's string/list message normalization, unsupported-part filtering,
first-system-message `cache_control` injection, nested `image_url` retry-copy
guard, constant `vl_high_resolution_images=true` body field, and top-level
non-empty `qwen_session_metadata` mapping. The profile mirrors Qwen's three
aliases, `QWEN_API_KEY`, Portal URL, external OAuth auth type, and 65,536
default max-token cap; Qwen CLI credential resolution and full transport
integration remain future higher-layer seams.

The Custom/Ollama local bundled profile is included in the new `adf7332`
source commit and its `6e88a75` GitHub mirror. It mirrors Custom's six
aliases, empty user-configured endpoint fields, 65,536 default max-token
cap, `ollama_num_ctx` options body, disabled/`none` dual reasoning fields,
trimmed/lowercased top-level effort passthrough, empty-config omission,
and fail-open catalog guard until a base URL is configured. Broader CLI
custom endpoint and transport integration remain future higher-layer seams.

The Ollama Cloud bundled profile is included in the new `63db0cf` source
commit and its `1262f1f` GitHub mirror. It mirrors the profile metadata,
alias, API-key environment, base URL, auxiliary model, and the top-level
`reasoning_effort` hook. The `ollama_cloud_reasoning` capability preserves
native thinking-capability gating, disabled/`none` off switch,
xhigh/max/ultra normalization, standard effort passthrough, and unknown
effort omission without an extra-body reasoning field. The `/api/show`
probe, dynamic live+models.dev catalog merge, and hermes-cli
credential/model-picker integration remain future higher-layer seams.

The repository documentation checkpoint is included in the separate
`2523860` local commit and `a6c7179` GitHub mirror (`AGENTS.md`).

The new `hermes-providers` crate ports `providers/base.py` and
`providers/__init__.py` @ `b9aa928`: declarative profile defaults and hooks,
model endpoint precedence, strict fail-open catalog parsing,
credential-safe redirects, canonical/alias registry behavior, copy-safe
caching, and sorted bundled/user/legacy discovery. The focused suites contain
32 auxiliary-client routing/predicate/wire/error/pool-runtime/endpoint/client-option/proxy/TLS/Codex-header/token-selection tests,
9 base, 8 registry, 2 AI Gateway profile, 2 Alibaba profile, 2 Alibaba Coding
Plan profile, 3 Anthropic profile, 3 Gemini profile, 2 Arcee profile, 2 Azure
Foundry profile, 2 Bedrock profile, 3 Copilot profile, 2 Copilot ACP profile,
4 Custom profile, 2 Fireworks profile, 2 GMI profile, 2 Kilo Code profile,
3 Kimi Coding profile,
5 ZAI profile,
2 NovitaAI profile,
2 NVIDIA profile, 2 StepFun profile, 3 Vertex profile, 2 DeepInfra profile,
2 DeepSeek profile, 3 Nous profile, 3 Actual profile, 3 Ollama Cloud profile,
3 Minimax profile, 2 OpenAI Codex profile, 4 Qwen OAuth profile, 4 Upstage profile,
and 2 Hugging Face
profile tests are green.
The provider
surface remains partial
for the future CLI version/opener integration and remaining Rust plugin profile
loaders. `agent.auxiliary_client` remains partial; continue its client/
credential-resolution, transport, and fallback sections before promoting it.
The ledger's next missing production unit is `run_agent` (8,206 LOC), but the
active continuation seam is the partial auxiliary client.

The required auxiliary-client Codex-token-selection workspace verification was
green before the hook/docs commit. The hook-specific validation was also green:

```text
bash -n .githooks/pre-commit .githooks/post-commit tools/install_hooks.sh
python3 - <<'PY'
from pathlib import Path
for path in (Path('tools/refresh_docs.py'), Path('tools/pre_commit_docs.py'), Path('tools/sync_github_metadata.py')):
    compile(path.read_text(encoding='utf-8'), str(path), 'exec')
print('python helper syntax passed')
PY
HERMES_UPSTREAM=/run/media/mustbearnold/Projects/Research/hermes-agent-repo python3 tools/refresh_docs.py --upstream /run/media/mustbearnold/Projects/Research/hermes-agent-repo
env -u HERMES_GITHUB_TOKEN -u GH_TOKEN -u GITHUB_TOKEN python3 tools/sync_github_metadata.py --readme-mode skip
.githooks/pre-commit
git diff --check
/home/mustbearnold/.cargo/bin/cargo test -p hermes-agent --test parity_auxiliary_client
/home/mustbearnold/.cargo/bin/cargo test -p hermes-providers --test parity_zai --test parity_base --test parity_registry
/home/mustbearnold/.cargo/bin/cargo build --workspace
/home/mustbearnold/.cargo/bin/cargo test --workspace
```

The first full test attempt exposed a race among the existing
process-global hermes-constants platform-cache tests; serializing only those
cache-resetting tests is committed as `b2c1f4f`/`1682e21`. The final workspace
run passed all active tests; only three intentional delegation/schema doc
tests remain ignored. Earlier hermes-logging and hermes-constants test-state
hardening remains in the preceding synchronized history.

## Exact working-tree state

The agent-utility batch is committed with this documentation checkpoint and
**did change the ledger**: 79 done / 13 partial / 3,790 missing tracked
modules and 79 done / 13 partial / 1,011 missing production modules —
**2.04%** tracked and **7.16%** production strict completion — regenerated with
`HERMES_UPSTREAM=/run/media/mustbearnold/Projects/Research/hermes-agent-repo
tools/inventory.sh` and `python3 tools/conversion_ledger.py`, with
`cargo build --workspace` and the serialized workspace run green at 1,234 tests
/ 5 ignored.

0. Keep taking the small-module batch: `agent.turn_retry_state`,
   `agent.verify_hooks`, `agent.kanban_stop`, `agent.relay_tools`,
   `agent.manual_compression_feedback`, and
   `agent.async_utils` each have a dedicated upstream test module and no
   unported lower layer, so they port in the same TDD shape as session 4d3.
1. Continue the remaining `agent.auxiliary_client` request/response lifecycle:
   concrete client construction against these header/`extra_body` seams, xAI
   default headers, and the URL-inferred provider fallback used by the async
   conversion chain. Promote `agent.errors` usage into
   `resolve_auxiliary_tls_verify`'s failure path when `agent.ssl_guard` lands.
2a. Publish the ambient conversation id at the real fan-out sites
   (`agent/tool_executor`, MoA fan-out, background review) with
   `propagate_conversation_context_to_thread`, or register the capture with a
   composite context once the approval/gateway contextvars layer owns
   `hermes_tools::thread_context`'s snapshot factory.
2. Publish the ambient conversation id from the agent turn wrapper
   (`AIAgent._compress_context` / turn entry) once that surface is ported, and
   wire `agent.portal_tags` into `hermes_tools::thread_context`'s snapshot
   factory so propagated worker threads inherit it.
3. Add the full CLI default-catalog/managed write-back integration when the
   missing `hermes-cli` crate exists; managed-scope discovery itself is now
   ported in `hermes-agent::managed_scope`, so only the CLI-owned callers are
   still blocked. Keep ownership disjoint; commit and publish each logical
   unit immediately.

## Next actions, in order

1. Add the full CLI managed-overlay/default-catalog integration when the
   missing `hermes-cli` crate exists; this is the current higher-layer
   blocker, not a reason to move discovery into `hermes-agent::config`.
2. Continue provider-specific auth transport/cache orchestration, leases, and
   logging throttles through dependency-safe lower-layer seams.
3. Connect auxiliary-client concrete transport to credential-pool lifecycle,
   cancellation, and provider fallback without promoting either partial
   module prematurely. Keep ownership disjoint; commit and publish each
   logical unit immediately.

## Current conversion ledger

`CONVERSION-LEDGER.md` is generated and contains one row for every 3,882 upstream inventory modules: 1,103 production tasks plus 2,779 oracle/test tasks. Only `done` counts toward completion.

- All tracked modules: 79 done / 13 partial / 3,790 missing = **2.04%**.
- Production modules: 79 done / 13 partial / 1,011 missing = **7.16%**.

The thirteen partial production rows are `agent.auxiliary_client`,
`agent.credential_pool`, `agent.interrupt_compat`, `agent.portal_tags`,
`hermes_constants`, `providers.base`, `providers.__init__`,
`tools.credential_files`, `tools.delegation_output_schema`,
`tools.threat_patterns`, `tools.todo_tool`, `tools.tool_backend_helpers`,
and `tools.tool_output_limits`. Their closure
seams are listed in the ledger and PLAN.md. This session's batch added five
`done` rows (`agent.errors`, `agent.message_content`,
`agent.tool_result_classification`, `agent.lmstudio_reasoning`,
`agent.reasoning_summaries`) plus the `agent.interrupt_compat` partial.

Regenerate with:

```bash
HERMES_UPSTREAM=/run/media/mustbearnold/Projects/Research/hermes-agent-repo tools/inventory.sh
python3 tools/conversion_ledger.py
```

The strict production formula is `done production modules / 1,103`; partial rows receive zero credit. The all-module percentage is also shown because every upstream test/oracle task remains explicit.

## Fidelity notes

- Session 4d1 repairs (recorded so a later review does not re-introduce them):
  the managed-scope fold is one `config.yaml` per managed directory, not a
  sorted `*.yaml` drop-in merge; `MergedConfigSnapshot` exposes the real
  `managed_dir` (`Option<PathBuf>`) rather than the config path's parent; the
  user default-header overlay skips only explicit `null` values, never trims
  keys, and lets a non-empty `model.extra_headers` alias win; the OpenRouter
  cache gate is the union of the `_try_openrouter` route gate and the
  `_to_async_client` `base_url_host_matches(..., "openrouter.ai")` host gate.
- Approved Portal-attribution divergences: `agent.portal_tags` pins
  `hermes_client_tag` to the `b9aa928` `hermes_cli.__version__` value `0.20.0`
  instead of importing it live, which awaits the missing `hermes-cli` crate.
  The ambient conversation id is a Rust `thread_local!`, so a bare worker
  already behaves like a bare Python thread, and session 4d2 added the
  captured-context half (`ConversationContext` + the module-local propagate
  wrapper). Remaining deltas there: CPython raises `RuntimeError` when a
  `Context` is entered recursively or by two threads at once, while the port
  serializes both, and the wrapper covers this one variable until a composite
  approval/gateway context registers with
  `hermes_tools::thread_context`'s snapshot factory.
- Header-value stringification matches Python `str()` for scalars
  (`True`/`False`, `None` skipped) and renders list/dict values as JSON rather
  than Python `repr`; no upstream test pins the degenerate container shapes.
- OpenRouter TTL handling keeps upstream's bool-is-int quirk
  (`response_cache_ttl: true` emits `"1"`) and ASCII-digit-only
  `str.isdigit()` env parsing.
- The managed-scope port logs parse failures through the `log` facade without
  upstream's `exc_info` traceback, and its `(mtime_ns, size)` cache matches the
  merged-config snapshot cache identity upstream folds into the combined
  load-cache signature.

- The credential environment seeder, lower `load_pool` transaction, explicit
  full `load_pool` composition, and Anthropic/Nous/Copilot/Qwen/MiniMax/OpenAI
  Codex/xAI singleton seeders plus custom-pool config/model seeding keep the
  agent crate
  bottom-up by taking provider registry metadata, pool config, resolved
  singleton state, secret-scope values, suppression state, and auth-store paths
  as explicit inputs. Anthropic's provider/API-key-path gates and resolved
  `hermes_pkce`/`claude_code` credential-file outputs are represented in that
  seam. Kimi's pure key-prefix endpoint routing, Nous state-to-pool field copy,
  Copilot resolved exchanged-token/source/endpoint mapping, Qwen
  resolved-credential field copy, MiniMax OAuth state mapping, OpenAI Codex
  nested-token field copy, xAI nested-token field copy, and the pure
  custom-provider compatibility adapter are mirrored; outer config
  discovery/loading and the source's Z.AI network endpoint probe remain
  deferred to the auth/provider layer.
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
- Fireworks pins the upstream `HermesAgent/0.20.0` default header; dynamic
  `hermes_cli.__version__` injection remains an explicit future CLI seam.
- Anthropic's subclass model fetcher is represented by the explicit
  `ModelsFetchMode::Anthropic` capability: it requires a non-empty key, uses
  the fixed native endpoint and headers, and fails open on probe errors. A
  cloned-profile `models_url` override exists only as the loopback mock seam;
  the production profile still ignores caller `base_url`.
- Gemini's source `build_extra_body` override is represented by the explicit
  `gemini_thinking` capability; it mirrors model gating, effort clamping,
  native camelCase output, and the Google OpenAI-compatible nested snake_case
  body. Full native-client and transport integrations remain future crates.
- Copilot's source `build_api_kwargs_extras` override is represented by the
  explicit `copilot_reasoning` capability. The live
  `github_model_reasoning_efforts(model)` lookup is an injected context seam;
  catalog-gated clamp precedence and fail-open behavior are preserved.
- Vertex's source `build_extra_body` override is represented by the explicit
  `vertex_thinking` capability; Gemini model gating, effort clamping, and
  nested snake_case `extra_body.google.thinking_config` output are preserved.
  Its unconditional `fetch_models() -> None` override is represented by
  `models_fetch_disabled`; OAuth token resolution remains a future runtime
  adapter seam.
- DeepInfra's source `default_vision_model` override is represented by the
  explicit `deepinfra_vision` capability. It gates on the process environment
  until the profile-scoped secret context is available, filters the live
  catalog to chat+vision models with the upstream legacy exclusion fallback,
  and preserves positive/negative cache semantics; the higher-layer opener
  integration remains a future CLI seam.
- DeepSeek's source `build_api_kwargs_extras` override is represented by
  the explicit `deepseek_reasoning` capability. V4+ model gating, explicit
  enabled/disabled thinking, and low/medium/high versus max effort mapping
  are preserved; broader reasoning-content replay/history repair remains a
  future agent/transport seam.
- Nous's source `build_extra_body` and `build_api_kwargs_extras` overrides are
  represented by the explicit `nous_portal` capability. Product/client and
  conversation tags, cron timestamp normalization, truthy provider preference
  forwarding, reasoning defaults, and disabled omission are preserved. The
  pinned CLI version and `conversation_context` map key are explicit seams
  until the higher-layer runtime supplies the live version and ContextVar.
- Actual's source `fetch_models()` override is represented by the explicit
  `actual_catalog` capability. `ACTUAL_BASE_URL` precedence, hosted/local root
  `/v1` normalization, optional Bearer auth, JSON/Accept/User-Agent headers,
  list/`data` payload parsing, ID filtering, and fail-open errors are
  preserved; runtime credential/model-picker/transport integration remains a
  future hermes-cli seam.
- Ollama Cloud's source `build_api_kwargs_extras` override is represented by
  the explicit `ollama_cloud_reasoning` capability. It gates on
  `supports_reasoning`, emits only top-level `reasoning_effort`, maps
  xhigh/max/ultra to max, passes low/medium/high, uses none for
  disabled/explicit none, and omits blank/unknown efforts; `/api/show`/native
  capability probing and higher-layer catalog/credential/model picker
  integration remain future seams.
- MiniMax's source `build_api_kwargs_extras` override is represented by the
  explicit `minimax_reasoning` capability. It gates on the parsed
  `api.minimax.io/v1` path and normalized M3 model names, always emits
  `reasoning_split`, selects adaptive for any supplied config, selects
  disabled only for explicit `enabled=False`, and omits the thinking field
  when config is absent. Query-bearing `/v1` paths remain accepted per the
  upstream `urlparse(...).path` predicate; auxiliary/OAuth/transport seams
  remain future higher-layer integrations.
- Custom's source `build_api_kwargs_extras` and `fetch_models` overrides are
  represented by the explicit `custom_provider` capability. Truthy
  `ollama_num_ctx` maps to `extra_body.options.num_ctx`; disabled/`none`
  emits both top-level `reasoning_effort=none` and `think=false`; effort
  values are trimmed/lowercased and passed through; empty configs omit
  reasoning; and catalog probing fails open until a base URL is configured.
- Qwen's source selectively copies mutable nested `image_url` parts; the Rust
  `serde_json::Value` adapter owns its returned tree and clones the complete
  message value while preserving the same normalization, cache-control, and
  input-mutation contract.
- Upstage's source uses a deny-list substring predicate and identity comparison
  for `enabled is False`; the Rust capability preserves both, including
  medium defaulting for missing/empty effort, minimal omission, and high
  fallback for stronger or unknown effort values.
- Kimi Coding's source URL predicate is represented by the `kimi_coding`
  capability: only HTTPS api.kimi.com /coding and /coding/v1 without userinfo,
  non-default ports, queries, or fragments are confirmed; unconfirmed
  catalogs filter trimmed/lowercased k3 IDs, while confirmed /coding receives
  the /v1 normalization. The Rust malformed-URL test uses a cloned profile's
  models_url as the deterministic loopback equivalent of the upstream
  monkeypatched base fetcher.
- Z.AI's `ZaiProfile` behavior is represented by the `zai_reasoning`
  capability: the GLM version predicate is case/whitespace normalized and
  preserves the GLM 4.5+ threshold, while GLM-5.2 alias tokens are matched
  independently so vendor-prefixed IDs still receive the native top-level
  `reasoning_effort`. The Rust adapter emits the same `thinking` body and
  fail-open empty maps for unsupported/no-config cases.
- The auxiliary-client predicate, task-routing, and pool-runtime slices use
  explicit Rust adapter inputs for Python hidden state: `main_provider`,
  `AuxiliaryTaskConfig`, MoA aggregator results, provider-registry IDs,
  `AuxiliaryPoolEntry` projected credentials, the Nous inference override,
  OpenRouter/Nous credential-presence booleans, and `AuxiliaryError`
  status/type/message. This preserves the source classification, precedence,
  and normalization order while leaving secret-scope/key-env lookup,
  credential-pool JWT validation, client/transport/fallback lifecycle
  semantics pending with the partial module.

- Platform cache-resetting tests are serialized because the production WSL and
  container detectors intentionally cache for process lifetime; the mutex is
  test-only and does not change detector behavior.
- `tools/gen_computer_use_schema.py` discovers the upstream root via `HERMES_UPSTREAM` and has path fallbacks for this machine.
- `cargo fmt --all -- --check` reports many pre-existing unformatted foundation files outside this wave. Do not mass-reformat unrelated crates; use targeted formatting only if needed.

For the OAuth refresh/re-selection unit, the focused
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-agent --test
parity_credential_pool --test parity_credential_store --test
parity_auxiliary_client` run passed 121 tests, the full workspace build passed,
the serialized workspace test passed with 1,112 tests and 5 intentional
ignored tests, and the approved leaf gate recheck passed all 5 gates.

For the parallel config/Z.AI unit, `/home/mustbearnold/.cargo/bin/cargo test
-p hermes-agent --test parity_config --test parity_credential_pool --test
parity_credential_store --test parity_auxiliary_client` and
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-providers --test parity_zai
--test parity_registry --test parity_base` suites passed; the
config-to-pool integration contract passed. The workspace build and serialized
workspace test passed with 1,125 tests and 5 intentional ignored tests. The
config and Z.AI child gates each passed 3/3, and the parent integration gate
passed 3/3. Targeted rustfmt passed; concrete Z.AI HTTP/cache and full merged
CLI config behavior remain deferred.

## Verification evidence

For the current agent-utility batch, the red-first focused
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-agent --test parity_errors
--test parity_message_content --test parity_tool_result_classification --test
parity_lmstudio_reasoning --test parity_reasoning_summaries --test
parity_interrupt_compat` run passed 2 + 6 + 5 + 8 + 7 + 6 = 34 tests,
`/home/mustbearnold/.cargo/bin/cargo build --workspace` passed, and the
required serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace --
--test-threads=1` passed with 1,234 tests and the same 5 intentionally ignored
doc tests. Targeted `/home/mustbearnold/.cargo/bin/rustfmt --edition 2021
--check` over the workspace sources/tests is clean for the touched files,
`/home/mustbearnold/.cargo/bin/cargo clippy -p hermes-agent --all-targets`
reports only the two pre-existing `auxiliary_client` diagnostics, and
`git diff --check` passed.


For the current ambient-propagation unit, the red-first focused
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-agent --test
parity_portal_tags` run now passes 12 tests,
`/home/mustbearnold/.cargo/bin/cargo build --workspace` passed, and the
required serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace --
--test-threads=1` passed with 1,200 tests and the same 5 intentionally ignored
doc tests. Targeted
`/home/mustbearnold/.cargo/bin/rustfmt --edition 2021 --check` on the two
changed files is clean, targeted
`/home/mustbearnold/.cargo/bin/cargo clippy -p hermes-agent --all-targets`
reports only the two pre-existing `auxiliary_client` `too_many_arguments` and
`needless_lifetimes` diagnostics, and `git diff --check` passed. Ledger counts
are unchanged (no module-status change), so `tools/inventory.json`,
`CONVERSION-LEDGER.md`, and the README snapshot regenerate identically.


For the current Portal-attribution/managed-scope unit, the focused
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-agent --test
parity_auxiliary_client --test parity_config --test parity_portal_tags --test
parity_managed_scope --test parity_credential_pool --test
parity_credential_store` run passed 61 + 21 + 9 + 12 + 66 + 19 tests,
`/home/mustbearnold/.cargo/bin/cargo build --workspace` passed, and the
required serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace --
--test-threads=1` passed with 1,197 tests and the same 5 intentionally
ignored doc tests. Targeted
`/home/mustbearnold/.cargo/bin/rustfmt --edition 2021 --check` on the eight
changed Rust files is clean, targeted
`/home/mustbearnold/.cargo/bin/cargo clippy -p hermes-agent --all-targets`
reached only the two pre-existing `auxiliary_client` `too_many_arguments` and
`needless_lifetimes` diagnostics, and `git diff --check` passed. The first
focused run before this session's repairs was red on the invented
managed-directory test, which is what surfaced the fidelity defect.


For the current Qwen-singleton unit, targeted rustfmt, the focused
`/home/mustbearnold/.cargo/bin/cargo test -p hermes-agent --test parity_credential_pool`
run passed all 35 tests, and `/home/mustbearnold/.cargo/bin/cargo build
--workspace` passed. The required serialized `/home/mustbearnold/.cargo/bin/cargo
test --workspace -- --test-threads=1` passed; three delegation/schema doc tests
remain intentionally ignored. Targeted `hermes-agent` Clippy reached only the
pre-existing `auxiliary_client` `too_many_arguments` and `needless_lifetimes`
diagnostics. The approved credential-lifecycle leaf recheck passed all 4 gates.

For the synchronized credential-store unit, `/home/mustbearnold/.cargo/bin/cargo
test -p hermes-agent --test parity_credential_pool --test
parity_credential_store` passed 21 pool plus 15 persistence tests. The approved
credential-lifecycle leaf gates passed both the focused parity check and
`/home/mustbearnold/.cargo/bin/cargo build --workspace`. The required
serialized `/home/mustbearnold/.cargo/bin/cargo test --workspace
-- --test-threads=1` also passed. `cargo fmt --all -- --check` still reports
pre-existing formatting drift outside this wave, and targeted `hermes-agent`
Clippy still reports only the pre-existing `auxiliary_client`
`too_many_arguments` and `needless_lifetimes` diagnostics.

The focused provider parity suites passed 9 base, 8 registry, 4 Custom, 3 Actual, 3 Ollama
Cloud, 2 AI Gateway, 2 Alibaba, 2 Alibaba Coding Plan, 3 Anthropic, 3 Gemini,
2 Arcee, 2 Azure
Foundry, 2 Bedrock, 3 Copilot, 2 Copilot ACP, 2 Fireworks, 2 GMI, 2 Kilo
Code, 3 Kimi Coding, 2 NovitaAI, 2 NVIDIA, 2 StepFun, 3 Vertex, 2 DeepInfra,
41 auxiliary-client routing/predicate/wire/error/pool-runtime/endpoint/client-option/proxy/TLS/Codex-header/token-selection/client-construction, 5 ZAI profile,
2 DeepSeek, 3 Nous, 3 Minimax, 2 OpenAI Codex, 4 Qwen OAuth, 4 Upstage,
2 Xiaomi, 2 XAI, and 2 Hugging Face profile
tests. The
required workspace build and test also passed with the explicit cargo
toolchain; three
delegation/schema doc tests are intentionally ignored. Inventory and
conversion ledger were regenerated and now record 73 done / 11 partial / 1,019
missing production modules.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```
