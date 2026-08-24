# Hermes Agent Rust — Next-session handoff

Date: 2026-08-24 (Pacific/Auckland), session 4b3.

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

Latest synchronized unit: local source `0dfa448` → GitHub `df894bb`
(`plugins.model-providers.zai.__init__`, including current plan/inventory
metadata), after local handoff source `c7f93e5` → GitHub `db3df00`
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
GitHub `b1cb43a7` (`providers.__init__`), all at upstream `b9aa928`. `main`
is aligned to the fetched remote mirror; the API-authored SHA differs only
because it cannot preserve the local author/committer timestamps.

## What landed this session

Module-sized commits are complete through
`plugins.model-providers.zai.__init__`: `0dfa448` locally, mirrored as
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
loaders. The next dependency-safe unit is `agent.auxiliary_client` (10,044 LOC).

The required ZAI workspace verification was green before the synchronized
commit:

```text
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

After the current ZAI source commit is mirrored and this handoff commit is aligned
to its remote mirror, the working tree is clean. The committed metadata
includes `AGENTS.md`, `PLAN.md`, `tools/port_status.json`, generated
`tools/inventory.json`, `CONVERSION-LEDGER.md`, and this handoff. No code or
parity test is pending for the ZAI unit.

## Next actions, in order

1. Start `agent.auxiliary_client` (10,044 LOC) by reading its pinned
   source/tests and identifying the next dependency-safe Rust seam.
2. Keep the TDD/parity-test-first protocol and record any higher-layer
   adapter seams explicitly while adding the next module.
3. For every future module, commit and publish each logical unit immediately;
   use the connected GitHub API until local GitHub CLI authentication exists.

## Current conversion ledger

`CONVERSION-LEDGER.md` is generated and contains one row for every 3,882 upstream inventory modules: 1,103 production tasks plus 2,779 oracle/test tasks. Only `done` counts toward completion.

- All tracked modules: 73 done / 9 partial / 3,800 missing = **1.88%**.
- Production modules: 73 done / 9 partial / 1,021 missing = **6.62%**.

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

- Platform cache-resetting tests are serialized because the production WSL and
  container detectors intentionally cache for process lifetime; the mutex is
  test-only and does not change detector behavior.
- `tools/gen_computer_use_schema.py` discovers the upstream root via `HERMES_UPSTREAM` and has path fallbacks for this machine.
- `cargo fmt --all -- --check` reports many pre-existing unformatted foundation files outside this wave. Do not mass-reformat unrelated crates; use targeted formatting only if needed.

## Verification evidence

The focused provider parity suites passed 9 base, 8 registry, 4 Custom, 3 Actual, 3 Ollama
Cloud, 2 AI Gateway, 2 Alibaba, 2 Alibaba Coding Plan, 3 Anthropic, 3 Gemini,
2 Arcee, 2 Azure
Foundry, 2 Bedrock, 3 Copilot, 2 Copilot ACP, 2 Fireworks, 2 GMI, 2 Kilo
Code, 3 Kimi Coding, 2 NovitaAI, 2 NVIDIA, 2 StepFun, 3 Vertex, 2 DeepInfra,
5 ZAI profile,
2 DeepSeek, 3 Nous, 3 Minimax, 2 OpenAI Codex, 4 Qwen OAuth, 4 Upstage,
2 Xiaomi, 2 XAI, and 2 Hugging Face profile
tests. The
required workspace build and test also passed with the explicit cargo
toolchain; three
delegation/schema doc tests are intentionally ignored. Inventory and
conversion ledger were regenerated and now record 73 done / 9 partial / 1,021
missing production modules.

## First command tomorrow

```bash
git status --short
git fetch origin main
git rev-parse origin/main
git log --oneline -5
git ls-remote origin refs/heads/main
```
