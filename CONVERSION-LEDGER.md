# Hermes Agent → Hermes Agent Rust Conversion Ledger

**Current strict completion: 1.88% of all tracked upstream modules (73/3882).**
Production-only strict completion: **6.62%** (73/1103 production modules).

> This file is generated from `tools/inventory.json`; update the source ledger in `tools/port_status.json`, regenerate the inventory, then run `python3 tools/conversion_ledger.py`. Only `done` counts toward the percentage. `partial` is intentionally zero credit until its stated parity seams are closed.

## Current state

| Scope | Done | Partial | Missing | Strict completion | Lines |
|---|---:|---:|---:|---:|---:|
| All tracked modules | 73 | 10 | 3799 | 1.88% | 29,844/1,510,733 done LOC |
| Production modules | 73 | 10 | 1020 | 6.62% | 29,844/843,792 done LOC |

Inventory source: `/run/media/mustbearnold/Projects/Research/hermes-agent-repo` at `2026-08-24T15:04:43`.

## Definition of 100.00%

The conversion is complete only when all of these are true:

1. Every row below is `✅ done`; no production or oracle module remains `partial` or `missing`.
2. Each production module has Rust behavior, upstream-derived parity tests, and a line-by-line review of errors, fail-open paths, precedence, caching, and lifecycle semantics.
3. Each upstream test/oracle row has equivalent Rust coverage or an explicit, resolved reason why the behavior is covered elsewhere; no ignored test hides a parity gap.
4. `cargo build --workspace` and `cargo test --workspace` are green, with the exact commands and evidence tier recorded in `PLAN.md`.
5. All intentional divergences have been removed or explicitly signed off; config/provider/gateway/platform seams are wired, not merely injectable placeholders.
6. `PLAN.md`, `tools/port_status.json`, `tools/inventory.json`, and this ledger agree, and every logical unit is committed and pushed.

## Active partial modules

| Module | Phase | Upstream LOC | Required closure |
|---|---|---:|---|
| `agent.auxiliary_client` | P2 | 10,044 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `hermes_constants` | P1 | 1,481 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `providers.__init__` | P2 | 198 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `providers.base` | P2 | 238 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `tools.credential_files` | P2 | 530 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `tools.delegation_output_schema` | P2 | 151 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `tools.threat_patterns` | P2 | 284 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `tools.todo_tool` | P2 | 335 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `tools.tool_backend_helpers` | P2 | 311 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |
| `tools.tool_output_limits` | P2 | 110 | Close the module-specific seam documented in `PLAN.md` and its Rust module doc. |

## Recommended next production units

Work bottom-up by phase. The list is regenerated from missing production rows, with the current phase and largest modules first; completing a different unit is valid when its dependency boundary is better prepared.

| Order | Module | Phase | Upstream LOC | Task |
|---:|---|---|---:|---|
| 1 | `run_agent` | P2 | 8,206 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 2 | `tools.mcp_tool` | P2 | 7,530 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 3 | `agent.conversation_loop` | P2 | 7,524 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 4 | `agent.context_compressor` | P2 | 7,110 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 5 | `tools.browser_tool` | P2 | 5,098 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 6 | `agent.chat_completion_helpers` | P2 | 4,599 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 7 | `tools.approval` | P2 | 4,557 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 8 | `tools.skills_hub` | P2 | 4,432 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 9 | `tools.delegate_tool` | P2 | 4,342 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 10 | `agent.agent_runtime_helpers` | P2 | 4,077 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 11 | `agent.conversation_compression` | P2 | 4,035 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 12 | `tools.tts_tool` | P2 | 3,964 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 13 | `tools.terminal_tool` | P2 | 3,580 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 14 | `agent.model_metadata` | P2 | 3,370 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 15 | `tools.computer_use.cua_backend` | P2 | 3,295 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 16 | `optional-skills.migration.openclaw-migration.scripts.openclaw_to_hermes` | P2 | 3,286 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 17 | `agent.anthropic_adapter` | P2 | 3,177 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 18 | `agent.credential_pool` | P2 | 3,147 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 19 | `tools.transcription_tools` | P2 | 3,016 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |
| 20 | `tools.process_registry` | P2 | 2,937 | TDD against upstream module/tests, implement, review, build/test, update ledger, commit. |

## Operating protocol

1. Load this file, `PLAN.md` §5, `tools/inventory.json`, and the session log; take the next dependency-safe row.
2. Read the pinned upstream module and its tests. Write Rust parity tests first and label each `unit`, `mock`, or `live`.
3. Implement exact behavior. Keep `// PARITY:` references and document every intentional divergence in code and `PLAN.md`.
4. Run `cargo build --workspace` and `cargo test --workspace`; never record a red commit.
5. Set the module status in `tools/port_status.json`, run `HERMES_UPSTREAM=... tools/inventory.sh`, then run this generator.
6. Commit one logical module at most per commit and push it; append the exact evidence and next unit to `PLAN.md` §7.

## Complete upstream module task ledger

Every upstream Python module in the inventory has one row. Production rows are conversion tasks; test rows are parity-oracle tasks. A test row is not silently treated as complete merely because a production port exists.

| Module task | Kind | Phase | Upstream LOC | Status | Remaining action |
|---|---|---|---:|---|---|
| `acp_adapter.__init__` | production | P5 | 1 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.__main__` | production | P5 | 5 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.auth` | production | P5 | 79 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.edit_approval` | production | P5 | 338 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.entry` | production | P5 | 280 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.events` | production | P5 | 279 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.permissions` | production | P5 | 182 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.provenance` | production | P5 | 127 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.server` | production | P5 | 2,510 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.session` | production | P5 | 684 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `acp_adapter.tools` | production | P5 | 1,347 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.__init__` | production | P2 | 8 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.account_usage` | production | P2 | 902 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.agent_init` | production | P2 | 2,823 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.agent_runtime_helpers` | production | P2 | 4,077 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.anthropic_adapter` | production | P2 | 3,177 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.async_utils` | production | P2 | 84 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.aux_accounting` | production | P2 | 138 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.auxiliary_client` | production | P2 | 10,044 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `agent.azure_identity_adapter` | production | P2 | 571 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.backend_identity` | production | P2 | 204 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.background_review` | production | P2 | 1,081 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.battery` | production | P2 | 131 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.bedrock_adapter` | production | P2 | 1,573 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.billing_links` | production | P2 | 124 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.billing_usage` | production | P2 | 323 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.billing_view` | production | P2 | 511 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.bounded_response` | production | P2 | 148 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.browser_provider` | production | P2 | 177 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.browser_registry` | production | P2 | 192 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.chat_completion_helpers` | production | P2 | 4,599 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.codex_responses_adapter` | production | P2 | 1,590 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.codex_runtime` | production | P2 | 1,467 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.coding_context` | production | P2 | 916 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.context_breakdown` | production | P2 | 360 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.context_compressor` | production | P2 | 7,110 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.context_engine` | production | P2 | 489 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.context_references` | production | P2 | 605 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.conversation_compression` | production | P2 | 4,035 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.conversation_loop` | production | P2 | 7,524 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.copilot_acp_client` | production | P2 | 756 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.credential_persistence` | production | P2 | 174 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.credential_pool` | production | P2 | 3,147 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.credential_sources` | production | P2 | 443 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.credits_tracker` | production | P2 | 852 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.curator` | production | P2 | 2,019 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.curator_backup` | production | P2 | 757 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.delegation_context` | production | P2 | 161 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.display` | production | P2 | 1,547 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.error_classifier` | production | P2 | 1,842 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.errors` | production | P2 | 13 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.estop` | production | P2 | 167 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.file_safety` | production | P2 | 693 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `agent.gemini_native_adapter` | production | P2 | 1,127 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.gemini_schema` | production | P2 | 140 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.i18n` | production | P2 | 282 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.image_gen_provider` | production | P2 | 393 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.image_gen_registry` | production | P2 | 145 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.image_routing` | production | P2 | 821 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.insights` | production | P2 | 1,162 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.interrupt_compat` | production | P2 | 35 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.iteration_budget` | production | P2 | 62 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.jiter_preload` | production | P2 | 39 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.kanban_stop` | production | P2 | 108 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.learn_prompt` | production | P2 | 220 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.learning_graph` | production | P2 | 328 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.learning_graph_render` | production | P2 | 658 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.learning_mutations` | production | P2 | 206 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lmstudio_reasoning` | production | P2 | 60 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.__init__` | production | P2 | 106 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.cli` | production | P2 | 299 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.client` | production | P2 | 1,029 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.eventlog` | production | P2 | 233 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.install` | production | P2 | 412 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.manager` | production | P2 | 744 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.protocol` | production | P2 | 196 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.range_shift` | production | P2 | 149 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.reporter` | production | P2 | 130 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.servers` | production | P2 | 1,187 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.lsp.workspace` | production | P2 | 223 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.manual_compression_feedback` | production | P2 | 120 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.markdown_tables` | production | P2 | 309 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.memory_manager` | production | P2 | 1,241 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.memory_provider` | production | P2 | 357 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.message_content` | production | P2 | 50 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.message_sanitization` | production | P2 | 865 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.moa_loop` | production | P2 | 2,384 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.moa_trace` | production | P2 | 167 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.model_metadata` | production | P2 | 3,370 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.models_dev` | production | P2 | 903 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.__init__` | production | P2 | 29 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.cron_health` | production | P2 | 201 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.emitter` | production | P2 | 211 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.events` | production | P2 | 86 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.gateway_health` | production | P2 | 469 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.gateway_health_export` | production | P2 | 643 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.otlp_exporter` | production | P2 | 272 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.policy` | production | P2 | 57 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.monitoring.redaction` | production | P2 | 71 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.moonshot_schema` | production | P2 | 269 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.nous_rate_guard` | production | P2 | 325 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.onboarding` | production | P2 | 266 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.oneshot` | production | P2 | 158 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.outbound_webhooks` | production | P2 | 569 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.__init__` | production | P2 | 51 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.constants` | production | P2 | 167 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.generate.__init__` | production | P2 | 29 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.generate.atlas` | production | P2 | 1,183 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.generate.imagegen` | production | P2 | 251 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.generate.orchestrate` | production | P2 | 358 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.generate.prompts` | production | P2 | 183 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.manifest` | production | P2 | 165 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.render` | production | P2 | 682 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.state` | production | P2 | 81 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.pet.store` | production | P2 | 503 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.plugin_llm` | production | P2 | 1,046 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.portal_tags` | production | P2 | 144 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.process_bootstrap` | production | P2 | 227 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.prompt_builder` | production | P2 | 2,206 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.prompt_caching` | production | P2 | 394 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.proxy_sources.__init__` | production | P2 | 8 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.proxy_sources.iron_proxy` | production | P2 | 2,494 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.rate_limit_tracker` | production | P2 | 246 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.reactions` | production | P2 | 56 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.reasoning_summaries` | production | P2 | 67 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.reasoning_timeouts` | production | P2 | 231 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.redact` | production | P1 | 1,197 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `agent.relay_llm` | production | P2 | 1,239 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.relay_runtime` | production | P2 | 1,036 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.relay_tools` | production | P2 | 123 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.replay_cleanup` | production | P2 | 323 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.retry_utils` | production | P2 | 208 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.runtime_cwd` | production | P2 | 100 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_scope` | production | P2 | 293 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_sources.__init__` | production | P2 | 41 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_sources._cache` | production | P2 | 215 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_sources.base` | production | P2 | 336 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_sources.bitwarden` | production | P2 | 1,048 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_sources.command` | production | P2 | 501 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_sources.onepassword` | production | P2 | 682 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.secret_sources.registry` | production | P2 | 470 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.session_activity` | production | P1 | 106 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `agent.shell_hooks` | production | P2 | 930 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.skill_bundles` | production | P2 | 438 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.skill_commands` | production | P2 | 812 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.skill_preprocessing` | production | P2 | 144 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.skill_utils` | production | P2 | 934 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.ssl_guard` | production | P2 | 95 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.ssl_verify` | production | P2 | 63 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.stream_diag` | production | P2 | 280 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.stream_single_writer` | production | P2 | 70 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.subagent_lifecycle` | production | P2 | 540 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.subdirectory_hints` | production | P2 | 340 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.subscription_view` | production | P2 | 507 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.system_prompt` | production | P2 | 685 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.think_scrubber` | production | P2 | 396 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.thinking_timeout_guidance` | production | P2 | 136 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.thread_scoped_output` | production | P2 | 142 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.title_generator` | production | P2 | 402 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.tool_dispatch_helpers` | production | P2 | 732 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.tool_executor` | production | P2 | 2,403 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.tool_guardrails` | production | P2 | 632 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.tool_result_classification` | production | P2 | 40 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.trace_upload` | production | P2 | 398 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.trajectory` | production | P2 | 56 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transcription_provider` | production | P2 | 193 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transcription_registry` | production | P2 | 124 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.__init__` | production | P2 | 68 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.anthropic` | production | P2 | 251 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.base` | production | P2 | 89 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.bedrock` | production | P2 | 154 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.chat_completions` | production | P2 | 895 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.codex` | production | P2 | 672 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.codex_app_server` | production | P2 | 418 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.codex_app_server_session` | production | P2 | 1,292 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.codex_event_projector` | production | P2 | 314 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.hermes_tools_mcp_server` | production | P2 | 284 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.transports.types` | production | P2 | 174 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.tts_provider` | production | P2 | 274 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.tts_registry` | production | P2 | 134 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.turn_context` | production | P2 | 1,281 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.turn_finalizer` | production | P2 | 772 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.turn_retry_state` | production | P2 | 92 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.turn_summary` | production | P2 | 310 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.usage_pricing` | production | P2 | 1,432 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.verification_evidence` | production | P2 | 698 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.verification_stop` | production | P2 | 312 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.verify.__init__` | production | P2 | 38 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.verify.environment` | production | P2 | 75 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.verify.recipes` | production | P2 | 477 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.verify.runner` | production | P2 | 279 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.verify_hooks` | production | P2 | 69 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.vertex_adapter` | production | P2 | 228 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.video_gen_provider` | production | P2 | 590 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.video_gen_registry` | production | P2 | 133 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.web_search_provider` | production | P2 | 211 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `agent.web_search_registry` | production | P2 | 304 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `batch_runner` | production | P2 | 1,330 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cli` | production | P3 | 18,589 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.__init__` | production | P4 | 42 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.blueprint_catalog` | production | P4 | 713 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.executions` | production | P4 | 280 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.jobs` | production | P4 | 3,093 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.lifecycle_guard` | production | P4 | 714 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.monitor` | production | P4 | 212 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.notepad` | production | P4 | 181 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.scheduler` | production | P4 | 5,072 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.scheduler_provider` | production | P4 | 367 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.scripts.__init__` | production | P4 | 1 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.scripts.classify_items` | production | P4 | 226 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.suggestion_catalog` | production | P4 | 154 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `cron.suggestions` | production | P4 | 269 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.__init__` | production | P4 | 35 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.agent_cache_pressure` | production | P4 | 310 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.authz_mixin` | production | P4 | 888 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.builtin_hooks.__init__` | production | P4 | 1 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.cgroup_cleanup` | production | P4 | 81 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.channel_directory` | production | P4 | 658 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.code_skew` | production | P4 | 64 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.config` | production | P4 | 2,693 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.cwd_placeholder` | production | P4 | 49 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.dead_targets` | production | P4 | 143 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.delivery` | production | P4 | 646 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.delivery_ledger` | production | P4 | 374 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.display_config` | production | P4 | 311 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.drain_control` | production | P4 | 273 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.hooks` | production | P4 | 227 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.kanban_watchers` | production | P4 | 1,523 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.lifecycle_ledger` | production | P4 | 323 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.memory_monitor` | production | P4 | 230 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.message_timestamps` | production | P4 | 166 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.mirror` | production | P4 | 206 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.pairing` | production | P4 | 905 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platform_registry` | production | P4 | 381 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.__init__` | production | P4 | 45 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms._http_client_limits` | production | P4 | 84 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.api_server` | production | P4 | 7,284 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.base` | production | P4 | 6,870 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.bluebubbles` | production | P4 | 1,071 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.helpers` | production | P4 | 942 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.media_cache` | production | P4 | 202 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.msgraph_webhook` | production | P4 | 453 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.__init__` | production | P4 | 91 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.adapter` | production | P4 | 3,273 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.chunked_upload` | production | P4 | 602 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.constants` | production | P4 | 74 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.crypto` | production | P4 | 45 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.keyboards` | production | P4 | 461 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.onboard` | production | P4 | 220 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.qqbot.utils` | production | P4 | 71 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.signal` | production | P4 | 1,707 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.signal_format` | production | P4 | 140 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.signal_rate_limit` | production | P4 | 374 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.webhook` | production | P4 | 1,412 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.webhook_filters` | production | P4 | 302 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.weixin` | production | P4 | 2,419 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.whatsapp_cloud` | production | P4 | 2,111 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.whatsapp_common` | production | P4 | 552 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.yuanbao` | production | P4 | 5,298 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.yuanbao_media` | production | P4 | 665 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.yuanbao_proto` | production | P4 | 1,418 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.platforms.yuanbao_sticker` | production | P4 | 558 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.profile_routing` | production | P4 | 166 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.readiness` | production | P4 | 122 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.__init__` | production | P4 | 889 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.adapter` | production | P4 | 2,144 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.auth` | production | P4 | 168 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.command_manifest` | production | P4 | 145 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.descriptor` | production | P4 | 176 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.media` | production | P4 | 205 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.transport` | production | P4 | 143 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.relay.ws_transport` | production | P4 | 902 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.response_filters` | production | P4 | 147 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.restart` | production | P4 | 120 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.restart_loop_guard` | production | P4 | 150 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.rich_sent_store` | production | P4 | 83 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.run` | production | P4 | 27,659 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.runtime_footer` | production | P4 | 181 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.scale_to_zero` | production | P4 | 124 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.session` | production | P4 | 3,733 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.session_context` | production | P4 | 495 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.session_stall` | production | P4 | 121 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.session_state` | production | P4 | 476 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.shutdown_flush` | production | P4 | 321 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.shutdown_forensics` | production | P4 | 462 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.shutdown_watchdog` | production | P4 | 457 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.slash_access` | production | P4 | 229 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.slash_commands` | production | P4 | 5,706 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.status` | production | P4 | 2,260 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.status_phrases` | production | P4 | 227 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.sticker_cache` | production | P4 | 124 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.stream_consumer` | production | P4 | 2,410 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.stream_dispatch` | production | P4 | 132 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.stream_events` | production | P4 | 171 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.streaming_tts_consumer` | production | P4 | 423 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.systemd_notify` | production | P4 | 176 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.turn_context` | production | P4 | 131 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.turn_lease` | production | P4 | 352 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.wake` | production | P4 | 184 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `gateway.whatsapp_identity` | production | P4 | 206 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_bootstrap` | production | P2 | 239 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.__init__` | production | P3 | 92 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli._early_recovery` | production | P3 | 271 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli._parser` | production | P3 | 473 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli._scan_venv_blockers` | production | P3 | 166 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli._startup_fast` | production | P3 | 222 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli._subprocess_compat` | production | P3 | 464 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.active_sessions` | production | P3 | 426 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.agent_import` | production | P3 | 1,024 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.agent_plugins` | production | P3 | 498 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.approval_mode` | production | P3 | 87 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.approvals_suggest` | production | P3 | 487 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.approvals_test` | production | P3 | 178 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.auth` | production | P3 | 9,240 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.auth_commands` | production | P3 | 802 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.azure_detect` | production | P3 | 408 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.backup` | production | P3 | 1,904 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.bang_shell` | production | P3 | 212 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.banner` | production | P3 | 907 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.blueprint_cmd` | production | P3 | 323 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.browser_connect` | production | P3 | 423 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.build_info` | production | P3 | 51 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.bundles` | production | P3 | 229 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.callbacks` | production | P3 | 253 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.checkpoints` | production | P3 | 291 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.claw` | production | P3 | 809 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.cli_agent_setup_mixin` | production | P3 | 858 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.cli_billing_mixin` | production | P3 | 1,566 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.cli_commands_mixin` | production | P3 | 3,560 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.cli_output` | production | P3 | 77 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.clipboard` | production | P3 | 568 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.codex_models` | production | P3 | 255 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.codex_runtime_plugin_migration` | production | P3 | 757 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.codex_runtime_switch` | production | P3 | 279 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.colors` | production | P3 | 38 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.commands` | production | P3 | 2,260 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.completion` | production | P3 | 319 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.config` | production | P3 | 5,458 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.config_defaults` | production | P3 | 4,383 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.config_migrations` | production | P3 | 685 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.console_engine` | production | P3 | 1,636 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.container_boot` | production | P3 | 615 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.context_switch_guard` | production | P3 | 203 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.copilot_auth` | production | P3 | 693 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.credential_lifecycle` | production | P3 | 272 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.cron` | production | P3 | 588 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.curator` | production | P3 | 850 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.curses_ui` | production | P3 | 997 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.__init__` | production | P3 | 48 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.audit` | production | P3 | 95 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.base` | production | P3 | 306 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.cookies` | production | P3 | 338 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.login_page` | production | P3 | 534 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.middleware` | production | P3 | 591 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.native_flow` | production | P3 | 297 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.prefix` | production | P3 | 232 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.public_paths` | production | P3 | 60 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.registry` | production | P3 | 81 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.routes` | production | P3 | 964 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.token_auth` | production | P3 | 194 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_auth.ws_tickets` | production | P3 | 161 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_procs` | production | P3 | 458 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dashboard_register` | production | P3 | 427 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.debug` | production | P3 | 1,046 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.default_soul` | production | P3 | 76 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dep_ensure` | production | P3 | 165 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.diagnostics_upload` | production | P3 | 138 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dingtalk_auth` | production | P3 | 291 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.doctor` | production | P3 | 2,785 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.doctor_live` | production | P3 | 316 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.dump` | production | P3 | 449 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.env_loader` | production | P3 | 752 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.fallback_cmd` | production | P3 | 377 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.fallback_config` | production | P3 | 101 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.focus_view` | production | P3 | 166 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.gateway` | production | P3 | 7,539 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.gateway_enroll` | production | P3 | 277 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.gateway_windows` | production | P3 | 1,696 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.goals` | production | P3 | 2,133 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.gui_uninstall` | production | P3 | 306 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.heartbeat` | production | P3 | 332 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.hooks` | production | P3 | 434 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.init_command` | production | P3 | 150 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.input_sanitize` | production | P3 | 70 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.inventory` | production | P3 | 856 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.journey` | production | P3 | 357 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.kanban` | production | P3 | 3,236 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.kanban_db` | production | P3 | 10,378 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.kanban_decompose` | production | P3 | 468 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.kanban_diagnostics` | production | P3 | 1,133 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.kanban_specify` | production | P3 | 264 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.kanban_swarm` | production | P3 | 278 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.lifecycle` | production | P3 | 63 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.linux_desktop_entry` | production | P3 | 173 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.logs` | production | P3 | 397 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.main` | production | P3 | 12,620 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.managed_scope` | production | P3 | 214 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.managed_uv` | production | P3 | 1,304 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.mcp_catalog` | production | P3 | 831 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.mcp_config` | production | P3 | 1,135 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.mcp_picker` | production | P3 | 322 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.mcp_security` | production | P3 | 181 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.mcp_startup` | production | P3 | 265 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.mem_trim` | production | P3 | 255 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.memory_oauth` | production | P3 | 83 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.memory_setup` | production | P3 | 578 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.middleware` | production | P3 | 327 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.migrate` | production | P3 | 115 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.moa_cmd` | production | P3 | 152 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.moa_config` | production | P3 | 509 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.model_catalog` | production | P3 | 471 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.model_cost_guard` | production | P3 | 134 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.model_normalize` | production | P3 | 582 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.model_search` | production | P3 | 50 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.model_setup_flows` | production | P3 | 3,151 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.model_switch` | production | P3 | 3,203 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.models` | production | P3 | 5,453 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.nous_account` | production | P3 | 814 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.nous_auth_keepalive` | production | P3 | 189 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.nous_billing` | production | P3 | 675 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.nous_subscription` | production | P3 | 1,302 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.npm_engine` | production | P3 | 339 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.observability.__init__` | production | P3 | 31 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.observability.relay_runtime` | production | P3 | 14 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.observability.relay_shared_metrics` | production | P3 | 1,294 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.observability.shared_metrics` | production | P3 | 718 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.observability.shared_metrics_contract` | production | P3 | 976 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.observability.shared_metrics_subscriber` | production | P3 | 101 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.onepassword_secrets_cli` | production | P3 | 530 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.oneshot` | production | P3 | 502 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.pairing` | production | P3 | 120 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.partial_compress` | production | P3 | 324 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.pets` | production | P3 | 502 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.platforms` | production | P3 | 84 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.plugins` | production | P3 | 2,732 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.plugins_cmd` | production | P3 | 2,129 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.portal_cli` | production | P3 | 246 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.profile_describer` | production | P3 | 288 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.profile_distribution` | production | P3 | 782 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.profiles` | production | P3 | 2,262 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.projects_cmd` | production | P3 | 335 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.projects_db` | production | P3 | 782 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.prompt_size` | production | P3 | 375 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.prompt_stash` | production | P3 | 260 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.provider_catalog` | production | P3 | 181 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.providers` | production | P3 | 959 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy.__init__` | production | P3 | 20 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy.adapters.__init__` | production | P3 | 37 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy.adapters.base` | production | P3 | 108 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy.adapters.nous_portal` | production | P3 | 199 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy.adapters.xai` | production | P3 | 145 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy.cli` | production | P3 | 140 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy.server` | production | P3 | 298 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.proxy_cli` | production | P3 | 903 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.psutil_android` | production | P3 | 108 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.pt_input_extras` | production | P3 | 163 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.pty_bridge` | production | P3 | 293 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.pty_session` | production | P3 | 195 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.relaunch` | production | P3 | 205 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.route_identity` | production | P3 | 104 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.runtime_provider` | production | P3 | 2,298 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.secret_prompt` | production | P3 | 126 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.secrets_cli` | production | P3 | 745 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.security_advisories` | production | P3 | 453 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.security_audit` | production | P3 | 589 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.security_audit_startup` | production | P3 | 285 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.send_cmd` | production | P3 | 489 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.service_manager` | production | P3 | 1,125 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.session_export` | production | P3 | 317 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.session_export_html` | production | P3 | 870 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.session_export_md` | production | P3 | 279 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.session_filters` | production | P3 | 234 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.session_listing` | production | P3 | 117 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.session_recap` | production | P3 | 322 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.session_recovery` | production | P3 | 1,447 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.sessions_cmd` | production | P3 | 1,179 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.setup` | production | P3 | 3,645 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.setup_hidden_env` | production | P3 | 56 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.setup_whatsapp_cloud` | production | P3 | 541 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.skills_config` | production | P3 | 202 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.skills_hub` | production | P3 | 2,036 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.skin_cmd` | production | P3 | 108 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.skin_engine` | production | P3 | 1,068 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.slack_cli` | production | P3 | 282 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.slash_exec` | production | P3 | 272 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.sqlite_runtime` | production | P3 | 124 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.sqlite_safe_read` | production | P3 | 415 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.sqlite_util` | production | P3 | 49 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.status` | production | P3 | 724 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.stdio` | production | P3 | 251 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.__init__` | production | P3 | 18 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands._shared` | production | P3 | 29 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.acp` | production | P3 | 52 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.approvals` | production | P3 | 115 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.auth` | production | P3 | 98 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.backup` | production | P3 | 38 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.claw` | production | P3 | 92 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.config` | production | P3 | 68 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.console` | production | P3 | 18 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.cron` | production | P3 | 249 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.dashboard` | production | P3 | 214 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.debug` | production | P3 | 100 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.doctor` | production | P3 | 44 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.dump` | production | P3 | 28 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.gateway` | production | P3 | 355 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.gui` | production | P3 | 63 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.hooks` | production | P3 | 77 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.import_agent` | production | P3 | 49 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.import_cmd` | production | P3 | 31 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.insights` | production | P3 | 25 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.login` | production | P3 | 78 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.logout` | production | P3 | 28 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.logs` | production | P3 | 78 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.mcp` | production | P3 | 126 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.memory` | production | P3 | 53 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.model` | production | P3 | 62 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.monitoring` | production | P3 | 36 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.pairing` | production | P3 | 40 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.pause` | production | P3 | 70 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.plugins` | production | P3 | 109 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.profile` | production | P3 | 203 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.prompt_size` | production | P3 | 36 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.security` | production | P3 | 62 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.setup` | production | P3 | 67 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.skills` | production | P3 | 316 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.skin` | production | P3 | 30 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.slack` | production | P3 | 93 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.status` | production | P3 | 28 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.sync` | production | P3 | 99 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.tools` | production | P3 | 95 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.uninstall` | production | P3 | 46 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.update` | production | P3 | 76 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.verify` | production | P3 | 80 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.version` | production | P3 | 18 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.webhook` | production | P3 | 83 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.subcommands.whatsapp` | production | P3 | 22 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.suggestions_cmd` | production | P3 | 158 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.telegram_managed_bot` | production | P3 | 358 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.timefmt` | production | P3 | 30 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.timeouts` | production | P3 | 82 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.tips` | production | P3 | 485 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.tools_config` | production | P3 | 5,452 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.toolset_validation` | production | P3 | 74 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.uninstall` | production | P3 | 979 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.update_cmd` | production | P3 | 5,540 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.update_lock` | production | P3 | 289 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.urllib_security` | production | P3 | 139 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.vercel_auth` | production | P3 | 70 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.verify_cmd` | production | P3 | 178 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.voice` | production | P3 | 1,060 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_deps` | production | P3 | 153 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_git` | production | P3 | 713 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_models` | production | P3 | 725 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.__init__` | production | P3 | 8 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.cron` | production | P3 | 245 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.git` | production | P3 | 138 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.mcp` | production | P3 | 478 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.profiles` | production | P3 | 841 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.sessions` | production | P3 | 720 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.skills` | production | P3 | 490 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_routers.tools` | production | P3 | 736 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.web_server` | production | P3 | 17,812 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.webhook` | production | P3 | 307 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.win_pty_bridge` | production | P3 | 184 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.windows_ssh_runtime` | production | P3 | 508 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.write_approval_commands` | production | P3 | 209 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_cli.xai_retirement` | production | P3 | 274 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `hermes_constants` | production | P1 | 1,481 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `hermes_logging` | production | P1 | 800 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `hermes_state` | production | P1 | 9,996 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `hermes_state_common` | production | P1 | 614 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `hermes_state_portability` | production | P1 | 714 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `hermes_state_schema` | production | P1 | 1,126 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `hermes_state_search` | production | P1 | 2,305 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `hermes_time` | production | P1 | 135 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `mcp_serve` | production | P2 | 1,037 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `mini_swe_runner` | production | P2 | 732 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `model_tools` | production | P2 | 1,569 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `optional-skills.blockchain.evm.scripts.evm_client` | production | P2 | 1,508 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.blockchain.hyperliquid.scripts.hyperliquid_client` | production | P2 | 1,660 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.blockchain.solana.scripts.solana_client` | production | P2 | 698 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.creative.kanban-video-orchestrator.scripts.bootstrap_pipeline` | production | P2 | 499 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.creative.kanban-video-orchestrator.scripts.monitor` | production | P2 | 195 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.creative.meme-generation.scripts.generate_meme` | production | P2 | 470 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.creative.pixel-art.scripts.__init__` | production | P2 | 0 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.creative.pixel-art.scripts.palettes` | production | P2 | 167 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.creative.pixel-art.scripts.pixel_art` | production | P2 | 162 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.creative.pixel-art.scripts.pixel_art_video` | production | P2 | 345 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.devops.watchers.scripts._watermark` | production | P2 | 148 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.devops.watchers.scripts.watch_github` | production | P2 | 169 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.devops.watchers.scripts.watch_http_json` | production | P2 | 131 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.devops.watchers.scripts.watch_rss` | production | P2 | 121 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.finance.dcf-model.scripts.validate_dcf` | production | P2 | 291 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.finance.excel-author.scripts.recalc` | production | P2 | 88 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.finance.polymarket.scripts.polymarket` | production | P2 | 284 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.finance.stocks.scripts.stocks_client` | production | P2 | 755 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.health.fitness-nutrition.scripts.body_calc` | production | P2 | 210 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.health.fitness-nutrition.scripts.nutrition_search` | production | P2 | 85 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.mcp.fastmcp.scripts.scaffold_fastmcp` | production | P2 | 56 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.mcp.fastmcp.templates.api_wrapper` | production | P2 | 54 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.mcp.fastmcp.templates.database_server` | production | P2 | 77 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.mcp.fastmcp.templates.file_processor` | production | P2 | 55 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.mcp.mcp-oauth-remote-gateway.scripts.diagnose-oauth-mcp` | production | P2 | 178 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.migration.openclaw-migration.scripts.openclaw_to_hermes` | production | P2 | 3,286 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.mlops.training.trl-fine-tuning.templates.basic_grpo_training` | production | P2 | 228 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.productivity.canvas.scripts.canvas_api` | production | P2 | 160 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.productivity.memento-flashcards.scripts.memento_cards` | production | P2 | 353 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.productivity.memento-flashcards.scripts.youtube_quiz` | production | P2 | 88 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.productivity.telephony.scripts.telephony` | production | P2 | 1,343 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.darwinian-evolver.scripts.parrot_openrouter` | production | P2 | 218 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.darwinian-evolver.scripts.show_snapshot` | production | P2 | 92 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.darwinian-evolver.templates.custom_problem_template` | production | P2 | 240 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.domain-intel.scripts.domain_intel` | production | P2 | 397 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.drug-discovery.scripts.chembl_target` | production | P2 | 53 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.drug-discovery.scripts.ro5_screen` | production | P2 | 44 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts._http` | production | P2 | 82 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts._normalize` | production | P2 | 67 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.build_findings` | production | P2 | 221 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.entity_resolution` | production | P2 | 228 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_courtlistener` | production | P2 | 149 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_gdelt` | production | P2 | 161 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_icij_offshore` | production | P2 | 234 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_nyc_acris` | production | P2 | 203 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_ofac_sdn` | production | P2 | 175 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_opencorporates` | production | P2 | 191 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_sec_edgar` | production | P2 | 184 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_senate_ld` | production | P2 | 146 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_usaspending` | production | P2 | 170 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_wayback` | production | P2 | 142 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.fetch_wikipedia` | production | P2 | 266 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.osint-investigation.scripts.timing_analysis` | production | P2 | 252 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.pinecone-research.scripts.memory_manager` | production | P2 | 155 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.research.pinecone-research.scripts.rag_pipeline` | production | P2 | 156 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.godmode.scripts.auto_jailbreak` | production | P2 | 771 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.godmode.scripts.godmode_race` | production | P2 | 530 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.godmode.scripts.load_godmode` | production | P2 | 45 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.godmode.scripts.parseltongue` | production | P2 | 550 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.oss-forensics.scripts.evidence-store` | production | P2 | 313 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.autopilot` | production | P2 | 417 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.badbool` | production | P2 | 177 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.brokers` | production | P2 | 77 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.cdp` | production | P2 | 159 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.config` | production | P2 | 144 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.crypto` | production | P2 | 88 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.dossier` | production | P2 | 135 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.email_modes` | production | P2 | 76 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.emailer` | production | P2 | 342 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.ledger` | production | P2 | 170 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.legal` | production | P2 | 63 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.paths` | production | P2 | 79 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.pdd` | production | P2 | 914 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.registry` | production | P2 | 293 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.report` | production | P2 | 161 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.scan` | production | P2 | 32 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.storage` | production | P2 | 138 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.tiers` | production | P2 | 283 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.security.unbroker.scripts.vectors` | production | P2 | 53 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `optional-skills.web-development.cloudflare-temporary-deploy.scripts.parse_deploy_output` | production | P2 | 122 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.__init__` | production | P4 | 1 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.browser.browser_use.__init__` | production | P4 | 14 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.browser.browser_use.provider` | production | P4 | 324 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.browser.browserbase.__init__` | production | P4 | 15 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.browser.browserbase.provider` | production | P4 | 300 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.browser.firecrawl.__init__` | production | P4 | 16 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.browser.firecrawl.provider` | production | P4 | 174 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.context_engine.__init__` | production | P4 | 285 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.cron_providers.__init__` | production | P4 | 356 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.cron_providers.chronos.__init__` | production | P4 | 254 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.cron_providers.chronos._nas_client` | production | P4 | 123 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.cron_providers.chronos.verify` | production | P4 | 154 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.dashboard_auth.basic.__init__` | production | P4 | 491 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.dashboard_auth.drain.__init__` | production | P4 | 291 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.dashboard_auth.nous.__init__` | production | P4 | 671 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.dashboard_auth.self_hosted.__init__` | production | P4 | 862 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.disk-cleanup.__init__` | production | P4 | 316 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.disk-cleanup.disk_cleanup` | production | P4 | 611 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.__init__` | production | P4 | 103 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.audio_bridge` | production | P4 | 248 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.cli` | production | P4 | 476 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.meet_bot` | production | P4 | 862 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.node.__init__` | production | P4 | 54 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.node.cli` | production | P4 | 125 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.node.client` | production | P4 | 107 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.node.protocol` | production | P4 | 124 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.node.registry` | production | P4 | 112 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.node.server` | production | P4 | 200 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.process_manager` | production | P4 | 339 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.realtime.__init__` | production | P4 | 10 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.realtime.openai_client` | production | P4 | 332 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.google_meet.tools` | production | P4 | 348 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.hermes-achievements.dashboard.plugin_api` | production | P4 | 1,061 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.hermes-achievements.tests.test_achievement_engine` | oracle/test | P4 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `plugins.image_gen.deepinfra.__init__` | production | P4 | 336 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.image_gen.fal.__init__` | production | P4 | 211 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.image_gen.krea.__init__` | production | P4 | 744 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.image_gen.openai-codex.__init__` | production | P4 | 639 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.image_gen.openai.__init__` | production | P4 | 419 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.image_gen.openrouter.__init__` | production | P4 | 526 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.image_gen.xai.__init__` | production | P4 | 494 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.kanban.dashboard.plugin_api` | production | P4 | 2,862 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.__init__` | production | P4 | 461 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.byterover.__init__` | production | P4 | 449 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.config_schema` | production | P4 | 144 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.hindsight.__init__` | production | P4 | 2,232 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.hindsight.config_schema` | production | P4 | 76 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.holographic.__init__` | production | P4 | 462 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.holographic.holographic` | production | P4 | 290 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.holographic.retrieval` | production | P4 | 668 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.holographic.store` | production | P4 | 644 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.honcho.__init__` | production | P4 | 1,550 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.honcho.cli` | production | P4 | 1,967 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.honcho.client` | production | P4 | 1,113 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.honcho.config_schema` | production | P4 | 324 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.honcho.oauth` | production | P4 | 401 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.honcho.oauth_flow` | production | P4 | 656 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.honcho.session` | production | P4 | 1,447 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.mem0.__init__` | production | P4 | 628 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.mem0._backend` | production | P4 | 315 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.mem0._oss_providers` | production | P4 | 88 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.mem0._setup` | production | P4 | 1,001 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.openviking.__init__` | production | P4 | 5,212 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.query_rewrite` | production | P4 | 139 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.retaindb.__init__` | production | P4 | 804 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.memory.supermemory.__init__` | production | P4 | 1,053 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.model-providers.actual.__init__` | production | P4 | 89 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.ai-gateway.__init__` | production | P4 | 43 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.alibaba-coding-plan.__init__` | production | P4 | 21 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.alibaba.__init__` | production | P4 | 13 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.anthropic.__init__` | production | P4 | 54 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.arcee.__init__` | production | P4 | 13 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.azure-foundry.__init__` | production | P4 | 21 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.bedrock.__init__` | production | P4 | 30 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.copilot-acp.__init__` | production | P4 | 35 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.copilot.__init__` | production | P4 | 74 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.custom.__init__` | production | P4 | 103 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.deepinfra.__init__` | production | P4 | 81 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.deepseek.__init__` | production | P4 | 102 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.fireworks.__init__` | production | P4 | 46 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.gemini.__init__` | production | P4 | 61 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.gmi.__init__` | production | P4 | 32 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.huggingface.__init__` | production | P4 | 20 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.kilocode.__init__` | production | P4 | 14 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.kimi-coding.__init__` | production | P4 | 121 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.minimax.__init__` | production | P4 | 97 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.nous.__init__` | production | P4 | 88 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.novita.__init__` | production | P4 | 27 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.nvidia.__init__` | production | P4 | 21 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.ollama-cloud.__init__` | production | P4 | 89 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.openai-codex.__init__` | production | P4 | 15 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.opencode-zen.__init__` | production | P4 | 147 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.model-providers.openrouter.__init__` | production | P4 | 213 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.model-providers.qwen-oauth.__init__` | production | P4 | 108 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.stepfun.__init__` | production | P4 | 14 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.upstage.__init__` | production | P4 | 115 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.vertex.__init__` | production | P4 | 75 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.xai.__init__` | production | P4 | 17 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.xiaomi.__init__` | production | P4 | 16 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.model-providers.zai.__init__` | production | P4 | 127 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `plugins.observability.langfuse.__init__` | production | P4 | 1,137 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.observability.nemo_relay.__init__` | production | P4 | 1,023 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.a2a.__init__` | production | P4 | 138 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.a2a.adapter` | production | P4 | 1,272 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.a2a.protocol` | production | P4 | 842 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.a2a.security` | production | P4 | 372 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.a2a.tools` | production | P4 | 595 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.buzz.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.buzz.adapter` | production | P4 | 1,528 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.buzz.nostr_auth` | production | P4 | 230 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.dingtalk.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.dingtalk.adapter` | production | P4 | 1,930 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.discord.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.discord.adapter` | production | P4 | 10,150 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.discord.ffmpeg_utils` | production | P4 | 43 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.discord.recovery` | production | P4 | 112 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.discord.voice_mixer` | production | P4 | 387 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.email.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.email.adapter` | production | P4 | 1,318 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.feishu.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.feishu.adapter` | production | P4 | 5,895 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.feishu.feishu_comment` | production | P4 | 1,382 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.feishu.feishu_comment_rules` | production | P4 | 429 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.feishu.feishu_meeting_invite` | production | P4 | 212 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.google_chat.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.google_chat.adapter` | production | P4 | 3,738 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.google_chat.oauth` | production | P4 | 695 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.homeassistant.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.homeassistant.adapter` | production | P4 | 604 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.irc.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.irc.adapter` | production | P4 | 995 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.line.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.line.adapter` | production | P4 | 1,758 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.matrix.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.matrix.adapter` | production | P4 | 5,423 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.mattermost.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.mattermost.adapter` | production | P4 | 1,327 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.ntfy.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.ntfy.adapter` | production | P4 | 617 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.photon.__init__` | production | P4 | 4 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.photon.adapter` | production | P4 | 2,910 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.photon.auth` | production | P4 | 1,163 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.photon.cli` | production | P4 | 540 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.photon.sidecar_paths` | production | P4 | 141 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.raft.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.raft.adapter` | production | P4 | 852 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.simplex.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.simplex.adapter` | production | P4 | 1,382 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.slack.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.slack.adapter` | production | P4 | 9,100 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.slack.block_kit` | production | P4 | 688 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.sms.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.sms.adapter` | production | P4 | 536 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.teams.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.teams.adapter` | production | P4 | 1,537 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.telegram.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.telegram.adapter` | production | P4 | 10,241 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.telegram.telegram_ids` | production | P4 | 51 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.telegram.telegram_network` | production | P4 | 305 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.wecom.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.wecom.adapter` | production | P4 | 1,932 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.wecom.callback_adapter` | production | P4 | 484 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.wecom.wecom_crypto` | production | P4 | 142 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.whatsapp.__init__` | production | P4 | 3 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.platforms.whatsapp.adapter` | production | P4 | 1,918 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.plugin_utils` | production | P4 | 135 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.security-guidance.__init__` | production | P4 | 259 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.security-guidance.patterns` | production | P4 | 368 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.spotify.__init__` | production | P4 | 66 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.spotify.client` | production | P4 | 435 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.spotify.tools` | production | P4 | 454 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.__init__` | production | P4 | 23 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.cli` | production | P4 | 461 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.meetings` | production | P4 | 333 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.models` | production | P4 | 350 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.pipeline` | production | P4 | 701 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.runtime` | production | P4 | 135 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.store` | production | P4 | 193 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.teams_pipeline.subscriptions` | production | P4 | 249 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.video_gen.deepinfra.__init__` | production | P4 | 90 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.video_gen.fal.__init__` | production | P4 | 624 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `plugins.video_gen.xai.__init__` | production | P4 | 925 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `providers.__init__` | production | P2 | 198 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `providers.base` | production | P2 | 238 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `run_agent` | production | P2 | 8,206 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.add_contributor` | production | P2 | 103 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.analyze_livetest` | production | P2 | 114 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.audit_pr_attribution` | production | P2 | 147 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.benchmark_browser_eval` | production | P2 | 138 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.build_model_catalog` | production | P2 | 118 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.build_skills_index` | production | P2 | 459 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.check-windows-footguns` | production | P2 | 768 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.check_subprocess_stdin` | production | P2 | 227 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.assemble_review_comment` | production | P2 | 428 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.classify_changes` | production | P2 | 172 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.e2e_screenshot_status` | production | P2 | 155 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.emit_review_status` | production | P2 | 214 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.live_comment` | production | P2 | 671 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.lockfile_diff` | production | P2 | 174 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.publish_e2e_evidence` | production | P2 | 328 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.ci.timings_report` | production | P2 | 1,085 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.contributor_audit` | production | P2 | 493 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.discord-voice-doctor` | production | P2 | 396 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.docker_config_migrate` | production | P2 | 110 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.docker_rebootstrap_nous_session` | production | P2 | 227 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.generate_conformance_vectors` | production | P2 | 266 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.install_psutil_android` | production | P2 | 102 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.iso-certify` | production | P2 | 625 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.keystroke_diagnostic` | production | P2 | 81 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.lint_diff` | production | P2 | 207 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.micro_compaction_report` | production | P2 | 171 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.observability.gateway_health_export_probe` | production | P2 | 92 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.observability.otel_capture_collector` | production | P2 | 59 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.profile-tui` | production | P2 | 625 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.release` | production | P2 | 2,638 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.run_tests_parallel` | production | P2 | 1,142 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.sample_and_compress` | production | P2 | 409 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.sandbox.proxy` | production | P2 | 237 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.smoke_nemo_relay_shared_metrics` | production | P2 | 739 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.tool_search_livetest` | production | P2 | 553 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.tool_search_livetest2` | production | P2 | 218 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.tool_search_livetest_ue` | production | P2 | 296 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.tool_search_livetest_ue_disc` | production | P2 | 234 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.tool_search_livetest_ue_hard` | production | P2 | 308 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `scripts.toolperf_abeval.ab_eval` | production | P2 | 307 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `setup` | production | P2 | 74 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts._common` | production | P2 | 835 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.auto_fix_deps` | production | P2 | 225 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.check_deps` | production | P2 | 437 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.extract_schema` | production | P2 | 315 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.fetch_logs` | production | P2 | 157 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.hardware_check` | production | P2 | 497 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.health_check` | production | P2 | 223 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.run_batch` | production | P2 | 243 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.run_workflow` | production | P2 | 796 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.scripts.ws_monitor` | production | P2 | 267 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.creative.comfyui.tests.conftest` | oracle/test | oracle | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `skills.creative.comfyui.tests.test_check_deps` | oracle/test | oracle | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `skills.creative.comfyui.tests.test_cloud_integration` | oracle/test | oracle | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `skills.creative.comfyui.tests.test_common` | oracle/test | oracle | 443 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `skills.creative.comfyui.tests.test_extract_schema` | oracle/test | oracle | 184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `skills.creative.comfyui.tests.test_run_workflow` | oracle/test | oracle | 210 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `skills.creative.excalidraw.scripts.upload` | production | P2 | 133 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.github.github-auth.scripts.git-credential-token` | production | P2 | 65 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.media.youtube-content.scripts.fetch_transcript` | production | P2 | 124 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.__init__` | production | P2 | 1 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.accept_changes` | production | P2 | 135 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.comment` | production | P2 | 368 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.merge_runs` | production | P2 | 310 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.helpers.__init__` | production | P2 | 111 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.helpers.pptx_chart` | production | P2 | 170 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.helpers.pptx_slide` | production | P2 | 60 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.helpers.pptx_theme` | production | P2 | 114 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.soffice` | production | P2 | 192 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.validate` | production | P2 | 173 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.validators.__init__` | production | P2 | 15 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.validators.base` | production | P2 | 875 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.validators.docx` | production | P2 | 466 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.validators.pptx` | production | P2 | 441 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.docx.scripts.office.validators.redlining` | production | P2 | 299 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.google-workspace.scripts._hermes_home` | production | P2 | 42 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.google-workspace.scripts.google_api` | production | P2 | 1,225 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.google-workspace.scripts.gws_bridge` | production | P2 | 111 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.google-workspace.scripts.setup` | production | P2 | 514 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.maps.scripts.maps_client` | production | P2 | 1,297 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.ocr-and-documents.scripts.extract_marker` | production | P2 | 87 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.ocr-and-documents.scripts.extract_pymupdf` | production | P2 | 98 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.check_bounding_boxes` | production | P2 | 65 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.check_fillable_fields` | production | P2 | 11 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.convert_pdf_to_images` | production | P2 | 33 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.create_validation_image` | production | P2 | 37 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.extract_form_field_info` | production | P2 | 122 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.extract_form_structure` | production | P2 | 115 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.fill_fillable_fields` | production | P2 | 98 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.pdf.scripts.fill_pdf_form_with_annotations` | production | P2 | 107 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.__init__` | production | P2 | 0 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.add_slide` | production | P2 | 367 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.clean` | production | P2 | 309 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.helpers.__init__` | production | P2 | 111 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.helpers.pptx_chart` | production | P2 | 170 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.helpers.pptx_slide` | production | P2 | 60 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.helpers.pptx_theme` | production | P2 | 114 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.soffice` | production | P2 | 192 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.validate` | production | P2 | 173 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.validators.__init__` | production | P2 | 15 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.validators.base` | production | P2 | 875 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.validators.docx` | production | P2 | 466 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.validators.pptx` | production | P2 | 441 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.office.validators.redlining` | production | P2 | 299 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.powerpoint.scripts.thumbnail` | production | P2 | 313 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.xlsx.scripts.office.soffice` | production | P2 | 192 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.productivity.xlsx.scripts.recalc` | production | P2 | 308 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.research.arxiv.scripts.search_arxiv` | production | P2 | 114 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.research.grounded-citations.scripts._hermes_home` | production | P2 | 23 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `skills.research.grounded-citations.scripts.sources` | production | P2 | 678 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tests.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.conftest` | oracle/test | oracle | 32 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_approval_isolation` | oracle/test | oracle | 195 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_auth` | oracle/test | oracle | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_edit_approval` | oracle/test | oracle | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_entry` | oracle/test | oracle | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_events` | oracle/test | oracle | 255 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_mcp_e2e` | oracle/test | oracle | 313 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_named_provider_catalogs` | oracle/test | oracle | 158 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_permissions` | oracle/test | oracle | 189 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_ping_suppression` | oracle/test | oracle | 191 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_server` | oracle/test | oracle | 728 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_session` | oracle/test | oracle | 329 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_session_db_private_access` | oracle/test | oracle | 180 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_session_provenance` | oracle/test | oracle | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp.test_tools` | oracle/test | oracle | 274 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp_adapter.test_acp_commands` | oracle/test | P5 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp_adapter.test_acp_images` | oracle/test | P5 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp_adapter.test_acp_mcp_discovery` | oracle/test | P5 | 327 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.acp_adapter.test_detect_provider_entra` | oracle/test | P5 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.__init__` | oracle/test | P2 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.__init__` | oracle/test | P2 | 1 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp._mock_lsp_server` | oracle/test | P2 | 205 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_backend_gate` | oracle/test | P2 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_broken_set` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_client_e2e` | oracle/test | P2 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_delta_key` | oracle/test | P2 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_diagnostics_field` | oracle/test | P2 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_eventlog` | oracle/test | P2 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_install_and_lint_fixes` | oracle/test | P2 | 182 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_lifecycle` | oracle/test | P2 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_powershell_server` | oracle/test | P2 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_protocol` | oracle/test | P2 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_reporter` | oracle/test | P2 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_service` | oracle/test | P2 | 227 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_shell_linter_lsp_skip` | oracle/test | P2 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_stale_diagnostics` | oracle/test | P2 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.lsp.test_workspace` | oracle/test | P2 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_account_usage` | oracle/test | P2 | 229 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_adapter` | oracle/test | P2 | 1,884 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_billing_guidance` | oracle/test | P2 | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_keychain` | oracle/test | P2 | 261 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_kimi_signed_thinking_replay` | oracle/test | P2 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_kwargs_sanitize` | oracle/test | P2 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_mcp_prefix_strip` | oracle/test | P2 | 157 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_oauth_pkce` | oracle/test | P2 | 217 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_oauth_ua_prefix` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_output_field_leak` | oracle/test | P2 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_thinking_block_order` | oracle/test | P2 | 314 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_token_scope_isolation` | oracle/test | P2 | 254 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_anthropic_whitespace_text_blocks` | oracle/test | P2 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_api_content_sidecar` | oracle/test | P2 | 1,038 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_arcee_trinity_overrides` | oracle/test | P2 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_async_token_accounting` | oracle/test | P2 | 508 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_async_utils` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_aux_progress_streaming` | oracle/test | P2 | 359 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_anthropic_pool_fallback_regression` | oracle/test | P2 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client` | oracle/test | P2 | 4,490 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_anthropic_custom` | oracle/test | P2 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_azure_foundry` | oracle/test | P2 | 323 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_base_url_host_validation_52608` | oracle/test | P2 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_bootstrap_skew` | oracle/test | P2 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_proxy_env` | oracle/test | P2 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_resolve_dedup` | oracle/test | P2 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_ssl_verify` | oracle/test | P2 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_client_xai_oauth_recovery` | oracle/test | P2 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_compression_timeout_floor` | oracle/test | P2 | 158 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_concurrency` | oracle/test | P2 | 403 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_config_bridge` | oracle/test | P2 | 246 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_explicit_cancellation` | oracle/test | P2 | 622 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_main_first` | oracle/test | P2 | 559 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_named_custom_providers` | oracle/test | P2 | 410 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_relay` | oracle/test | P2 | 532 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_runtime_cache_key` | oracle/test | P2 | 154 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_transient_retry` | oracle/test | P2 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_transport_autodetect` | oracle/test | P2 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_auxiliary_user_default_headers` | oracle/test | P2 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_azure_identity_adapter` | oracle/test | P2 | 496 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_backend_identity` | oracle/test | P2 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_battery` | oracle/test | P2 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_bedrock_1m_context` | oracle/test | P2 | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_bedrock_adapter` | oracle/test | P2 | 1,159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_bedrock_empty_text_blocks` | oracle/test | P2 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_bedrock_integration` | oracle/test | P2 | 439 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_bedrock_interrupt_post_worker` | oracle/test | P2 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_billing_links` | oracle/test | P2 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_billing_usage` | oracle/test | P2 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_billing_view` | oracle/test | P2 | 346 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_bounded_response` | oracle/test | P2 | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_budget_reasoning_details_exclusion` | oracle/test | P2 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_cache_disabled_on_stubs` | oracle/test | P2 | 382 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_canon_args_memo_parity` | oracle/test | P2 | 330 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_cascading_interrupt_6600` | oracle/test | P2 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_chat_completion_helpers_provider_sort` | oracle/test | P2 | 13 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_cjk_token_estimation` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_close_interrupted_tool_sequence` | oracle/test | P2 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_codex_app_server_event_bridge` | oracle/test | P2 | 401 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_codex_app_server_persist` | oracle/test | P2 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_codex_cloudflare_headers` | oracle/test | P2 | 198 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_codex_gpt55_autoraise_notice` | oracle/test | P2 | 162 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_codex_responses_adapter` | oracle/test | P2 | 489 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_codex_runtime_live_events` | oracle/test | P2 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_codex_ttfb_watchdog` | oracle/test | P2 | 351 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_coding_context` | oracle/test | P2 | 355 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compaction_anti_thrash` | oracle/test | P2 | 205 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compaction_redaction_boundaries` | oracle/test | P2 | 223 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compress_context_progress_timeout` | oracle/test | P2 | 535 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compress_focus` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compress_signal_leak` | oracle/test | P2 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressed_summary_metadata` | oracle/test | P2 | 223 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_anti_thrash_persistence` | oracle/test | P2 | 205 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_anti_thrash_recovery` | oracle/test | P2 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_attempt_telemetry` | oracle/test | P2 | 151 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_concurrent_fork` | oracle/test | P2 | 1,832 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_count_warning_36908` | oracle/test | P2 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_fallback_budget` | oracle/test | P2 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_interrupt_protection` | oracle/test | P2 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_logging_session_context` | oracle/test | P2 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_max_attempts_config` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_orphan_recovery` | oracle/test | P2 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_progress` | oracle/test | P2 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_review_76354` | oracle/test | P2 | 599 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_rotation_state` | oracle/test | P2 | 728 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_small_ctx_threshold_floor` | oracle/test | P2 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compression_worker_isolation_76354` | oracle/test | P2 | 282 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_actionable_tail_anchor` | oracle/test | P2 | 185 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_assistant_tail_anchor` | oracle/test | P2 | 505 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_historical_media` | oracle/test | P2 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_image_tokens` | oracle/test | P2 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_media_stripping` | oracle/test | P2 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_tail_cut_oob_fix` | oracle/test | P2 | 180 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_tail_cut_tool_pair_floor` | oracle/test | P2 | 162 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_tool_call_budget` | oracle/test | P2 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_compressor_zero_user_guard` | oracle/test | P2 | 276 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_breakdown` | oracle/test | P2 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_compressor` | oracle/test | P2 | 3,184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_compressor_cross_session_guard` | oracle/test | P2 | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_compressor_session_end_clears_state` | oracle/test | P2 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_compressor_summary_continuity` | oracle/test | P2 | 390 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_compressor_temporal_anchoring` | oracle/test | P2 | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_compressor_zero_user_provenance` | oracle/test | P2 | 280 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_engine` | oracle/test | P2 | 347 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_engine_host_contract` | oracle/test | P2 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_engine_on_turn_complete_usage` | oracle/test | P2 | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_engine_select_context` | oracle/test | P2 | 253 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_references` | oracle/test | P2 | 249 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_refs_concurrent` | oracle/test | P2 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_context_route_mismatch` | oracle/test | P2 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_copilot_acp_client` | oracle/test | P2 | 234 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_copilot_acp_deprecation` | oracle/test | P2 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool` | oracle/test | P2 | 2,026 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_deferred_refresh` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_key_rotation` | oracle/test | P2 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_lease_refresh_reselect` | oracle/test | P2 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_no_entries_log_throttle` | oracle/test | P2 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_oat_authtype` | oracle/test | P2 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_oauth_writethrough` | oracle/test | P2 | 319 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_provider_boundary` | oracle/test | P2 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_quarantine_locking` | oracle/test | P2 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_routing` | oracle/test | P2 | 545 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_sole_cooldown` | oracle/test | P2 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credential_pool_unmatched_rotation_bound` | oracle/test | P2 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credits_cold_start` | oracle/test | P2 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credits_fixture_snapshot` | oracle/test | P2 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credits_policy` | oracle/test | P2 | 595 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credits_tracker` | oracle/test | P2 | 701 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_credits_view` | oracle/test | P2 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_cron_inline_api_call_62151` | oracle/test | P2 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_crossloop_client_cache` | oracle/test | P2 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_curator` | oracle/test | P2 | 778 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_curator_activity` | oracle/test | P2 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_curator_backup` | oracle/test | P2 | 460 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_curator_classification` | oracle/test | P2 | 522 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_curator_reports` | oracle/test | P2 | 208 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_cursor_optimizations_parity` | oracle/test | P2 | 275 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_custom_pool_mismatch_guard` | oracle/test | P2 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_custom_provider_extra_body` | oracle/test | P2 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_custom_provider_extra_body_matching` | oracle/test | P2 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_custom_providers_vision` | oracle/test | P2 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_deepseek_anthropic_thinking` | oracle/test | P2 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_direct_provider_url_detection` | oracle/test | P2 | 23 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_display` | oracle/test | P2 | 316 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_display_emoji` | oracle/test | P2 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_display_todo_progress` | oracle/test | P2 | 176 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_display_tool_failure` | oracle/test | P2 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_empty_tool_name_loop_dampening` | oracle/test | P2 | 249 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_endpoint_blackhole` | oracle/test | P2 | 291 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_engine_preflight_wire` | oracle/test | P2 | 191 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_error_classifier` | oracle/test | P2 | 1,085 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_external_skills` | oracle/test | P2 | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_external_skills_dirs_cache` | oracle/test | P2 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_failover_identity` | oracle/test | P2 | 354 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_file_safety` | oracle/test | P2 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_file_safety_container_mirror` | oracle/test | P2 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_file_safety_credentials` | oracle/test | P2 | 283 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_file_safety_cross_profile` | oracle/test | P2 | 184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_file_safety_sandbox_mirror` | oracle/test | P2 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_file_safety_session_state` | oracle/test | P2 | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_gateway_turn_sidecar` | oracle/test | P2 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_gemini_fast_fallback` | oracle/test | P2 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_gemini_free_tier_gate` | oracle/test | P2 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_gemini_native_adapter` | oracle/test | P2 | 311 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_gemini_schema` | oracle/test | P2 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_gemini_standard_key_guidance` | oracle/test | P2 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_ghost_skill_pruning` | oracle/test | P2 | 298 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_i18n` | oracle/test | P2 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_idle_compaction` | oracle/test | P2 | 41 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_idle_compaction_lock_and_guards` | oracle/test | P2 | 189 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_image_gen_registry` | oracle/test | P2 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_image_routing` | oracle/test | P2 | 508 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_insights` | oracle/test | P2 | 672 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_intent_ack_continuation` | oracle/test | P2 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_interrupt_compat` | oracle/test | P2 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_jiter_preload` | oracle/test | P2 | 25 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_kanban_stop` | oracle/test | P2 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_kimi_coding_anthropic_thinking` | oracle/test | P2 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_last_total_tokens` | oracle/test | P2 | 22 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_learn_prompt` | oracle/test | P2 | 111 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_learning_graph` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_learning_graph_render` | oracle/test | P2 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_learning_mutations` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_lmstudio_reasoning` | oracle/test | P2 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_local_probe_disk_cache` | oracle/test | P2 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_local_stream_timeout` | oracle/test | P2 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_manual_compression_feedback` | oracle/test | P2 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_markdown_tables` | oracle/test | P2 | 162 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_memory_async_sync` | oracle/test | P2 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_memory_boundary_commit` | oracle/test | P2 | 116 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_memory_provider` | oracle/test | P2 | 1,160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_memory_session_switch` | oracle/test | P2 | 249 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_memory_skill_scaffolding` | oracle/test | P2 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_memory_user_id` | oracle/test | P2 | 281 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_memory_write_bridge` | oracle/test | P2 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_message_content` | oracle/test | P2 | 25 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_message_sanitization_policy` | oracle/test | P2 | 296 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_micro_compaction` | oracle/test | P2 | 823 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_minimax_auxiliary_url` | oracle/test | P2 | 27 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_minimax_provider` | oracle/test | P2 | 329 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_aggregator_cache_control` | oracle/test | P2 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_aggregator_cost_slot` | oracle/test | P2 | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_cold_start_cache_66793` | oracle/test | P2 | 226 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_context_max_tokens` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_progress` | oracle/test | P2 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_quiet_reference_output` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_reasoning_effort` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_reference_system_prompt` | oracle/test | P2 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_slot_api_mode` | oracle/test | P2 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_slot_max_tokens` | oracle/test | P2 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_switch_api_mode` | oracle/test | P2 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moa_trace_streamed_capture` | oracle/test | P2 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_model_extra_type_guard` | oracle/test | P2 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_model_metadata` | oracle/test | P2 | 1,360 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_model_metadata_local_ctx` | oracle/test | P2 | 669 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_model_metadata_ssl` | oracle/test | P2 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_models_dev` | oracle/test | P2 | 392 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_moonshot_schema` | oracle/test | P2 | 395 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_non_stream_stale_timeout` | oracle/test | P2 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_none_deref_guards` | oracle/test | P2 | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_nous_credits_gauge` | oracle/test | P2 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_nous_credits_snapshot` | oracle/test | P2 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_nous_oauth_401_guidance` | oracle/test | P2 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_nous_portal_anthropic_wire` | oracle/test | P2 | 524 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_nous_rate_guard` | oracle/test | P2 | 284 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_onboarding` | oracle/test | P2 | 227 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_oneshot` | oracle/test | P2 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_openrouter_response_cache` | oracle/test | P2 | 146 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_org_skill_namespace` | oracle/test | P2 | 389 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_outbound_webhooks` | oracle/test | P2 | 530 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_pet_engine` | oracle/test | P2 | 255 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_pet_generate` | oracle/test | P2 | 329 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_platform_hint_desktop` | oracle/test | P2 | 179 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_platform_hint_overrides` | oracle/test | P2 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_plugin_llm` | oracle/test | P2 | 655 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_portal_tags` | oracle/test | P2 | 177 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_post_compression_trim` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_pre_compress_memory_context` | oracle/test | P2 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_preflight_compression_gate` | oracle/test | P2 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_preflight_lock_defer` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_proactive_prune_config` | oracle/test | P2 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_proactive_prune_restart_safety` | oracle/test | P2 | 273 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_proactive_tool_result_pruning` | oracle/test | P2 | 241 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_probe_cache_followups` | oracle/test | P2 | 281 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_prompt_builder` | oracle/test | P2 | 923 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_prompt_caching` | oracle/test | P2 | 469 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_protected_tail_pressure_61932` | oracle/test | P2 | 206 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_proxy_and_url_validation` | oracle/test | P2 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_rate_limit_tracker` | oracle/test | P2 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_reactions` | oracle/test | P2 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_reasoning_stale_timeout_floor` | oracle/test | P2 | 208 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_reasoning_summaries` | oracle/test | P2 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_redact` | oracle/test | P2 | 1,053 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_reference_handoff_active_turn` | oracle/test | P2 | 184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_refine_focus` | oracle/test | P2 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_relay_llm` | oracle/test | P2 | 871 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_relay_tools` | oracle/test | P2 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_replay_cleanup` | oracle/test | P2 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_request_client_reuse` | oracle/test | P2 | 167 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_restore_primary_pool_reselect` | oracle/test | P2 | 167 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_resume_stale_active_task` | oracle/test | P2 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_rotation_flush_persisted_boundary_68196` | oracle/test | P2 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_runtime_cwd` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_save_url_image` | oracle/test | P2 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_secret_scope` | oracle/test | P2 | 289 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_secret_scope_tier1_migration` | oracle/test | P2 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_send_path_history_isolation` | oracle/test | P2 | 209 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_session_activity` | oracle/test | P2 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_session_rotation_flush_cold_resume_68454` | oracle/test | P2 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_set_runtime_main_custom_provider` | oracle/test | P2 | 231 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_shell_hooks` | oracle/test | P2 | 421 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_shell_hooks_consent` | oracle/test | P2 | 224 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_skill_bundles` | oracle/test | P2 | 298 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_skill_commands` | oracle/test | P2 | 692 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_skill_commands_reload` | oracle/test | P2 | 111 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_skill_invocation_description` | oracle/test | P2 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_skill_utils` | oracle/test | P2 | 304 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_skip_background_review` | oracle/test | P2 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_skip_memory_store_65429` | oracle/test | P2 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_ssl_ca_guard` | oracle/test | P2 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_ssl_verify` | oracle/test | P2 | 32 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_stream_chunk_byte_estimate` | oracle/test | P2 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_stream_read_timeout_floor` | oracle/test | P2 | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_stream_single_writer_guard` | oracle/test | P2 | 61 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_streaming_context_scrubber` | oracle/test | P2 | 184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_subagent_lifecycle` | oracle/test | P2 | 174 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_subagent_progress` | oracle/test | P2 | 268 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_subagent_stop_hook` | oracle/test | P2 | 263 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_subdirectory_hints` | oracle/test | P2 | 286 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_subdirectory_hints_tilde` | oracle/test | P2 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_subprocess_env_guard` | oracle/test | P2 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_subscription_view` | oracle/test | P2 | 252 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_summarize_tool_result_type_safety` | oracle/test | P2 | 162 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_summary_prefix_semantics` | oracle/test | P2 | 250 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_summary_prefix_tool_use` | oracle/test | P2 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_summary_role_template_alternation` | oracle/test | P2 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_synthetic_turn_display_kind` | oracle/test | P2 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_system_prompt` | oracle/test | P2 | 268 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_system_prompt_restore` | oracle/test | P2 | 383 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_think_scrubber` | oracle/test | P2 | 191 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_thinking_timeout_guidance` | oracle/test | P2 | 185 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_thread_scoped_output` | oracle/test | P2 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_title_generator` | oracle/test | P2 | 320 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_tool_call_arg_no_redaction` | oracle/test | P2 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_tool_dispatch_helpers` | oracle/test | P2 | 211 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_tool_executor_checkpoint_paths` | oracle/test | P2 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_tool_guardrails` | oracle/test | P2 | 179 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_tool_result_classification` | oracle/test | P2 | 31 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_trace_upload` | oracle/test | P2 | 167 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_transcription_registry` | oracle/test | P2 | 187 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_tts_registry` | oracle/test | P2 | 219 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_context` | oracle/test | P2 | 377 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_context_overflow_warning` | oracle/test | P2 | 264 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_finalizer_cleanup_guard` | oracle/test | P2 | 166 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_finalizer_final_response_persistence` | oracle/test | P2 | 225 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_finalizer_interrupt_alternation` | oracle/test | P2 | 182 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_finalizer_iteration_limit_exit` | oracle/test | P2 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_overlap_tripwire` | oracle/test | P2 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_retry_state` | oracle/test | P2 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_turn_summary` | oracle/test | P2 | 230 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_unsupported_parameter_retry` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_unsupported_temperature_retry` | oracle/test | P2 | 246 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_usage_pricing` | oracle/test | P2 | 314 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_verification_evidence` | oracle/test | P2 | 212 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_verification_evidence_fd_leak` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_verification_stop` | oracle/test | P2 | 235 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_verification_stop_caching` | oracle/test | P2 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_verify_hooks` | oracle/test | P2 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_vertex_adapter` | oracle/test | P2 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_video_gen_registry` | oracle/test | P2 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_vision_resolved_args` | oracle/test | P2 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.test_vision_routing_31179` | oracle/test | P2 | 258 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.__init__` | oracle/test | P2 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_bedrock_transport` | oracle/test | P2 | 146 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_chat_completions` | oracle/test | P2 | 752 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_codex_app_server_runtime` | oracle/test | P2 | 342 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_codex_app_server_session` | oracle/test | P2 | 898 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_codex_event_projector` | oracle/test | P2 | 279 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_codex_transport` | oracle/test | P2 | 831 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_hermes_tools_mcp_server` | oracle/test | P2 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_transport` | oracle/test | P2 | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.agent.transports.test_types` | oracle/test | P2 | 200 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.ci.test_assemble_review_comment` | oracle/test | oracle | 328 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.ci.test_classify_changes` | oracle/test | oracle | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.ci.test_e2e_screenshot_status` | oracle/test | oracle | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.ci.test_emit_review_status` | oracle/test | oracle | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.ci.test_lockfile_diff` | oracle/test | oracle | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.ci.test_publish_e2e_evidence` | oracle/test | oracle | 191 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.ci.test_timings_report` | oracle/test | oracle | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.__init__` | oracle/test | P3 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.conftest` | oracle/test | P3 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_bang_shell_mode` | oracle/test | P3 | 313 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_bracketed_paste_timeout` | oracle/test | P3 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_branch_command` | oracle/test | P3 | 183 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_busy_input_mode_command` | oracle/test | P3 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_chat_q_exit_clear` | oracle/test | P3 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_active_agent_ref_wiring` | oracle/test | P3 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_approval_ui` | oracle/test | P3 | 564 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_async_delegation_delivery` | oracle/test | P3 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_background_busy_path` | oracle/test | P3 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_background_status_indicator` | oracle/test | P3 | 176 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_background_tui_refresh` | oracle/test | P3 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_bracketed_paste_sanitizer` | oracle/test | P3 | 34 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_browser_connect` | oracle/test | P3 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_cmd_backspace` | oracle/test | P3 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_codex_context_reference` | oracle/test | P3 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_context_warning` | oracle/test | P3 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_copy_command` | oracle/test | P3 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_delegate_background_notice` | oracle/test | P3 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_extension_hooks` | oracle/test | P3 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_external_editor` | oracle/test | P3 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_file_drop` | oracle/test | P3 | 204 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_first_run_setup` | oracle/test | P3 | 252 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_force_redraw` | oracle/test | P3 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_goal_interrupt` | oracle/test | P3 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_image_command` | oracle/test | P3 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_init` | oracle/test | P3 | 527 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_insights_command` | oracle/test | P3 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_interrupt_ack_race` | oracle/test | P3 | 538 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_interrupt_drain_regression` | oracle/test | P3 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_interrupt_subagent` | oracle/test | P3 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_light_mode` | oracle/test | P3 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_loading_indicator` | oracle/test | P3 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_markdown_rendering` | oracle/test | P3 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_mcp_config_watch` | oracle/test | P3 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_new_session` | oracle/test | P3 | 299 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_pet_pane` | oracle/test | P3 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_prefix_matching` | oracle/test | P3 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_preloaded_skills` | oracle/test | P3 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_provider_resolution` | oracle/test | P3 | 595 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_queue_paste` | oracle/test | P3 | 22 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_reload_skills` | oracle/test | P3 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_resume_command` | oracle/test | P3 | 266 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_retry` | oracle/test | P3 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_save_config_value` | oracle/test | P3 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_secret_capture` | oracle/test | P3 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_shift_enter_newline` | oracle/test | P3 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_shutdown_memory_messages` | oracle/test | P3 | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_skin_integration` | oracle/test | P3 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_status_bar` | oracle/test | P3 | 321 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_status_bar_goal` | oracle/test | P3 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_status_command` | oracle/test | P3 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_steer_busy_path` | oracle/test | P3 | 146 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_terminal_response_sanitizer` | oracle/test | P3 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_terminal_shortcuts` | oracle/test | P3 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_tools_command` | oracle/test | P3 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_user_message_preview` | oracle/test | P3 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_yolo_resume_persistence` | oracle/test | P3 | 251 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cli_yolo_toggle` | oracle/test | P3 | 197 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_compress_flags` | oracle/test | P3 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_compress_focus` | oracle/test | P3 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_compress_here` | oracle/test | P3 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_compress_type_ahead` | oracle/test | P3 | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cpr_local_leak` | oracle/test | P3 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cprint_bg_thread` | oracle/test | P3 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_ctrl_enter_newline` | oracle/test | P3 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_cwd_env_respect` | oracle/test | P3 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_destructive_slash_confirm` | oracle/test | P3 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_destructive_slash_inline_skip_e2e` | oracle/test | P3 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_exit_delete_session` | oracle/test | P3 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_exit_summary_resume_hint` | oracle/test | P3 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_exit_watchdog_signal_arm` | oracle/test | P3 | 111 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_fast_command` | oracle/test | P3 | 325 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_focus_view` | oracle/test | P3 | 392 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_indicator_command` | oracle/test | P3 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_manual_compress` | oracle/test | P3 | 262 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_moa_command` | oracle/test | P3 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_partial_compress` | oracle/test | P3 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_personality_none` | oracle/test | P3 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_prefill_config` | oracle/test | P3 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_prepend_note_to_message` | oracle/test | P3 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_prompt_stash` | oracle/test | P3 | 436 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_prompt_stash_cli` | oracle/test | P3 | 289 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_prompt_text_input_thread_safety` | oracle/test | P3 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_quick_commands` | oracle/test | P3 | 177 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_reasoning_command` | oracle/test | P3 | 645 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_resume_display` | oracle/test | P3 | 409 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_resume_quiet_stderr` | oracle/test | P3 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_save_conversation_location` | oracle/test | P3 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_session_boundary_hooks` | oracle/test | P3 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_single_query_session_finalize` | oracle/test | P3 | 203 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_slash_command_interrupt` | oracle/test | P3 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_slash_confirm_windows` | oracle/test | P3 | 327 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_steer_inline_repaint_34569` | oracle/test | P3 | 116 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_stream_delta_think_tag` | oracle/test | P3 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_stream_flush_left` | oracle/test | P3 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_stream_partial_line_flush` | oracle/test | P3 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_surrogate_sanitization` | oracle/test | P3 | 231 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_terminal_interrupt_recovery` | oracle/test | P3 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_tool_progress_scrollback` | oracle/test | P3 | 197 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_tui_terminal_reset_on_exit` | oracle/test | P3 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_update_command` | oracle/test | P3 | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_version_command` | oracle/test | P3 | 28 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_worktree` | oracle/test | P3 | 1,184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_worktree_security` | oracle/test | P3 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cli.test_worktree_sync_base` | oracle/test | P3 | 219 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.live_cua_0_9_smoke` | oracle/test | oracle | 471 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_cua_atexit_teardown` | oracle/test | oracle | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_cua_cli_fallback_env` | oracle/test | oracle | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_cua_no_overlay` | oracle/test | oracle | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_cua_perf_knobs` | oracle/test | oracle | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_cua_spawn_env_sanitization` | oracle/test | oracle | 226 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_cua_telemetry` | oracle/test | oracle | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_cua_wsl_manifest_path` | oracle/test | oracle | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_doctor` | oracle/test | oracle | 431 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.computer_use.test_permissions_resolution` | oracle/test | oracle | 32 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.conformance.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.conformance.test_vector_generator` | oracle/test | oracle | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.conftest` | oracle/test | oracle | 1,542 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.conftest` | oracle/test | P4 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_blueprint_catalog` | oracle/test | P4 | 204 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_claim_job_for_fire` | oracle/test | P4 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_codex_execution_paths` | oracle/test | P4 | 192 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_compute_next_run_last_run_at` | oracle/test | P4 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_context_from` | oracle/test | P4 | 219 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_direct_api_call_62151` | oracle/test | P4 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_direct_api_call_watchdog` | oracle/test | P4 | 384 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_inactivity_timeout` | oracle/test | P4 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_kanban_env_isolation` | oracle/test | P4 | 446 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_no_agent` | oracle/test | P4 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_profile_isolation` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_prompt_injection_skill` | oracle/test | P4 | 350 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_provider_pin` | oracle/test | P4 | 396 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_script` | oracle/test | P4 | 431 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cron_workdir` | oracle/test | P4 | 253 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_cronjob_schema` | oracle/test | P4 | 21 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_execution_ledger` | oracle/test | P4 | 399 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_file_permissions` | oracle/test | P4 | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_idle_tick_config_skip` | oracle/test | P4 | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_jobs` | oracle/test | P4 | 1,252 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_jobs_changed_notify` | oracle/test | P4 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_jobs_crossprocess_lock` | oracle/test | P4 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_jobs_file_ownership` | oracle/test | P4 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_jobs_shrink_merge_80624` | oracle/test | P4 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_monitor_kind` | oracle/test | P4 | 364 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_notepad` | oracle/test | P4 | 229 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_parallel_pool` | oracle/test | P4 | 221 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_preflight_config` | oracle/test | P4 | 342 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_reasoning_config_per_model` | oracle/test | P4 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_rewrite_skill_refs` | oracle/test | P4 | 265 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_run_one_job` | oracle/test | P4 | 111 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_scheduler` | oracle/test | P4 | 1,978 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_scheduler_cron_session_isolation` | oracle/test | P4 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_scheduler_mcp_init` | oracle/test | P4 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_scheduler_provider` | oracle/test | P4 | 416 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_scheduler_shutdown_guard` | oracle/test | P4 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_script_claim_heartbeat` | oracle/test | P4 | 165 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_sessiondb_init_hang` | oracle/test | P4 | 247 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_shutdown_interrupt` | oracle/test | P4 | 249 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_suggestions` | oracle/test | P4 | 200 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_terminal_cwd_lock` | oracle/test | P4 | 300 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_ticker_stall_60703` | oracle/test | P4 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.cron.test_usage_audit_logger` | oracle/test | P4 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.dashboard.test_ws_client_host` | oracle/test | oracle | 180 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.conftest` | oracle/test | oracle | 313 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_config_migration` | oracle/test | oracle | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_container_restart` | oracle/test | oracle | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_dashboard` | oracle/test | oracle | 227 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_docker_exec_privilege_drop` | oracle/test | oracle | 226 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_dump_build_sha` | oracle/test | oracle | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_gateway_bootstrap_state` | oracle/test | oracle | 198 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_gateway_run_supervised` | oracle/test | oracle | 296 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_home_override_scripts` | oracle/test | oracle | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_immutable_install` | oracle/test | oracle | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_immutable_install_permissions` | oracle/test | oracle | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_license_file_present` | oracle/test | oracle | 26 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_log_dir_seed` | oracle/test | oracle | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_main_invocation` | oracle/test | oracle | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_profile_gateway` | oracle/test | oracle | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_puid_pgid_remap` | oracle/test | oracle | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_s6_profile_gateway_integration` | oracle/test | oracle | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_smoke` | oracle/test | oracle | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_sqlite_runtime` | oracle/test | oracle | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_stage2_browser_discovery` | oracle/test | oracle | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_tini_compat_shim` | oracle/test | oracle | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_toplevel_chown` | oracle/test | oracle | 182 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_tui_passthrough` | oracle/test | oracle | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_tui_prebuilt_bundle` | oracle/test | oracle | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_user_flag_guard` | oracle/test | oracle | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.docker.test_zombie_reaping` | oracle/test | oracle | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.e2e.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.e2e.conftest` | oracle/test | oracle | 450 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.e2e.matrix_xsign_bootstrap.test_bootstrap` | oracle/test | oracle | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.e2e.test_discord_adapter` | oracle/test | oracle | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.e2e.test_platform_commands` | oracle/test | oracle | 247 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.e2e.test_relay_native_anthropic_stream` | oracle/test | oracle | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.fakes.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.fakes.fake_ha_server` | oracle/test | oracle | 301 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.fixtures.plugins.example-dashboard.dashboard.plugin_api` | oracle/test | oracle | 24 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway._plugin_adapter_loader` | oracle/test | P4 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.conftest` | oracle/test | P4 | 554 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.feishu_helpers` | oracle/test | P4 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.platforms.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.platforms.test_yuanbao_recall_db_only` | oracle/test | P4 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.platforms.test_yuanbao_state_cleanup` | oracle/test | P4 | 174 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.stub_connector` | oracle/test | P4 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_auth` | oracle/test | P4 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_channel_context_consume` | oracle/test | P4 | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_contract_doc_conformance` | oracle/test | P4 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_descriptor` | oracle/test | P4 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_descriptor_from_entry` | oracle/test | P4 | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_handoff_relay_aliasing` | oracle/test | P4 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_identity_token_resolver` | oracle/test | P4 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_no_stub_leak` | oracle/test | P4 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_adapter` | oracle/test | P4 | 322 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_follow_up` | oracle/test | P4 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_going_idle` | oracle/test | P4 | 264 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_interactive` | oracle/test | P4 | 259 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_interrupt` | oracle/test | P4 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_media` | oracle/test | P4 | 178 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_multiplatform` | oracle/test | P4 | 154 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_passthrough` | oracle/test | P4 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_per_platform_caps` | oracle/test | P4 | 157 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_policy_send` | oracle/test | P4 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_registration` | oracle/test | P4 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_roundtrip` | oracle/test | P4 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_roundtrip_telegram` | oracle/test | P4 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_sheds_crypto` | oracle/test | P4 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_slack_dm_streaming` | oracle/test | P4 | 403 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_slack_prompt_dm_root` | oracle/test | P4 | 486 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_relay_threads` | oracle/test | P4 | 482 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_self_provision` | oracle/test | P4 | 259 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_wire_user_identity` | oracle/test | P4 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.relay.test_ws_transport` | oracle/test | P4 | 210 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.restart_test_helpers` | oracle/test | P4 | 165 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_10710_auto_reset_evicts_cached_agent` | oracle/test | P4 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_13121_shutdown_inflight_transcript_flush` | oracle/test | P4 | 189 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_25107_stale_base_url_api_mode` | oracle/test | P4 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_35809_auto_reset_clean_context` | oracle/test | P4 | 199 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_35994_reset_button_deadlock` | oracle/test | P4 | 200 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_42039_duplicate_user_message` | oracle/test | P4 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_48031_model_switch_after_auto_reset` | oracle/test | P4 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_53175_cleanup_off_loop` | oracle/test | P4 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_64674_multiplex_primary_token_scope` | oracle/test | P4 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_7100_transient_failure_transcript` | oracle/test | P4 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_71671_faulthandler_no_stderr` | oracle/test | P4 | 34 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_73297_memory_flush_on_reset` | oracle/test | P4 | 143 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_73771_media_resend_dedup` | oracle/test | P4 | 309 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_75349_whatsapp_multiplex_secret_scope` | oracle/test | P4 | 158 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_abandoned_turn_process_cleanup` | oracle/test | P4 | 234 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_active_session_text_merge` | oracle/test | P4 | 264 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_active_turn_recovery` | oracle/test | P4 | 515 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_adapter_connect_is_reconnect_contract` | oracle/test | P4 | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_adapter_startup_secret_scope` | oracle/test | P4 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_agent_cache` | oracle/test | P4 | 1,064 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_agent_cache_pressure` | oracle/test | P4 | 510 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_agents_command_delegations` | oracle/test | P4 | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_aiohttp_body_caps` | oracle/test | P4 | 20 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_allowed_channels_widening` | oracle/test | P4 | 222 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_allowlist_startup_check` | oracle/test | P4 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server` | oracle/test | P4 | 2,862 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_active_work_drain` | oracle/test | P4 | 608 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_bind_guard` | oracle/test | P4 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_jobs` | oracle/test | P4 | 500 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_media_data_urls` | oracle/test | P4 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_multimodal` | oracle/test | P4 | 165 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_multiplex_secret_scope` | oracle/test | P4 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_normalize` | oracle/test | P4 | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_runs` | oracle/test | P4 | 639 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_api_server_toolset` | oracle/test | P4 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_approval_prompt_redaction` | oracle/test | P4 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_approvals_command` | oracle/test | P4 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_approve_deny_commands` | oracle/test | P4 | 686 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_async_delegation_session_binding` | oracle/test | P4 | 219 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_async_delivery_capability` | oracle/test | P4 | 212 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_async_session_db` | oracle/test | P4 | 322 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_async_session_store` | oracle/test | P4 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_audio_cache` | oracle/test | P4 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_auth_fallback` | oracle/test | P4 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_auto_continue` | oracle/test | P4 | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_auto_voice_reply_format` | oracle/test | P4 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_background_command` | oracle/test | P4 | 211 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_background_process_notifications` | oracle/test | P4 | 338 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_base_auto_tts_output_format` | oracle/test | P4 | 143 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_base_topic_sessions` | oracle/test | P4 | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_bluebubbles` | oracle/test | P4 | 440 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_bounded_adapter_teardown` | oracle/test | P4 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_bundles_command` | oracle/test | P4 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_busy_session_ack` | oracle/test | P4 | 473 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_busy_session_auth_bypass` | oracle/test | P4 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_buzz_adapter` | oracle/test | P4 | 540 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_buzz_websocket` | oracle/test | P4 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cached_agent_max_iterations` | oracle/test | P4 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cancel_background_drain` | oracle/test | P4 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cgroup_cleanup` | oracle/test | P4 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_channel_continuity_hint` | oracle/test | P4 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_channel_directory` | oracle/test | P4 | 379 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_channel_directory_connected_only` | oracle/test | P4 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_channel_overrides` | oracle/test | P4 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_checkpoint_config` | oracle/test | P4 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_choice_picker` | oracle/test | P4 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cjk_fts_config_bridge` | oracle/test | P4 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_clarify_active_session_bypass` | oracle/test | P4 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_clarify_progress_leak` | oracle/test | P4 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_clarify_thread_followup_not_swallowed` | oracle/test | P4 | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_clean_shutdown_marker` | oracle/test | P4 | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_code_fence_tracking` | oracle/test | P4 | 463 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_command_bypass_active_session` | oracle/test | P4 | 441 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_complete_path_at_filter` | oracle/test | P4 | 265 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_completion_delivery` | oracle/test | P4 | 318 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compress_command` | oracle/test | P4 | 518 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compress_focus` | oracle/test | P4 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compress_plugin_engine` | oracle/test | P4 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compress_preview` | oracle/test | P4 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compression_concurrent_sessions` | oracle/test | P4 | 189 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compression_deferred_soft_result` | oracle/test | P4 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compression_failure_session_sync` | oracle/test | P4 | 285 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compression_in_flight_check` | oracle/test | P4 | 61 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compression_interrupt_demotion_56391` | oracle/test | P4 | 157 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compression_progress_notices` | oracle/test | P4 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_compression_session_id_persistence` | oracle/test | P4 | 184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_config` | oracle/test | P4 | 1,186 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_config_cwd_bridge` | oracle/test | P4 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_config_driven_access_policy` | oracle/test | P4 | 262 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_config_env_bridge_authority` | oracle/test | P4 | 216 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_context_ref_expansion_runtime` | oracle/test | P4 | 192 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_conversation_scope_funnel` | oracle/test | P4 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cron_active_work_drain` | oracle/test | P4 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cron_fire_webhook` | oracle/test | P4 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cron_shutdown_drain` | oracle/test | P4 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_cwd_placeholder` | oracle/test | P4 | 26 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_dead_targets` | oracle/test | P4 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_debug_command` | oracle/test | P4 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_dedupe_user_turns` | oracle/test | P4 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_delegation_session_id_leak` | oracle/test | P4 | 143 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_delivery` | oracle/test | P4 | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_delivery_ledger` | oracle/test | P4 | 299 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_delivery_ledger_fd_leak` | oracle/test | P4 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_delivery_ledger_producer` | oracle/test | P4 | 184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_delivery_silence_filter` | oracle/test | P4 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_destructive_slash_always_persist_report` | oracle/test | P4 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_destructive_slash_confirm` | oracle/test | P4 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_diff_command` | oracle/test | P4 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_dingtalk` | oracle/test | P4 | 767 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_allowed_channels` | oracle/test | P4 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_allowed_mentions` | oracle/test | P4 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_approval_mentions` | oracle/test | P4 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_attachment_download` | oracle/test | P4 | 258 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_bot_auth_bypass` | oracle/test | P4 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_bot_filter` | oracle/test | P4 | 146 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_channel_controls` | oracle/test | P4 | 242 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_channel_prompts` | oracle/test | P4 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_channel_skills` | oracle/test | P4 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_clarify_buttons` | oracle/test | P4 | 296 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_component_auth` | oracle/test | P4 | 280 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_connect` | oracle/test | P4 | 624 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_document_handling` | oracle/test | P4 | 353 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_double_dispatch` | oracle/test | P4 | 346 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_edit_message_overflow` | oracle/test | P4 | 314 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_exec_approval_content` | oracle/test | P4 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_fail_closed_feedback` | oracle/test | P4 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_format` | oracle/test | P4 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_free_response` | oracle/test | P4 | 829 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_imports` | oracle/test | P4 | 26 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_lazy_install_views` | oracle/test | P4 | 51 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_liveness` | oracle/test | P4 | 301 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_media_metadata` | oracle/test | P4 | 9 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_missed_message_backfill` | oracle/test | P4 | 460 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_model_picker` | oracle/test | P4 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_opus` | oracle/test | P4 | 17 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_pending_text_batch_shutdown` | oracle/test | P4 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_plugin_setup` | oracle/test | P4 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_prompt_content_siblings` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_prompt_timeout_config` | oracle/test | P4 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_race_polish` | oracle/test | P4 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_reactions` | oracle/test | P4 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_reply_mode` | oracle/test | P4 | 317 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_roles_dm_scope` | oracle/test | P4 | 206 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_send` | oracle/test | P4 | 420 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_slash_auth` | oracle/test | P4 | 552 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_slash_commands` | oracle/test | P4 | 604 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_sync_limit` | oracle/test | P4 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_system_messages` | oracle/test | P4 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_thread_persistence` | oracle/test | P4 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_thread_slash_expired_defer` | oracle/test | P4 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_discord_voice_mixer` | oracle/test | P4 | 165 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_display_config` | oracle/test | P4 | 304 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_dm_topics` | oracle/test | P4 | 480 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_document_cache` | oracle/test | P4 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_document_context_note` | oracle/test | P4 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_duplicate_reply_suppression` | oracle/test | P4 | 322 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_email` | oracle/test | P4 | 911 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_email_robustness` | oracle/test | P4 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_email_secret_scope` | oracle/test | P4 | 248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_empty_model_recovery` | oracle/test | P4 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_env_flag_truthy` | oracle/test | P4 | 23 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_ephemeral_reply` | oracle/test | P4 | 248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_escape_reasoning_fences` | oracle/test | P4 | 29 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_external_drain_control` | oracle/test | P4 | 213 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_extract_local_files` | oracle/test | P4 | 224 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_fallback_chain_reload` | oracle/test | P4 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_fallback_eviction` | oracle/test | P4 | 26 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_fast_command` | oracle/test | P4 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu` | oracle/test | P4 | 2,469 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_approval_buttons` | oracle/test | P4 | 449 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_bot_admission` | oracle/test | P4 | 568 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_bot_auth_bypass` | oracle/test | P4 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_channel_prompts` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_comment` | oracle/test | P4 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_comment_rules` | oracle/test | P4 | 158 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_lazy_import` | oracle/test | P4 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_meeting_invite` | oracle/test | P4 | 216 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_onboard` | oracle/test | P4 | 260 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_sdk_executor` | oracle/test | P4 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_table_markdown` | oracle/test | P4 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_feishu_voice_message_type` | oracle/test | P4 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_fence_chunker` | oracle/test | P4 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_first_turn_session_meta_rebaseline` | oracle/test | P4 | 220 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_footer_command_mid_run` | oracle/test | P4 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_fresh_reset_skill_injection` | oracle/test | P4 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_command_dispatch_minimal` | oracle/test | P4 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_command_help` | oracle/test | P4 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_command_line_matcher` | oracle/test | P4 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_inactivity_timeout` | oracle/test | P4 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_process_exit` | oracle/test | P4 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_shutdown` | oracle/test | P4 | 273 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_silence_tokens` | oracle/test | P4 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_gateway_utf8_encoding` | oracle/test | P4 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_goal_continuation_drain` | oracle/test | P4 | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_goal_max_turns_config` | oracle/test | P4 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_goal_status_notice` | oracle/test | P4 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_goal_verdict_send` | oracle/test | P4 | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_google_chat` | oracle/test | P4 | 1,742 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_google_chat_oauth_dependencies` | oracle/test | P4 | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_handoff_thread_session_key` | oracle/test | P4 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_handoff_watcher_async_db` | oracle/test | P4 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_history_media_current_turn` | oracle/test | P4 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_home_target_env_var` | oracle/test | P4 | 21 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_homeassistant` | oracle/test | P4 | 320 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_hooks` | oracle/test | P4 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_hygiene_failure_cooldown_ladder` | oracle/test | P4 | 374 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_image_input_routing_runtime` | oracle/test | P4 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_incomplete_gateway_turns` | oracle/test | P4 | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_insights_unicode_flags` | oracle/test | P4 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_interactive_prompt_base` | oracle/test | P4 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_internal_event_bypass_pairing` | oracle/test | P4 | 215 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_internal_event_never_interrupts_busy_session` | oracle/test | P4 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_interrupt_key_match` | oracle/test | P4 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_irc_adapter` | oracle/test | P4 | 408 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_kanban_auto_decompose_live` | oracle/test | P4 | 29 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_kanban_notifier` | oracle/test | P4 | 518 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_kanban_notifier_apiserver_wake` | oracle/test | P4 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_kanban_notifier_watcher_dispatch_gate` | oracle/test | P4 | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_kanban_notifier_zero_sub_gate` | oracle/test | P4 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_kanban_reconcile_orphans` | oracle/test | P4 | 178 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_kanban_watchers_mixin` | oracle/test | P4 | 28 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_keep_typing_timeout` | oracle/test | P4 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_lifecycle_ledger` | oracle/test | P4 | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_line_plugin` | oracle/test | P4 | 509 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_load_transcript_db_only` | oracle/test | P4 | 30 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_loop_exception_handler` | oracle/test | P4 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_loop_liveness_watchdog` | oracle/test | P4 | 200 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix` | oracle/test | P4 | 3,344 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_approval_reaction_fail_closed` | oracle/test | P4 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_dm_invite_recording` | oracle/test | P4 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_exec_approval` | oracle/test | P4 | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_mention` | oracle/test | P4 | 386 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_message_event_metadata` | oracle/test | P4 | 248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_message_length` | oracle/test | P4 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_plugin_setup` | oracle/test | P4 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_project_context_isolation` | oracle/test | P4 | 342 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_recovery_key_scope` | oracle/test | P4 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_matrix_voice` | oracle/test | P4 | 266 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_mattermost` | oracle/test | P4 | 596 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_mattermost_plugin_setup` | oracle/test | P4 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_max_concurrent_sessions` | oracle/test | P4 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_max_tokens_propagation` | oracle/test | P4 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_mcp_reload_refreshes_cached_agents` | oracle/test | P4 | 176 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_cache` | oracle/test | P4 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_download_retry` | oracle/test | P4 | 559 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_extraction` | oracle/test | P4 | 387 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_metadata_contract` | oracle/test | P4 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_spaced_paths_and_history_dedupe` | oracle/test | P4 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_tag_cleanup` | oracle/test | P4 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_tag_formatting_variants` | oracle/test | P4 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_media_tag_separator` | oracle/test | P4 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_memory_monitor` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_memory_trim_housekeeping` | oracle/test | P4 | 32 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_message_deduplicator` | oracle/test | P4 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_message_timestamps` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_mirror` | oracle/test | P4 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_mixed_attachment_routing` | oracle/test | P4 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_moa_one_shot_restore` | oracle/test | P4 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_model_command_async_offload` | oracle/test | P4 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_model_command_context_offload` | oracle/test | P4 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_model_command_custom_providers` | oracle/test | P4 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_model_command_expensive_confirm` | oracle/test | P4 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_model_command_flat_string_config` | oracle/test | P4 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_model_picker_persist` | oracle/test | P4 | 255 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_model_switch_persistence` | oracle/test | P4 | 259 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_msgraph_webhook` | oracle/test | P4 | 264 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_adapter_registry` | oracle/test | P4 | 497 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_api_server_routing` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_background_task_scope` | oracle/test | P4 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_credential_isolation` | oracle/test | P4 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_http_routing` | oracle/test | P4 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_lifecycle` | oracle/test | P4 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_pairing_stores` | oracle/test | P4 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_phase0` | oracle/test | P4 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_multiplex_profile_authz` | oracle/test | P4 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_native_image_buffer_isolation` | oracle/test | P4 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_new_clears_last_resolved_model` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_notice_delivery` | oracle/test | P4 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_notice_rendering` | oracle/test | P4 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_ntfy_plugin` | oracle/test | P4 | 493 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_own_policy_startup_gate` | oracle/test | P4 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_pairing` | oracle/test | P4 | 680 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_pairing_allowlist_bypass` | oracle/test | P4 | 394 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_pending_drain_no_recursion` | oracle/test | P4 | 292 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_pending_drain_race` | oracle/test | P4 | 212 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_pending_event_none` | oracle/test | P4 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_per_platform_streaming_defaults` | oracle/test | P4 | 21 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_pii_redaction` | oracle/test | P4 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_plaintext_approval_routing` | oracle/test | P4 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_planned_stop_watcher` | oracle/test | P4 | 203 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_platform_base` | oracle/test | P4 | 1,119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_platform_connected_checkers` | oracle/test | P4 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_platform_http_client_limits` | oracle/test | P4 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_platform_reconnect` | oracle/test | P4 | 855 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_platform_reconnect_fd_leak` | oracle/test | P4 | 286 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_platform_registry` | oracle/test | P4 | 699 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_plugin_platform_interface` | oracle/test | P4 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_post_delivery_callback_chaining` | oracle/test | P4 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_post_stream_media_delivery` | oracle/test | P4 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_pre_gateway_dispatch` | oracle/test | P4 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_priority_path_compression_demotion_56391` | oracle/test | P4 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_profile_resolution` | oracle/test | P4 | 260 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_profile_routing` | oracle/test | P4 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_prompt_tail_freeze` | oracle/test | P4 | 329 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_proxy_mode` | oracle/test | P4 | 297 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_qqbot` | oracle/test | P4 | 1,224 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_qqbot_credential_isolation` | oracle/test | P4 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_qqbot_scope_paths` | oracle/test | P4 | 268 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_queue_command` | oracle/test | P4 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_queue_consumption` | oracle/test | P4 | 222 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_queued_native_image_session_key` | oracle/test | P4 | 151 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_raft_adapter` | oracle/test | P4 | 220 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_readiness` | oracle/test | P4 | 61 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_reasoning_command` | oracle/test | P4 | 220 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_reasoning_config_per_model` | oracle/test | P4 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_relay_capability_surface` | oracle/test | P4 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_relay_upstream_authz` | oracle/test | P4 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_reload_skills_command` | oracle/test | P4 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_reload_skills_discord_resync` | oracle/test | P4 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_replace_child_reap` | oracle/test | P4 | 251 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_replay_entry_fields` | oracle/test | P4 | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_reply_to_injection` | oracle/test | P4 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_response_filters` | oracle/test | P4 | 21 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_restart_after_turn` | oracle/test | P4 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_restart_drain` | oracle/test | P4 | 370 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_restart_notification` | oracle/test | P4 | 415 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_restart_redelivery_dedup` | oracle/test | P4 | 166 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_restart_resume_pending` | oracle/test | P4 | 1,042 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_restart_service_detection` | oracle/test | P4 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_resume_command` | oracle/test | P4 | 770 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_retry_replacement` | oracle/test | P4 | 151 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_retry_response` | oracle/test | P4 | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_routing_save_fast_path` | oracle/test | P4 | 445 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_run_cleanup_progress` | oracle/test | P4 | 297 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_run_progress_interrupt` | oracle/test | P4 | 251 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_run_progress_topics` | oracle/test | P4 | 1,522 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_run_tool_media_re` | oracle/test | P4 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_runner_fatal_adapter` | oracle/test | P4 | 242 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_runner_startup_failures` | oracle/test | P4 | 415 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_running_agent_session_toggles` | oracle/test | P4 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_runtime_config_env_expansion` | oracle/test | P4 | 41 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_runtime_env_reload_config_authority` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_runtime_footer` | oracle/test | P4 | 319 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_safe_adapter_disconnect` | oracle/test | P4 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_scale_to_zero` | oracle/test | P4 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_scale_to_zero_watcher` | oracle/test | P4 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_send_error_classification` | oracle/test | P4 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_send_image_file` | oracle/test | P4 | 286 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_send_multiple_images` | oracle/test | P4 | 429 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_send_retry` | oracle/test | P4 | 208 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_send_voice_reply_notify` | oracle/test | P4 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session` | oracle/test | P4 | 1,532 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_api` | oracle/test | P4 | 663 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_boundary_hooks` | oracle/test | P4 | 228 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_boundary_security_state` | oracle/test | P4 | 220 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_context_inheritance` | oracle/test | P4 | 202 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_dm_thread_seeding` | oracle/test | P4 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_env` | oracle/test | P4 | 275 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_hygiene` | oracle/test | P4 | 1,167 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_id_cache_coherence` | oracle/test | P4 | 231 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_info` | oracle/test | P4 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_list_allowed_sources` | oracle/test | P4 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_load_bool` | oracle/test | P4 | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_messages_shutdown_preserve` | oracle/test | P4 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_model_override_credential_pool` | oracle/test | P4 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_model_override_persistence` | oracle/test | P4 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_model_override_routing` | oracle/test | P4 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_model_reset` | oracle/test | P4 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_override_thread_recovery` | oracle/test | P4 | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_race_guard` | oracle/test | P4 | 289 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_reset_notify` | oracle/test | P4 | 285 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_split_brain_11016` | oracle/test | P4 | 298 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_stall_watchdog` | oracle/test | P4 | 517 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_state_cleanup` | oracle/test | P4 | 162 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_store_expiry_finalized` | oracle/test | P4 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_store_lock_io` | oracle/test | P4 | 256 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_store_prune` | oracle/test | P4 | 261 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_store_runtime_stale_guard` | oracle/test | P4 | 190 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_session_store_stale_prune` | oracle/test | P4 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_setup_feishu` | oracle/test | P4 | 228 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_shared_group_sender_prefix` | oracle/test | P4 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_shutdown_cache_cleanup` | oracle/test | P4 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_shutdown_flush` | oracle/test | P4 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_shutdown_forensics` | oracle/test | P4 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_shutdown_memory_provider_messages` | oracle/test | P4 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_shutdown_watchdog` | oracle/test | P4 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_signal` | oracle/test | P4 | 1,335 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_signal_format` | oracle/test | P4 | 258 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_signal_rate_limit` | oracle/test | P4 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_simplex_plugin` | oracle/test | P4 | 303 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_skip_context_files_wiring` | oracle/test | P4 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack` | oracle/test | P4 | 4,561 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_approval_buttons` | oracle/test | P4 | 843 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_block_kit` | oracle/test | P4 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_block_kit_adapter` | oracle/test | P4 | 187 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_bot_auth_bypass` | oracle/test | P4 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_channel_session_scope` | oracle/test | P4 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_channel_skills` | oracle/test | P4 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_clarify_buttons` | oracle/test | P4 | 274 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_cron_continuable_surface` | oracle/test | P4 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_dedup_ttl` | oracle/test | P4 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_download_ssrf` | oracle/test | P4 | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_group_dm_scope_warning` | oracle/test | P4 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_ignore_other_user_mentions` | oracle/test | P4 | 263 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_log_noise` | oracle/test | P4 | 268 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_mention` | oracle/test | P4 | 618 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_mention_humanization` | oracle/test | P4 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_peer_agent_smoke` | oracle/test | P4 | 174 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_plugin_action_handlers` | oracle/test | P4 | 265 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_plugin_setup` | oracle/test | P4 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_relay_parent_command` | oracle/test | P4 | 41 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_require_mention_channels` | oracle/test | P4 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_runner_ignored_channels` | oracle/test | P4 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_send_retry` | oracle/test | P4 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_socket_reconnect_heal` | oracle/test | P4 | 297 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_status_update` | oracle/test | P4 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_user_token_warning` | oracle/test | P4 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slack_wake_external_bot_messages` | oracle/test | P4 | 210 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slash_access` | oracle/test | P4 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_slash_access_dispatch` | oracle/test | P4 | 368 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_sms` | oracle/test | P4 | 323 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_sse_agent_cancel` | oracle/test | P4 | 482 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_sse_frame` | oracle/test | P4 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_ssl_cert_detection` | oracle/test | P4 | 34 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_ssl_certs` | oracle/test | P4 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stacked_skill_platform_disabled` | oracle/test | P4 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stale_confirmation_expiry` | oracle/test | P4 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stale_finalize_suppression` | oracle/test | P4 | 609 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stale_platform_lock_retryable` | oracle/test | P4 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stale_self_heal_agent_cache_eviction` | oracle/test | P4 | 200 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_startup_no_eager_platform_install` | oracle/test | P4 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_startup_restart_race` | oracle/test | P4 | 218 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_status` | oracle/test | P4 | 1,130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_status_command` | oracle/test | P4 | 490 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_status_phrases` | oracle/test | P4 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_steer_command` | oracle/test | P4 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_steer_fifo_overwrite` | oracle/test | P4 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_step_callback_compat` | oracle/test | P4 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_sticker_cache` | oracle/test | P4 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stop_thread_sibling` | oracle/test | P4 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stream_consumer` | oracle/test | P4 | 1,490 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stream_consumer_draft` | oracle/test | P4 | 391 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stream_consumer_fresh_final` | oracle/test | P4 | 340 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stream_consumer_silence` | oracle/test | P4 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stream_consumer_thread_routing` | oracle/test | P4 | 174 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stream_events` | oracle/test | P4 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_streaming_tts_consumer` | oracle/test | P4 | 675 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_streaming_tts_gateway_regression` | oracle/test | P4 | 157 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stt_config` | oracle/test | P4 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stt_transcript_echo_config` | oracle/test | P4 | 24 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_stuck_loop` | oracle/test | P4 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_subagent_protection_30170` | oracle/test | P4 | 244 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_systemd_notify` | oracle/test | P4 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_systemd_watchdog_lifecycle` | oracle/test | P4 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_table_helpers` | oracle/test | P4 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_teams` | oracle/test | P4 | 751 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_teams_dotenv_isolation` | oracle/test | P4 | 223 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_teams_pipeline_runtime_wiring` | oracle/test | P4 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_approval_buttons` | oracle/test | P4 | 362 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_audio_vs_voice` | oracle/test | P4 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_auth_check` | oracle/test | P4 | 315 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_bot_auth_bypass` | oracle/test | P4 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_callback_auth_fail_closed` | oracle/test | P4 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_caption_merge` | oracle/test | P4 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_channel_posts` | oracle/test | P4 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_clarify_buttons` | oracle/test | P4 | 280 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_closewait_limits_31599` | oracle/test | P4 | 162 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_conflict` | oracle/test | P4 | 676 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_connect` | oracle/test | P4 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_documents` | oracle/test | P4 | 566 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_error_redaction` | oracle/test | P4 | 190 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_fallback_pool_release_71593` | oracle/test | P4 | 167 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_final_delivery` | oracle/test | P4 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_format` | oracle/test | P4 | 677 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_forum_commands` | oracle/test | P4 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_group_gating` | oracle/test | P4 | 884 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_init_deadline` | oracle/test | P4 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_long_command_batching` | oracle/test | P4 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_max_doc_bytes` | oracle/test | P4 | 45 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_media_read_timeout` | oracle/test | P4 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_mention_boundaries` | oracle/test | P4 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_model_picker` | oracle/test | P4 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_network` | oracle/test | P4 | 491 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_network_reconnect` | oracle/test | P4 | 688 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_noise_filter` | oracle/test | P4 | 345 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_overflow_partial` | oracle/test | P4 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_pending_update_probe` | oracle/test | P4 | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_photo_interrupts` | oracle/test | P4 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_polling_progress` | oracle/test | P4 | 403 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_progress_edit_transient` | oracle/test | P4 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_prune_stale_topic_binding_31501` | oracle/test | P4 | 332 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_reactions` | oracle/test | P4 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_reply_mode` | oracle/test | P4 | 229 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_reply_quote` | oracle/test | P4 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_rich_messages` | oracle/test | P4 | 557 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_rich_newlines` | oracle/test | P4 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_send_draft_format` | oracle/test | P4 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_send_path_health` | oracle/test | P4 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_slash_confirm` | oracle/test | P4 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_start_polling_timeout` | oracle/test | P4 | 176 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_status_indicator` | oracle/test | P4 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_status_update` | oracle/test | P4 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_text_batch_perf` | oracle/test | P4 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_text_batching` | oracle/test | P4 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_thread_fallback` | oracle/test | P4 | 726 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_topic_mode` | oracle/test | P4 | 764 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_typing_backoff` | oracle/test | P4 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_username_chat_id` | oracle/test | P4 | 160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_voice_caption_markdown` | oracle/test | P4 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_voice_duration` | oracle/test | P4 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_voice_v0_regressions` | oracle/test | P4 | 278 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_telegram_webhook_secret` | oracle/test | P4 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_text_batching` | oracle/test | P4 | 276 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_title_command` | oracle/test | P4 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_tool_log_mode` | oracle/test | P4 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_tool_response_drop_recovery` | oracle/test | P4 | 329 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_transcript_offset` | oracle/test | P4 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_tts_media_routing` | oracle/test | P4 | 198 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_tui_approval_redaction` | oracle/test | P4 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_turn_context` | oracle/test | P4 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_turn_lease` | oracle/test | P4 | 480 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_typing_indicator_toggle` | oracle/test | P4 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_unauthorized_dm_behavior` | oracle/test | P4 | 398 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_unavailable_skill_hint` | oracle/test | P4 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_undo_rewind_session` | oracle/test | P4 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_unknown_command` | oracle/test | P4 | 227 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_update_command` | oracle/test | P4 | 434 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_update_cron_drain` | oracle/test | P4 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_update_streaming` | oracle/test | P4 | 400 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_usage_command` | oracle/test | P4 | 239 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_verbose_command` | oracle/test | P4 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_version_command` | oracle/test | P4 | 12 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_video_context_note` | oracle/test | P4 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_vision_memory_leak` | oracle/test | P4 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_voice_command` | oracle/test | P4 | 2,043 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_voice_mode_platform_isolation` | oracle/test | P4 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_wake_delivery` | oracle/test | P4 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_watchdog_review_76354` | oracle/test | P4 | 259 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_weak_credential_guard` | oracle/test | P4 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_webhook_adapter` | oracle/test | P4 | 1,004 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_webhook_deliver_only` | oracle/test | P4 | 250 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_webhook_dynamic_routes` | oracle/test | P4 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_webhook_integration` | oracle/test | P4 | 340 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_webhook_session_close` | oracle/test | P4 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_webhook_signature_rate_limit` | oracle/test | P4 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_wecom` | oracle/test | P4 | 485 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_wecom_callback` | oracle/test | P4 | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_wecom_plugin_setup` | oracle/test | P4 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_weixin` | oracle/test | P4 | 830 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_weixin_secret_scope` | oracle/test | P4 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_weixin_typing` | oracle/test | P4 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_allowlist_lid_resolution` | oracle/test | P4 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_bridge_dir_resolution` | oracle/test | P4 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_bridge_pidfile` | oracle/test | P4 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_cloud` | oracle/test | P4 | 1,402 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_cloud_allowed_users` | oracle/test | P4 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_connect` | oracle/test | P4 | 480 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_formatting` | oracle/test | P4 | 231 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_from_owner` | oracle/test | P4 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_group_gating` | oracle/test | P4 | 232 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_identity` | oracle/test | P4 | 23 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_media_path_profile` | oracle/test | P4 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_native_delivery` | oracle/test | P4 | 45 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_plugin_setup` | oracle/test | P4 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_reply_prefix` | oracle/test | P4 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_stale_bridge` | oracle/test | P4 | 213 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_text_batching` | oracle/test | P4 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_whatsapp_to_jid` | oracle/test | P4 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_ws_auth_retry` | oracle/test | P4 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_yolo_command` | oracle/test | P4 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_yuanbao_forwarded_heartbeat` | oracle/test | P4 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.gateway.test_yuanbao_media_ssrf` | oracle/test | P4 | 25 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.__init__` | oracle/test | P3 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.conftest` | oracle/test | P3 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.conftest_dashboard_auth` | oracle/test | P3 | 184 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_25106_global_switch_persists_base_url_api_mode` | oracle/test | P3 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_active_sessions` | oracle/test | P3 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_actual_provider` | oracle/test | P3 | 252 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_agent_import` | oracle/test | P3 | 729 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_agent_plugins` | oracle/test | P3 | 411 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ai_gateway_models` | oracle/test | P3 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_anthropic_model_flow_stale_oauth` | oracle/test | P3 | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_anthropic_oauth_flow` | oracle/test | P3 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_anthropic_oauth_routes_to_messages_api` | oracle/test | P3 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_anthropic_picker_curated` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_anthropic_provider_persistence` | oracle/test | P3 | 34 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_api_key_providers` | oracle/test | P3 | 1,216 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_apply_model_switch_result_context` | oracle/test | P3 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_apply_profile_override` | oracle/test | P3 | 166 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_approvals_command` | oracle/test | P3 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_approvals_suggest` | oracle/test | P3 | 296 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_approvals_test` | oracle/test | P3 | 221 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_arcee_provider` | oracle/test | P3 | 160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_argparse_flag_propagation` | oracle/test | P3 | 273 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_at_context_completion_filter` | oracle/test | P3 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_atomic_json_write` | oracle/test | P3 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_atomic_yaml_write` | oracle/test | P3 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_codex_provider` | oracle/test | P3 | 613 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_codex_quota_probe` | oracle/test | P3 | 283 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_codex_self_heal` | oracle/test | P3 | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_commands` | oracle/test | P3 | 771 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_loopback_ssh_hint` | oracle/test | P3 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_nous_provider` | oracle/test | P3 | 1,176 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_profile_fallback` | oracle/test | P3 | 290 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_provider_gate` | oracle/test | P3 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_qwen_provider` | oracle/test | P3 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_ssl_macos` | oracle/test | P3 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_store_read_failure` | oracle/test | P3 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_toctou_file_modes` | oracle/test | P3 | 202 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_usable_secret` | oracle/test | P3 | 13 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_auth_xai_oauth_provider` | oracle/test | P3 | 992 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_authenticated_providers_exhausted_pool` | oracle/test | P3 | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_aux_config` | oracle/test | P3 | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_aux_picker_inventory` | oracle/test | P3 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_azure_detect` | oracle/test | P3 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_azure_foundry_entra` | oracle/test | P3 | 245 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_backup` | oracle/test | P3 | 1,248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_backup_stability` | oracle/test | P3 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_banner` | oracle/test | P3 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_banner_git_state` | oracle/test | P3 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_banner_skills` | oracle/test | P3 | 33 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_banner_skills_width` | oracle/test | P3 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_bedrock_model_picker` | oracle/test | P3 | 213 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_bedrock_region_scoped_picker` | oracle/test | P3 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_billing_cli` | oracle/test | P3 | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_billing_portal_url` | oracle/test | P3 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_billing_scope_stepup` | oracle/test | P3 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_bitwarden_status` | oracle/test | P3 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_browser_connect_dual_stack` | oracle/test | P3 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_build_info` | oracle/test | P3 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_bundles` | oracle/test | P3 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_busy_policy_invariants` | oracle/test | P3 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_bytecode_sweep` | oracle/test | P3 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cached_fetch_api_models` | oracle/test | P3 | 295 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_canonical_custom_identity` | oracle/test | P3 | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_certifi_repair` | oracle/test | P3 | 212 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_chat_skills_flag` | oracle/test | P3 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_checkout_mutation_guards` | oracle/test | P3 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_checkpoints_prune` | oracle/test | P3 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_claw` | oracle/test | P3 | 398 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_clear_stale_base_url` | oracle/test | P3 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cli_active_session_limit` | oracle/test | P3 | 41 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cli_custom_provider_vision` | oracle/test | P3 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cli_model_once` | oracle/test | P3 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cli_output` | oracle/test | P3 | 20 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_clipboard_text_write` | oracle/test | P3 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cmd_update` | oracle/test | P3 | 755 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cmd_update_docker` | oracle/test | P3 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_coalesce_session_args` | oracle/test | P3 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_codex_cli_model_picker` | oracle/test | P3 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_codex_models` | oracle/test | P3 | 181 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_codex_runtime_plugin_migration` | oracle/test | P3 | 423 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_codex_runtime_switch` | oracle/test | P3 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_commands` | oracle/test | P3 | 1,029 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_commands_execute` | oracle/test | P3 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_completion` | oracle/test | P3 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_config` | oracle/test | P3 | 1,442 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_config_env_expansion` | oracle/test | P3 | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_config_env_ref_parity` | oracle/test | P3 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_config_env_refs` | oracle/test | P3 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_config_loader_e2e` | oracle/test | P3 | 160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_config_read_guard` | oracle/test | P3 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_config_validation` | oracle/test | P3 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_configured_builtin_models` | oracle/test | P3 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_console_engine` | oracle/test | P3 | 341 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_container_aware_cli` | oracle/test | P3 | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_container_boot` | oracle/test | P3 | 303 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_context_switch_guard` | oracle/test | P3 | 173 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_copilot_auth` | oracle/test | P3 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_copilot_catalog_oauth_fallback` | oracle/test | P3 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_copilot_context` | oracle/test | P3 | 218 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_copilot_in_model_list` | oracle/test | P3 | 22 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_copilot_model_api_mode` | oracle/test | P3 | 23 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_copilot_runtime_api_mode` | oracle/test | P3 | 111 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_copilot_token_exchange` | oracle/test | P3 | 277 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_credential_lifecycle` | oracle/test | P3 | 152 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cron` | oracle/test | P3 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cron_dashboard_off_loop` | oracle/test | P3 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cron_fire_dashboard` | oracle/test | P3 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_cron_parser_builder` | oracle/test | P3 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ctrlg_editor_submit` | oracle/test | P3 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curator_archive_prune` | oracle/test | P3 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curator_recent_run_notice` | oracle/test | P3 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curator_run` | oracle/test | P3 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curator_status` | oracle/test | P3 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curator_usage` | oracle/test | P3 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curses_arrow_keys` | oracle/test | P3 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curses_color_compat` | oracle/test | P3 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curses_ui_fuzzy_rank` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_curses_ui_search` | oracle/test | P3 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_custom_provider_context_length` | oracle/test | P3 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_custom_provider_extra_headers` | oracle/test | P3 | 246 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_custom_provider_identity` | oracle/test | P3 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_custom_provider_model_switch` | oracle/test | P3 | 519 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_custom_provider_normalize_no_mutate` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_custom_provider_tls` | oracle/test | P3 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_admin_endpoints` | oracle/test | P3 | 969 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_401_reauth` | oracle/test | P3 | 652 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_audit` | oracle/test | P3 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_cookies` | oracle/test | P3 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_gate` | oracle/test | P3 | 205 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_middleware` | oracle/test | P3 | 360 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_native_flow` | oracle/test | P3 | 246 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_password_login` | oracle/test | P3 | 375 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_plugin_hook` | oracle/test | P3 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_prefix` | oracle/test | P3 | 527 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_provider_base` | oracle/test | P3 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_status_endpoint` | oracle/test | P3 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_stub_provider` | oracle/test | P3 | 45 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_ws_auth` | oracle/test | P3 | 429 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_auth_ws_tickets` | oracle/test | P3 | 178 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_basic_auth_plugin_enable` | oracle/test | P3 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_browser_safe_imports` | oracle/test | P3 | 16 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_lifecycle_flags` | oracle/test | P3 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_oauth_endpoints_server_gate` | oracle/test | P3 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_param_clamps` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_profiles_nav_label` | oracle/test | P3 | 12 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_register` | oracle/test | P3 | 424 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_token_auth` | oracle/test | P3 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_tui_backcompat` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_unified_launch` | oracle/test | P3 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dashboard_web_dist_validation` | oracle/test | P3 | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_debug` | oracle/test | P3 | 1,101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_default_interface_resolution` | oracle/test | P3 | 162 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dep_ensure` | oracle/test | P3 | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_deprecated_cwd_warning` | oracle/test | P3 | 29 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_desktop_exe_integrity` | oracle/test | P3 | 302 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_desktop_repo_discovery_config` | oracle/test | P3 | 16 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_destructive_slash_confirm_gate` | oracle/test | P3 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_detect_api_mode_for_url` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_determine_api_mode_hostname` | oracle/test | P3 | 27 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_diagnostics_upload` | oracle/test | P3 | 185 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_diff_command` | oracle/test | P3 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dingtalk_auth` | oracle/test | P3 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_discord_skill_clamp_warning` | oracle/test | P3 | 160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_doctor` | oracle/test | P3 | 1,412 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_doctor_command_install` | oracle/test | P3 | 151 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_doctor_dedicated_provider_skip` | oracle/test | P3 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_doctor_live` | oracle/test | P3 | 270 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dump_env_visibility` | oracle/test | P3 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dump_git_commit` | oracle/test | P3 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_dump_terminal_backend` | oracle/test | P3 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_early_recovery` | oracle/test | P3 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ensure_acp_launcher` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ensure_hermes_home_memo` | oracle/test | P3 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ensure_hermes_home_uid_34107` | oracle/test | P3 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ensure_utf8_locale` | oracle/test | P3 | 116 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_env_custom_keys` | oracle/test | P3 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_env_export_line_lifecycle` | oracle/test | P3 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_env_export_prefix` | oracle/test | P3 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_env_load_cache` | oracle/test | P3 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_env_loader` | oracle/test | P3 | 442 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_env_sanitize_on_load` | oracle/test | P3 | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_fallback_cmd` | oracle/test | P3 | 314 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_fallback_config` | oracle/test | P3 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_fireworks_provider` | oracle/test | P3 | 180 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway` | oracle/test | P3 | 433 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_external_supervisor` | oracle/test | P3 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_linger` | oracle/test | P3 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_platform_gating` | oracle/test | P3 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_proc_fallback` | oracle/test | P3 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_restart_loop` | oracle/test | P3 | 1,186 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_run_hard_exit` | oracle/test | P3 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_runtime_health` | oracle/test | P3 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_s6_dispatch` | oracle/test | P3 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_service` | oracle/test | P3 | 1,998 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_service_paths` | oracle/test | P3 | 22 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_windows` | oracle/test | P3 | 257 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gateway_wsl` | oracle/test | P3 | 146 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gemini_free_tier_setup_block` | oracle/test | P3 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gemini_provider` | oracle/test | P3 | 204 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_get_env_value_scope` | oracle/test | P3 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_git_probe_tree_kill` | oracle/test | P3 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gmi_provider` | oracle/test | P3 | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_goal_gates` | oracle/test | P3 | 224 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_goals` | oracle/test | P3 | 800 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gpt56_registration` | oracle/test | P3 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_graphical_browser_detection` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gui_command` | oracle/test | P3 | 461 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_gui_uninstall` | oracle/test | P3 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_heartbeat` | oracle/test | P3 | 187 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_hooks_cli` | oracle/test | P3 | 205 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ignore_user_config_flags` | oracle/test | P3 | 203 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_image_gen_picker` | oracle/test | P3 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_init_command` | oracle/test | P3 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_input_sanitize` | oracle/test | P3 | 36 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_install_cua_driver` | oracle/test | P3 | 1,019 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_inventory` | oracle/test | P3 | 515 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_inventory_pricing` | oracle/test | P3 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_jobs_json_utf8_bom` | oracle/test | P3 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_journey_render` | oracle/test | P3 | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_block_kinds` | oracle/test | P3 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_blocked_sticky` | oracle/test | P3 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_board_project` | oracle/test | P3 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_boards` | oracle/test | P3 | 346 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_cli` | oracle/test | P3 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_cli_dispatch_passthrough` | oracle/test | P3 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_comment_queries` | oracle/test | P3 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_core_functionality` | oracle/test | P3 | 1,410 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_count_notify_subs` | oracle/test | P3 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_db` | oracle/test | P3 | 1,585 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_db_init` | oracle/test | P3 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_db_repair` | oracle/test | P3 | 285 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_decompose` | oracle/test | P3 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_decompose_db` | oracle/test | P3 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_default_assignee` | oracle/test | P3 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_diagnostics` | oracle/test | P3 | 226 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_dispatch_lock` | oracle/test | P3 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_goal_mode` | oracle/test | P3 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_init_lock_bounded` | oracle/test | P3 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_lifecycle_hooks` | oracle/test | P3 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_notify` | oracle/test | P3 | 197 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_per_profile_cap` | oracle/test | P3 | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_project_link` | oracle/test | P3 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_promote` | oracle/test | P3 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_reclaim_claim_lock_guard` | oracle/test | P3 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_specify` | oracle/test | P3 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_specify_db` | oracle/test | P3 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_swarm` | oracle/test | P3 | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_worker_image_extraction` | oracle/test | P3 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_worker_session_source` | oracle/test | P3 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_worker_spawn_toolsets` | oracle/test | P3 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_worker_terminal_cwd` | oracle/test | P3 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_worktree_isolation` | oracle/test | P3 | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_write_guard` | oracle/test | P3 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kanban_write_txn_busy_retry` | oracle/test | P3 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_kimi_cn_provider_listing` | oracle/test | P3 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_launcher` | oracle/test | P3 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_lazy_refresh_venv_repair` | oracle/test | P3 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_lifecycle` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_linux_desktop_entry` | oracle/test | P3 | 197 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_list_picker_providers` | oracle/test | P3 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_lmstudio_context_policy` | oracle/test | P3 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_logs` | oracle/test | P3 | 151 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_main_model_custom_provider_normalization` | oracle/test | P3 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_installs` | oracle/test | P3 | 29 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope` | oracle/test | P3 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_cli_config` | oracle/test | P3 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_config` | oracle/test | P3 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_env` | oracle/test | P3 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_loaders` | oracle/test | P3 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_overlay` | oracle/test | P3 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_regression` | oracle/test | P3 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_surfacing` | oracle/test | P3 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_scope_writeguard` | oracle/test | P3 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_managed_uv` | oracle/test | P3 | 1,102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_add_command_dest` | oracle/test | P3 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_catalog` | oracle/test | P3 | 638 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_config` | oracle/test | P3 | 749 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_dashboard_oauth` | oracle/test | P3 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_discovery_timing` | oracle/test | P3 | 310 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_reload_confirm_gate` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_security` | oracle/test | P3 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_startup` | oracle/test | P3 | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mcp_tools_config` | oracle/test | P3 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_mem_trim` | oracle/test | P3 | 216 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_memory_reset` | oracle/test | P3 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_memory_setup` | oracle/test | P3 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_memory_setup_provider_arg` | oracle/test | P3 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_memory_status` | oracle/test | P3 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_migrate_xai` | oracle/test | P3 | 296 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_moa_config` | oracle/test | P3 | 183 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_moa_set_models_preserves_extra_keys` | oracle/test | P3 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_cache_swr` | oracle/test | P3 | 204 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_catalog` | oracle/test | P3 | 488 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_cost_guard` | oracle/test | P3 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_flow_pooled_credentials` | oracle/test | P3 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_normalize` | oracle/test | P3 | 189 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_picker_excluded_providers` | oracle/test | P3 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_picker_expensive_confirm` | oracle/test | P3 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_picker_viewport` | oracle/test | P3 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_provider_persistence` | oracle/test | P3 | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_search` | oracle/test | P3 | 18 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_search_alias_dedup` | oracle/test | P3 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_configured_provider_routing` | oracle/test | P3 | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_context_display` | oracle/test | P3 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_context_offload` | oracle/test | P3 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_copilot_api_mode` | oracle/test | P3 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_custom_providers` | oracle/test | P3 | 974 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_filter_unresolved` | oracle/test | P3 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_once_flags` | oracle/test | P3 | 14 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_openai_api_mode` | oracle/test | P3 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_opencode_anthropic` | oracle/test | P3 | 228 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_parsing` | oracle/test | P3 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_persist_default` | oracle/test | P3 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_switch_variant_tags` | oracle/test | P3 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_model_validation` | oracle/test | P3 | 522 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_models` | oracle/test | P3 | 485 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_models_dev_preferred_merge` | oracle/test | P3 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_non_ascii_credential` | oracle/test | P3 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_noninteractive_git` | oracle/test | P3 | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_normalize_main_model_assignment` | oracle/test | P3 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_account` | oracle/test | P3 | 342 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_auth_keepalive` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_auth_status_cache` | oracle/test | P3 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_billing_request` | oracle/test | P3 | 214 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_hermes_non_agentic` | oracle/test | P3 | 45 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_inference_url_validation` | oracle/test | P3 | 309 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_portal_staging_allowlist` | oracle/test | P3 | 154 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_session_validity` | oracle/test | P3 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_nous_subscription` | oracle/test | P3 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_npm_engine` | oracle/test | P3 | 337 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_official_openai_host` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ollama_cloud_auth` | oracle/test | P3 | 389 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ollama_cloud_provider` | oracle/test | P3 | 349 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_oneshot_usage_file` | oracle/test | P3 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_openai_codex_model_validation_fallback` | oracle/test | P3 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_openai_discovery_endpoint` | oracle/test | P3 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_openai_listing_authority` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_openai_picker_curated` | oracle/test | P3 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_opencode_go_flat_namespace` | oracle/test | P3 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_opencode_go_in_model_list` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_opencode_go_validation_fallback` | oracle/test | P3 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_opencode_zen_model_limit` | oracle/test | P3 | 51 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_overlay_slug_resolution` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_pairing` | oracle/test | P3 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_path_completion` | oracle/test | P3 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_pet_toggle` | oracle/test | P3 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_picker_prewarm` | oracle/test | P3 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_pin_kanban_board_env` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_pip_install_detection` | oracle/test | P3 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_placeholder_usage` | oracle/test | P3 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugin_auxiliary_tasks` | oracle/test | P3 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugin_cli_registration` | oracle/test | P3 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugin_runtime_disable_gate` | oracle/test | P3 | 246 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugin_scanner_recursion` | oracle/test | P3 | 302 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins` | oracle/test | P3 | 1,340 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins_cmd` | oracle/test | P3 | 682 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins_cmd_category_discovery` | oracle/test | P3 | 248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins_cmd_enable_disable_nested` | oracle/test | P3 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins_cmd_list` | oracle/test | P3 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins_hub_perf_guard` | oracle/test | P3 | 182 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins_transcription_registration` | oracle/test | P3 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_plugins_tts_registration` | oracle/test | P3 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_post_setup_gating` | oracle/test | P3 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_profile_describer` | oracle/test | P3 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_profile_distribution` | oracle/test | P3 | 761 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_profile_export_credentials` | oracle/test | P3 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_profile_install_env_encoding` | oracle/test | P3 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_profiles` | oracle/test | P3 | 923 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_profiles_s6_hooks` | oracle/test | P3 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_project_plugin_rce_bypass` | oracle/test | P3 | 310 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_projects_cli` | oracle/test | P3 | 61 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_projects_db` | oracle/test | P3 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_prompt_api_key` | oracle/test | P3 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_prompt_compose_command` | oracle/test | P3 | 61 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_prompt_size` | oracle/test | P3 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_provider_catalog` | oracle/test | P3 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_provider_config_validation` | oracle/test | P3 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_provider_groups` | oracle/test | P3 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_provider_live_curated_merge` | oracle/test | P3 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_provider_parity` | oracle/test | P3 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_provider_precedence` | oracle/test | P3 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_provider_section3_grouping` | oracle/test | P3 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_proxy` | oracle/test | P3 | 376 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_psutil_android_extract` | oracle/test | P3 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_pty_bridge` | oracle/test | P3 | 229 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_quarantine_forensic_logging` | oracle/test | P3 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_read_raw_config_readonly` | oracle/test | P3 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_reasoning_effort_menu` | oracle/test | P3 | 25 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_reasoning_full_command` | oracle/test | P3 | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_redact_config_bridge` | oracle/test | P3 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_regression_16767` | oracle/test | P3 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_relaunch` | oracle/test | P3 | 224 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_relay_shared_metrics` | oracle/test | P3 | 1,527 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_relay_shared_metrics_runtime` | oracle/test | P3 | 2,514 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_remote_spending_gate_contract` | oracle/test | P3 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_resolve_last_session` | oracle/test | P3 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_resolve_provider_openrouter_pool` | oracle/test | P3 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_resolve_token_memo` | oracle/test | P3 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_run_with_idle_timeout` | oracle/test | P3 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_runtime_provider_resolution` | oracle/test | P3 | 1,555 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_runtime_transport_precedence` | oracle/test | P3 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_safe_mode` | oracle/test | P3 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_sale_pricing` | oracle/test | P3 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_scan_venv_blockers` | oracle/test | P3 | 214 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_secret_prompt` | oracle/test | P3 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_secrets_bitwarden_non_tty` | oracle/test | P3 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_secrets_token_rotation` | oracle/test | P3 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_security_advisories` | oracle/test | P3 | 261 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_security_audit` | oracle/test | P3 | 216 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_security_audit_startup` | oracle/test | P3 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_send_cmd` | oracle/test | P3 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_serve_command` | oracle/test | P3 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_service_manager` | oracle/test | P3 | 491 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_browse` | oracle/test | P3 | 216 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_export` | oracle/test | P3 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_export_html` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_export_html_escape` | oracle/test | P3 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_export_md` | oracle/test | P3 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_filters` | oracle/test | P3 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_handoff` | oracle/test | P3 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_listing` | oracle/test | P3 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_recap` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_session_recovery` | oracle/test | P3 | 651 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_sessions_delete` | oracle/test | P3 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_sessions_export_md_cli` | oracle/test | P3 | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_sessions_size_delta_label` | oracle/test | P3 | 24 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_set_config_value` | oracle/test | P3 | 725 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup` | oracle/test | P3 | 252 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_agent_settings` | oracle/test | P3 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_blank_slate` | oracle/test | P3 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_hermes_script` | oracle/test | P3 | 20 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_hidden_env` | oracle/test | P3 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_irc` | oracle/test | P3 | 228 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_matrix_e2ee` | oracle/test | P3 | 30 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_menu_curses_migration` | oracle/test | P3 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_model_provider` | oracle/test | P3 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_noninteractive` | oracle/test | P3 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_openclaw_migration` | oracle/test | P3 | 339 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_prompt_menus` | oracle/test | P3 | 24 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_reconfigure` | oracle/test | P3 | 202 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_summary_provider_warning` | oracle/test | P3 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_telemetry` | oracle/test | P3 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_setup_tts_xai_oauth` | oracle/test | P3 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_signal_handler_kanban_worker` | oracle/test | P3 | 218 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skills_config` | oracle/test | P3 | 174 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skills_hub` | oracle/test | P3 | 315 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skills_install_flags` | oracle/test | P3 | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skills_skip_confirm` | oracle/test | P3 | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skills_subparser` | oracle/test | P3 | 34 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skin_cmd` | oracle/test | P3 | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skin_engine` | oracle/test | P3 | 272 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_skin_palettes` | oracle/test | P3 | 223 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_slack_cli` | oracle/test | P3 | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_spotify_auth` | oracle/test | P3 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_sqlite_runtime` | oracle/test | P3 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ssh_ownership_endpoint` | oracle/test | P3 | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_ssh_session_token_parser` | oracle/test | P3 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_startup_fast_guards` | oracle/test | P3 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_startup_plugin_gating` | oracle/test | P3 | 186 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_state_db_guard` | oracle/test | P3 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_status` | oracle/test | P3 | 239 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_status_model_provider` | oracle/test | P3 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_status_provider_label` | oracle/test | P3 | 31 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_stt_picker` | oracle/test | P3 | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_subcommands_batch` | oracle/test | P3 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_subcommands_followup` | oracle/test | P3 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_subcommands_profile_gateway` | oracle/test | P3 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_subparser_routing_fallback` | oracle/test | P3 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_subprocess_timeouts` | oracle/test | P3 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_subscription_cli` | oracle/test | P3 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_suppress_eio_on_interrupt` | oracle/test | P3 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_system_stats_platform` | oracle/test | P3 | 31 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_systemd_optional_directives` | oracle/test | P3 | 158 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_systemd_watchdog_unit` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_teams_pipeline_plugin_cli` | oracle/test | P3 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_telegram_managed_bot` | oracle/test | P3 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tencent_tokenhub_provider` | oracle/test | P3 | 357 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_terminal_menu_fallbacks` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_timeouts` | oracle/test | P3 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_timestamps_command` | oracle/test | P3 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tips` | oracle/test | P3 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tool_token_estimation` | oracle/test | P3 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tools_config` | oracle/test | P3 | 744 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tools_disable_enable` | oracle/test | P3 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_toolset_validation` | oracle/test | P3 | 50 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tts_picker` | oracle/test | P3 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tui_bundled` | oracle/test | P3 | 15 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tui_heap_sizing` | oracle/test | P3 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tui_mouse_residue_suppression` | oracle/test | P3 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tui_npm_install` | oracle/test | P3 | 178 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_tui_resume_flow` | oracle/test | P3 | 288 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_uninstall_dry_run` | oracle/test | P3 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_uninstall_node_symlinks` | oracle/test | P3 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_uninstall_shell_configs` | oracle/test | P3 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_autostash` | oracle/test | P3 | 339 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_check` | oracle/test | P3 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_concurrent_quarantine` | oracle/test | P3 | 528 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_config_clears_custom_fields` | oracle/test | P3 | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_eol_churn` | oracle/test | P3 | 212 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_fleet_restart_timeout` | oracle/test | P3 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_gateway_launcher_refresh` | oracle/test | P3 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_hangup_protection` | oracle/test | P3 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_import_guard` | oracle/test | P3 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_interrupted_recovery` | oracle/test | P3 | 107 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_lock` | oracle/test | P3 | 264 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_modified_notice` | oracle/test | P3 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_post_pull_syntax_guard` | oracle/test | P3 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_stale_dashboard` | oracle/test | P3 | 427 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_venv_health` | oracle/test | P3 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_yes_flag` | oracle/test | P3 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_zip_atomic_replace` | oracle/test | P3 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_zip_symlink_reject` | oracle/test | P3 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_update_zip_two_phase` | oracle/test | P3 | 470 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_upstage_provider` | oracle/test | P3 | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_urllib_security` | oracle/test | P3 | 397 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_user_providers_model_switch` | oracle/test | P3 | 616 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_verify_console_scripts` | oracle/test | P3 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_verify_core_dependencies` | oracle/test | P3 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_vertex_model_picker` | oracle/test | P3 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_vertex_provider` | oracle/test | P3 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_video_gen_picker` | oracle/test | P3 | 183 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_voice_wrapper` | oracle/test | P3 | 483 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_oauth_dispatch` | oracle/test | P3 | 699 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_profile_soul_writes` | oracle/test | P3 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server` | oracle/test | P3 | 4,337 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_boot_handshake` | oracle/test | P3 | 192 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_console_ws` | oracle/test | P3 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_cron_profiles` | oracle/test | P3 | 329 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_files` | oracle/test | P3 | 304 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_fs` | oracle/test | P3 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_gateway_topology` | oracle/test | P3 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_git` | oracle/test | P3 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_host_header` | oracle/test | P3 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_messaging_profiles` | oracle/test | P3 | 269 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_oauth_write` | oracle/test | P3 | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_profile_unification` | oracle/test | P3 | 451 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_pty_import` | oracle/test | P3 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_pty_reconnect` | oracle/test | P3 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_session_search` | oracle/test | P3 | 143 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_skill_editor` | oracle/test | P3 | 208 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_skills_profiles` | oracle/test | P3 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_server_speak_stream` | oracle/test | P3 | 123 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_web_ui_build` | oracle/test | P3 | 370 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_webhook_cli` | oracle/test | P3 | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_whatsapp_cloud_setup` | oracle/test | P3 | 212 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_whatsapp_onboarding` | oracle/test | P3 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_whatsapp_setup_ordering` | oracle/test | P3 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_win_pty_bridge` | oracle/test | P3 | 250 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_windows_native_docs` | oracle/test | P3 | 10 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xai_curated_models` | oracle/test | P3 | 14 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xai_model_flow` | oracle/test | P3 | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xai_oauth_profile_auth` | oracle/test | P3 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xai_oauth_refresh` | oracle/test | P3 | 44 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xai_oauth_writethrough` | oracle/test | P3 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xai_provider_labels` | oracle/test | P3 | 13 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xai_retirement` | oracle/test | P3 | 143 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_xiaomi_provider` | oracle/test | P3 | 320 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_cli.test_yolo_startup_order` | oracle/test | P3 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_append_messages_batch` | oracle/test | P1 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_aux_usage_accounting` | oracle/test | P1 | 285 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_conversation_root` | oracle/test | P1 | 33 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_get_anchored_view` | oracle/test | P1 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_get_messages_around` | oracle/test | P1 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_live_db_isolation_guard` | oracle/test | P1 | 192 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_replace_messages_archive_siblings` | oracle/test | P1 | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_resolve_resume_session_id` | oracle/test | P1 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_restore_alternation_repair` | oracle/test | P1 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_session_archiving` | oracle/test | P1 | 51 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_session_md_export` | oracle/test | P1 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.hermes_state.test_session_read_state` | oracle/test | P1 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.conftest` | oracle/test | oracle | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_async_memory` | oracle/test | oracle | 458 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_cli` | oracle/test | oracle | 746 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_client` | oracle/test | oracle | 587 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_empty_profile_hint` | oracle/test | oracle | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_network_isolation` | oracle/test | oracle | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_oauth` | oracle/test | oracle | 195 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_oauth_flow` | oracle/test | oracle | 487 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_pin_peer_name` | oracle/test | oracle | 574 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_query_rewrite` | oracle/test | oracle | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.honcho_plugin.test_session` | oracle/test | oracle | 1,317 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_batch_runner` | oracle/test | oracle | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_checkpoint_resumption` | oracle/test | oracle | 439 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_daytona_terminal` | oracle/test | oracle | 123 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_ha_integration` | oracle/test | oracle | 341 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_modal_terminal` | oracle/test | oracle | 294 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_vision_docker_resolve` | oracle/test | oracle | 157 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_voice_channel_flow` | oracle/test | oracle | 761 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.integration.test_web_tools` | oracle/test | oracle | 505 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.manual.cron_inchannel_dm_e2e` | oracle/test | oracle | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.manual.cron_inchannel_e2e` | oracle/test | oracle | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.monitoring.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.monitoring.test_cron_health_export` | oracle/test | oracle | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.monitoring.test_emitter` | oracle/test | oracle | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.monitoring.test_export_redaction` | oracle/test | oracle | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.monitoring.test_gateway_health_export` | oracle/test | oracle | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.monitoring.test_otlp_exporter` | oracle/test | oracle | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.openviking_plugin.test_openviking` | oracle/test | oracle | 1,289 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.browser.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.browser.check_parity_vs_main` | oracle/test | P4 | 273 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.browser.test_browser_provider_plugins` | oracle/test | P4 | 249 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.dashboard_auth.test_basic_provider` | oracle/test | P4 | 221 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.dashboard_auth.test_drain_provider` | oracle/test | P4 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.dashboard_auth.test_nous_provider` | oracle/test | P4 | 665 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.dashboard_auth.test_self_hosted_provider` | oracle/test | P4 | 729 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.check_parity_vs_main` | oracle/test | P4 | 300 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.test_deepinfra_provider` | oracle/test | P4 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.test_fal_provider` | oracle/test | P4 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.test_krea_provider` | oracle/test | P4 | 598 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.test_openai_codex_provider` | oracle/test | P4 | 355 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.test_openai_provider` | oracle/test | P4 | 242 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.test_openrouter_compat_provider` | oracle/test | P4 | 311 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.image_gen.test_xai_provider` | oracle/test | P4 | 406 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_byterover_provider` | oracle/test | P4 | 18 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_config_schema` | oracle/test | P4 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_hindsight_config_schema` | oracle/test | P4 | 51 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_hindsight_env_perms` | oracle/test | P4 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_hindsight_provider` | oracle/test | P4 | 1,375 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_holographic_auto_extract` | oracle/test | P4 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_holographic_retrieval` | oracle/test | P4 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_holographic_shutdown_closes_db` | oracle/test | P4 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_holographic_store` | oracle/test | P4 | 227 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_honcho_config_schema` | oracle/test | P4 | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_mem0_backend` | oracle/test | P4 | 215 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_mem0_providers` | oracle/test | P4 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_mem0_setup` | oracle/test | P4 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_mem0_v3` | oracle/test | P4 | 411 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_memory_lazy_install` | oracle/test | P4 | 250 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_openviking_endpoint_always_blocked` | oracle/test | P4 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_openviking_provider` | oracle/test | P4 | 1,611 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_openviking_shutdown` | oracle/test | P4 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_retaindb_provider` | oracle/test | P4 | 188 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.memory.test_supermemory_provider` | oracle/test | P4 | 458 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_copilot_profile` | oracle/test | P4 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_custom_profile` | oracle/test | P4 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_deepseek_profile` | oracle/test | P4 | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_fireworks_profile` | oracle/test | P4 | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_gemini_profile` | oracle/test | P4 | 27 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_kimi_profile` | oracle/test | P4 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_minimax_profile` | oracle/test | P4 | 183 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_ollama_cloud_profile` | oracle/test | P4 | 232 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_opencode_go_profile` | oracle/test | P4 | 163 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_upstage_profile` | oracle/test | P4 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.model_providers.test_zai_profile` | oracle/test | P4 | 178 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_auth` | oracle/test | P4 | 417 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_check_requirements_risks` | oracle/test | P4 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_fatal_notify_self_cancel` | oracle/test | P4 | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_inbound` | oracle/test | P4 | 223 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_markdown` | oracle/test | P4 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_mention_gating` | oracle/test | P4 | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_npm_error_log_regression` | oracle/test | P4 | 238 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_outbound_media` | oracle/test | P4 | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_overflow_recovery` | oracle/test | P4 | 495 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_poll_clarify` | oracle/test | P4 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_presence_watchdog` | oracle/test | P4 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_reactions` | oracle/test | P4 | 195 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_rich_links` | oracle/test | P4 | 239 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_runtime_record` | oracle/test | P4 | 232 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_setup_access` | oracle/test | P4 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_sidecar_deps_stale` | oracle/test | P4 | 51 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_sidecar_lifecycle` | oracle/test | P4 | 211 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_sidecar_paths` | oracle/test | P4 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_spectrum_patch` | oracle/test | P4 | 277 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_streaming` | oracle/test | P4 | 12 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_url_send_path` | oracle/test | P4 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.photon.test_zombie_stream_watchdog` | oracle/test | P4 | 231 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.platforms.test_discord_gate_isolation` | oracle/test | P4 | 436 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_a2a_phase23` | oracle/test | P4 | 687 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_a2a_plugin` | oracle/test | P4 | 1,620 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_achievements_plugin` | oracle/test | P4 | 303 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_chronos_cron` | oracle/test | P4 | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_chronos_verify` | oracle/test | P4 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_discord_runtime_failure` | oracle/test | P4 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_disk_cleanup_plugin` | oracle/test | P4 | 421 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_google_meet_audio` | oracle/test | P4 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_google_meet_node` | oracle/test | P4 | 187 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_google_meet_plugin` | oracle/test | P4 | 300 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_google_meet_realtime` | oracle/test | P4 | 227 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_hindsight_health_grace_timeout` | oracle/test | P4 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_hindsight_root_guard` | oracle/test | P4 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_holographic_vector_storage` | oracle/test | P4 | 205 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_kanban_attachments` | oracle/test | P4 | 295 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_kanban_board_project_api` | oracle/test | P4 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_kanban_dashboard_plugin` | oracle/test | P4 | 710 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_kanban_estimate` | oracle/test | P4 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_kanban_model_override` | oracle/test | P4 | 322 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_kanban_worker_runs` | oracle/test | P4 | 154 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_langfuse_plugin` | oracle/test | P4 | 781 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_nemo_relay_plugin` | oracle/test | P4 | 484 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_plugin_dashboard_auth_contract` | oracle/test | P4 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_raft_check_fn_silent` | oracle/test | P4 | 33 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_retaindb_plugin` | oracle/test | P4 | 439 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_security_guidance_plugin` | oracle/test | P4 | 284 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.test_teams_pipeline_plugin` | oracle/test | P4 | 424 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.transcription.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.transcription.check_parity_vs_main` | oracle/test | P4 | 431 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.tts.__init__` | oracle/test | P4 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.tts.check_parity_vs_main` | oracle/test | P4 | 328 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.video_gen.__init__` | oracle/test | P4 | 1 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.video_gen.test_deepinfra_provider` | oracle/test | P4 | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.video_gen.test_fal_plugin` | oracle/test | P4 | 256 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.video_gen.test_xai_plugin` | oracle/test | P4 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.plugins.video_gen.test_xai_plugin_integration` | oracle/test | P4 | 182 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.__init__` | oracle/test | P2 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.test_e2e_wiring` | oracle/test | P2 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.test_fetch_models_base_url` | oracle/test | P2 | 173 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.test_plugin_discovery` | oracle/test | P2 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.test_profile_wiring` | oracle/test | P2 | 178 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.test_provider_profiles` | oracle/test | P2 | 223 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.test_provider_registry` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.providers.test_transport_parity` | oracle/test | P2 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.__init__` | oracle/test | P2 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.conftest` | oracle/test | P2 | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.repro_48013_image_shrink_brick` | oracle/test | P2 | 179 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_1630_context_overflow_loop` | oracle/test | P2 | 225 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_18028_content_policy_blocked` | oracle/test | P2 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_24996_fallback_exhaustion_cooldown` | oracle/test | P2 | 232 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_28161_anthropic_stream_pool_cleanup` | oracle/test | P2 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_31273_402_not_retried` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_32646_fallback_429_after_timeout` | oracle/test | P2 | 324 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_413_compression` | oracle/test | P2 | 1,193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_63425_credential_pool_auto_detect` | oracle/test | P2 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_66267_multimodal_interim` | oracle/test | P2 | 189 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_70773_shared_client_fd_corruption` | oracle/test | P2 | 239 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_860_dedup` | oracle/test | P2 | 171 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_agent_guardrails` | oracle/test | P2 | 302 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_anthropic_mid_tool_call_drop` | oracle/test | P2 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_anthropic_prompt_cache_policy` | oracle/test | P2 | 395 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_anthropic_response_header_capture` | oracle/test | P2 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_anthropic_third_party_oauth_guard` | oracle/test | P2 | 158 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_anthropic_truncation_continuation` | oracle/test | P2 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_api_max_retries_config` | oracle/test | P2 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_async_httpx_del_neuter` | oracle/test | P2 | 286 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_auth_provider_failover` | oracle/test | P2 | 123 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_authorization_gate` | oracle/test | P2 | 317 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_background_review` | oracle/test | P2 | 225 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_background_review_cache_parity` | oracle/test | P2 | 342 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_background_review_cost_controls` | oracle/test | P2 | 160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_background_review_summary` | oracle/test | P2 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_background_review_toolset_restriction` | oracle/test | P2 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_callable_api_key` | oracle/test | P2 | 316 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_codex_app_server_compaction` | oracle/test | P2 | 182 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_codex_app_server_integration` | oracle/test | P2 | 788 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_codex_app_server_lifecycle` | oracle/test | P2 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_codex_multimodal_tool_result` | oracle/test | P2 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_codex_no_tools_nonetype` | oracle/test | P2 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_codex_silent_hang_hint` | oracle/test | P2 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_codex_xai_oauth_recovery` | oracle/test | P2 | 700 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_commit_memory_session_context_engine` | oracle/test | P2 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compress_focus_plugin_fallback` | oracle/test | P2 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compression_abort_state_reset` | oracle/test | P2 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compression_boundary` | oracle/test | P2 | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compression_boundary_hook` | oracle/test | P2 | 325 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compression_feasibility` | oracle/test | P2 | 410 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compression_lock_defer` | oracle/test | P2 | 309 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compression_persistence` | oracle/test | P2 | 526 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compression_trigger_excludes_reasoning` | oracle/test | P2 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_compressor_fallback_update` | oracle/test | P2 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_concurrent_interrupt` | oracle/test | P2 | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_context_token_tracking` | oracle/test | P2 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_conversation_fallback_state` | oracle/test | P2 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_copilot_native_vision_headers` | oracle/test | P2 | 53 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_create_openai_client_disables_sdk_retries` | oracle/test | P2 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_create_openai_client_kwargs_isolation` | oracle/test | P2 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_create_openai_client_proxy_env` | oracle/test | P2 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_create_openai_client_reuse` | oracle/test | P2 | 269 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_create_openai_client_ssl_verify` | oracle/test | P2 | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_credential_pool_interrupt` | oracle/test | P2 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_credential_rotation_route_settings` | oracle/test | P2 | 111 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_credits_notices_toggle` | oracle/test | P2 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_custom_provider_extra_headers_client` | oracle/test | P2 | 51 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_deepseek_reasoning_content_echo` | oracle/test | P2 | 406 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_deepseek_v4_thinking_live` | oracle/test | P2 | 245 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_dict_tool_call_args` | oracle/test | P2 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_dropped_tool_call_recovery` | oracle/test | P2 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_empty_response_recovery_persistence` | oracle/test | P2 | 166 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_empty_terminal_reasoning_surface` | oracle/test | P2 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_env_credential_turn_refresh` | oracle/test | P2 | 262 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_exit_cleanup_interrupt` | oracle/test | P2 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_fallback_credential_isolation` | oracle/test | P2 | 264 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_fallback_reasoning_override` | oracle/test | P2 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_file_mutation_verifier` | oracle/test | P2 | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_fireworks_live` | oracle/test | P2 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_identity_flush` | oracle/test | P2 | 237 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_image_generate_parallel` | oracle/test | P2 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_image_rejection_fallback` | oracle/test | P2 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_image_shrink_recovery` | oracle/test | P2 | 530 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_in_place_compaction` | oracle/test | P2 | 304 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_infinite_compaction_loop` | oracle/test | P2 | 254 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_init_fallback_on_exhausted_pool` | oracle/test | P2 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_interactive_interrupt` | oracle/test | P2 | 200 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_interrupt_propagation` | oracle/test | P2 | 277 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_invalid_context_length_warning` | oracle/test | P2 | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_iteration_budget_race` | oracle/test | P2 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_jsondecodeerror_retryable` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_last_reasoning_per_turn` | oracle/test | P2 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_lmstudio_load_mode` | oracle/test | P2 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_long_context_tier_429` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_malformed_tool_arguments` | oracle/test | P2 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_materialize_data_url_cleanup` | oracle/test | P2 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_memory_nudge_counter_hydration` | oracle/test | P2 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_memory_provider_init` | oracle/test | P2 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_memory_sync_interrupted` | oracle/test | P2 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_message_sequence_repair` | oracle/test | P2 | 382 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_moa_fanout_cadence` | oracle/test | P2 | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_moa_loop_mode` | oracle/test | P2 | 1,251 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_moa_privacy_filter` | oracle/test | P2 | 186 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_moa_streaming` | oracle/test | P2 | 232 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_multimodal_tool_content_recovery` | oracle/test | P2 | 198 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_nonretryable_error_html_summary` | oracle/test | P2 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_notice_spine` | oracle/test | P2 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_nous_429_fallback_reentry` | oracle/test | P2 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_nous_fallback_unavailable` | oracle/test | P2 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_openai_client_lifecycle` | oracle/test | P2 | 217 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_overflow_overhead_aware_tokens` | oracle/test | P2 | 404 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_partial_stream_finish_reason` | oracle/test | P2 | 788 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_per_model_compression_threshold` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_per_model_threshold_init_ordering` | oracle/test | P2 | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_percentage_clamp` | oracle/test | P2 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_plugin_context_engine_init` | oracle/test | P2 | 212 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_post_tool_compression_attempt_cap` | oracle/test | P2 | 199 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_pre_compress_memory_context` | oracle/test | P2 | 230 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_preflight_compression_cap_e2e` | oracle/test | P2 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_primary_runtime_restore` | oracle/test | P2 | 591 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_proactive_prune_loop_wiring` | oracle/test | P2 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_provider_attribution_headers` | oracle/test | P2 | 238 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_provider_fallback` | oracle/test | P2 | 347 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_provider_parity` | oracle/test | P2 | 924 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_repair_tool_call_arguments` | oracle/test | P2 | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_repair_tool_call_name` | oracle/test | P2 | 127 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_request_client_reuse_abort_races` | oracle/test | P2 | 408 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_reset_aware_primary_restore` | oracle/test | P2 | 340 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_retry_status_buffer` | oracle/test | P2 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_review_prompt_class_first` | oracle/test | P2 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_run_agent` | oracle/test | P2 | 6,196 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_run_agent_codex_responses` | oracle/test | P2 | 2,043 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_run_agent_multimodal_prologue` | oracle/test | P2 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_sequential_chats_live` | oracle/test | P2 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_session_activity_persist` | oracle/test | P2 | 320 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_session_id_env` | oracle/test | P2 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_session_meta_filtering` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_session_reset_fix` | oracle/test | P2 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_session_source` | oracle/test | P2 | 31 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_start_order_gate` | oracle/test | P2 | 251 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_steer` | oracle/test | P2 | 546 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_stream_drop_logging` | oracle/test | P2 | 220 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_stream_interrupt_retry` | oracle/test | P2 | 242 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_stream_single_writer_65991` | oracle/test | P2 | 248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_stream_stale_breaker_reset` | oracle/test | P2 | 187 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_stream_stale_circuit_breaker` | oracle/test | P2 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_streaming` | oracle/test | P2 | 1,637 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_streaming_tool_call_repair` | oracle/test | P2 | 61 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_strict_api_validation` | oracle/test | P2 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_strip_reasoning_tags_cli` | oracle/test | P2 | 28 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_summarize_api_error` | oracle/test | P2 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_switch_model_context` | oracle/test | P2 | 286 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_switch_model_fallback_prune` | oracle/test | P2 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_switch_model_pool_reload_52727` | oracle/test | P2 | 219 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_switch_model_reapplies_headers` | oracle/test | P2 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_switch_model_reasoning_override` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_switch_model_rollback` | oracle/test | P2 | 204 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_switch_model_stale_base_url` | oracle/test | P2 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_thinking_only_sanitizer` | oracle/test | P2 | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_thinking_sig_recovery_persistence` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tls_fd_recycle_corruption` | oracle/test | P2 | 412 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_token_persistence_non_cli` | oracle/test | P2 | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tool_arg_coercion` | oracle/test | P2 | 248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tool_batch_segmentation` | oracle/test | P2 | 728 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tool_call_args_sanitizer` | oracle/test | P2 | 160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tool_call_guardrail_runtime` | oracle/test | P2 | 422 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tool_call_incremental_persistence` | oracle/test | P2 | 534 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tool_executor_contextvar_propagation` | oracle/test | P2 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_tool_name_db_persistence` | oracle/test | P2 | 45 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_turn_completion_explainer` | oracle/test | P2 | 173 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_unicode_ascii_codec` | oracle/test | P2 | 298 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_verification_continuation_budget` | oracle/test | P2 | 255 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_vision_aware_preprocessing` | oracle/test | P2 | 190 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_vision_tool_messages` | oracle/test | P2 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_agent.test_wait_state_visibility` | oracle/test | P2 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.run_interrupt_test` | oracle/test | oracle | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.scripts.test_build_skills_index_health` | oracle/test | oracle | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.scripts.test_contributor_map` | oracle/test | oracle | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.scripts.test_footgun_subprocess_encoding` | oracle/test | oracle | 234 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.scripts.test_smoke_nemo_relay_shared_metrics` | oracle/test | oracle | 41 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.secret_sources.__init__` | oracle/test | oracle | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.secret_sources.conformance` | oracle/test | oracle | 123 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.secret_sources.test_error_remediation` | oracle/test | oracle | 151 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.secret_sources.test_profile_secrets` | oracle/test | oracle | 177 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.secret_sources.test_secret_source_registry` | oracle/test | oracle | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_actual_setup_skill` | oracle/test | oracle | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_cloudflare_temporary_deploy_skill` | oracle/test | oracle | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_darwinian_evolver_skill` | oracle/test | oracle | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_document_to_action_items_skill` | oracle/test | oracle | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_fetch_transcript` | oracle/test | oracle | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_github_credential_token` | oracle/test | oracle | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_google_workspace_api` | oracle/test | oracle | 197 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_google_workspace_credential_files` | oracle/test | oracle | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_google_workspace_setup` | oracle/test | oracle | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_google_workspace_setup_deps` | oracle/test | oracle | 181 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_grounded_citations_skill` | oracle/test | oracle | 606 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_hyperliquid_skill` | oracle/test | oracle | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_mcp_oauth_remote_gateway_skill` | oracle/test | oracle | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_memento_cards` | oracle/test | oracle | 242 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_office_document_skills` | oracle/test | oracle | 173 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_openclaw_migration` | oracle/test | oracle | 755 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_openclaw_migration_hardening` | oracle/test | oracle | 323 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_pinecone_research_skill` | oracle/test | oracle | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_telephony_skill` | oracle/test | oracle | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_tldraw_offline_skill` | oracle/test | oracle | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_unbroker_skill` | oracle/test | oracle | 772 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_xurl_article_ingestion_docs` | oracle/test | oracle | 29 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_xurl_x_search_routing` | oracle/test | oracle | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.skills.test_youtube_quiz` | oracle/test | oracle | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.state.test_compression_lineage_guard` | oracle/test | oracle | 298 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.state.test_disk_full_error` | oracle/test | oracle | 30 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.state.test_fts_runtime_rebuild` | oracle/test | oracle | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.state.test_no_more_rows_retry` | oracle/test | oracle | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.state.test_session_model_usage_pk_heal` | oracle/test | oracle | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.state.test_write_lock_patience` | oracle/test | oracle | 150 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress._fake_worker` | oracle/test | oracle | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.conftest` | oracle/test | oracle | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_atypical_scenarios` | oracle/test | oracle | 1,059 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_benchmarks` | oracle/test | oracle | 221 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_concurrency` | oracle/test | oracle | 301 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_concurrency_mixed` | oracle/test | oracle | 350 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_concurrency_parent_gate` | oracle/test | oracle | 183 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_concurrency_reclaim_race` | oracle/test | oracle | 241 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_property_fuzzing` | oracle/test | oracle | 282 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.stress.test_subprocess_e2e` | oracle/test | oracle | 228 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_account_usage` | oracle/test | oracle | 203 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_atomic_replace_symlinks` | oracle/test | oracle | 247 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_atomic_write_text_metadata` | oracle/test | oracle | 186 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_audio_playback_guard` | oracle/test | oracle | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_background_review_list_shapes` | oracle/test | oracle | 313 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_background_review_session_isolation` | oracle/test | oracle | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_base_url_hostname` | oracle/test | oracle | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_batch_runner_checkpoint` | oracle/test | oracle | 241 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_batch_runner_durability` | oracle/test | oracle | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_bitwarden_secrets` | oracle/test | oracle | 523 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_cli_manual_compress` | oracle/test | oracle | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_cli_skin_integration` | oracle/test | oracle | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_code_skew` | oracle/test | oracle | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_command_secret_source` | oracle/test | oracle | 271 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_conftest_wal_gate` | oracle/test | oracle | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_copilot_initiator` | oracle/test | oracle | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_credential_file_permissions` | oracle/test | oracle | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_ctx_halving_fix` | oracle/test | oracle | 286 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_delegate_cascade_49148` | oracle/test | oracle | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_dispatch_session_id` | oracle/test | oracle | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_empty_model_fallback` | oracle/test | oracle | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_empty_session_hygiene` | oracle/test | oracle | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_engines_satisfiable` | oracle/test | oracle | 185 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_env_loader_applied_homes` | oracle/test | oracle | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_env_loader_op_bootstrap` | oracle/test | oracle | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_env_loader_secret_sources` | oracle/test | oracle | 523 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_estop` | oracle/test | oracle | 287 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_evidence_store` | oracle/test | oracle | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_fast_safe_load` | oracle/test | oracle | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_fts_cjk_bigram` | oracle/test | oracle | 244 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_fts_update_of_narrowing` | oracle/test | oracle | 283 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_gateway_streaming_nested_config` | oracle/test | oracle | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_get_tool_definitions_cache_isolation` | oracle/test | oracle | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_bootstrap` | oracle/test | oracle | 370 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_constants` | oracle/test | oracle | 667 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_home_profile_warning` | oracle/test | oracle | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_logging` | oracle/test | oracle | 670 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_state` | oracle/test | oracle | 4,397 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_state_compression_busy_retry` | oracle/test | oracle | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_state_compression_locks` | oracle/test | oracle | 225 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_state_readonly_preflight` | oracle/test | oracle | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermes_state_wal_fallback` | oracle/test | oracle | 557 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_hermetic_side_effect_guards` | oracle/test | oracle | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_honcho_client_concurrency` | oracle/test | oracle | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_honcho_client_config` | oracle/test | oracle | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_honcho_session_context` | oracle/test | oracle | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_honcho_startup_fail_open` | oracle/test | oracle | 326 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_autostash_conflict_recovery` | oracle/test | oracle | 195 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_commit_pin_rollback` | oracle/test | oracle | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_diverged_update` | oracle/test | oracle | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_lockfile_churn` | oracle/test | oracle | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_macos_launcher` | oracle/test | oracle | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_no_initial_commit` | oracle/test | oracle | 136 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_ps1_ascii_only` | oracle/test | oracle | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_ps1_native_stderr_eap` | oracle/test | oracle | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_ps1_node_path_for_npm` | oracle/test | oracle | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_ps1_python_fallback_venv` | oracle/test | oracle | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_ps1_uv_powershell_host` | oracle/test | oracle | 77 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_ps1_web_server_syntax_probe` | oracle/test | oracle | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_acp_launcher` | oracle/test | oracle | 214 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_bootstrap_marker` | oracle/test | oracle | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_browser_install` | oracle/test | oracle | 202 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_install_method_stamp` | oracle/test | oracle | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_node_global_prefix` | oracle/test | oracle | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_pythonpath_sanitization` | oracle/test | oracle | 30 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_root_fhs_uv_python_path` | oracle/test | oracle | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_setup_wizard_tty_probe` | oracle/test | oracle | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_symlink_stomp` | oracle/test | oracle | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_sh_termux_network_prereqs` | oracle/test | oracle | 22 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_install_unmerged_index` | oracle/test | oracle | 186 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_ipv4_preference` | oracle/test | oracle | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_iron_proxy` | oracle/test | oracle | 816 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_iron_proxy_cli` | oracle/test | oracle | 340 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_iron_proxy_e2e` | oracle/test | oracle | 368 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_journal_mode_config` | oracle/test | oracle | 228 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_lazy_session_regressions` | oracle/test | oracle | 372 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_list_recent_user_messages_handoffs` | oracle/test | oracle | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_live_system_guard` | oracle/test | oracle | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_live_system_guard_self_test` | oracle/test | oracle | 309 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_log_isolation` | oracle/test | oracle | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_managed_runtime_resolution` | oracle/test | oracle | 203 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_mcp_serve` | oracle/test | oracle | 1,387 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_message_reactions` | oracle/test | oracle | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_mini_swe_runner` | oracle/test | oracle | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_minimax_model_validation` | oracle/test | oracle | 84 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_minimax_oauth` | oracle/test | oracle | 570 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_minisweagent_path` | oracle/test | oracle | 2 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_model_forces_max_completion_tokens` | oracle/test | oracle | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_model_picker_scroll` | oracle/test | oracle | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_model_tools` | oracle/test | oracle | 511 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_model_tools_async_bridge` | oracle/test | oracle | 447 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_no_shadowed_test_definitions` | oracle/test | oracle | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_ollama_num_ctx` | oracle/test | oracle | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_onepassword_secrets` | oracle/test | oracle | 301 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_output_cap_parsing` | oracle/test | oracle | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_packaging_build_guard` | oracle/test | oracle | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_packaging_metadata` | oracle/test | oracle | 342 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_plugin_skills` | oracle/test | oracle | 425 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_plugin_utils` | oracle/test | oracle | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_process_loop_event_loop_warning` | oracle/test | oracle | 130 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_profile_isolation_runtime` | oracle/test | oracle | 159 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_project_metadata` | oracle/test | oracle | 291 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_pty_keepalive_ws` | oracle/test | oracle | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_pty_session` | oracle/test | oracle | 145 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_retry_utils` | oracle/test | oracle | 210 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_run_tests_parallel` | oracle/test | oracle | 380 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_run_tests_parallel_stdio` | oracle/test | oracle | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_sanitize_tool_error` | oracle/test | oracle | 111 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_schema_read_probe` | oracle/test | oracle | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_search_slow_query_log` | oracle/test | oracle | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_secret_scope_plugin_families` | oracle/test | oracle | 260 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_session_db_read_path_split` | oracle/test | oracle | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_session_skill_previews` | oracle/test | oracle | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_session_system_prompt_dedup` | oracle/test | oracle | 267 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_session_vacuum_config` | oracle/test | oracle | 41 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_session_workspace_binding` | oracle/test | oracle | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_slack_thread_require_mention` | oracle/test | oracle | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_slash_worker_watchdog` | oracle/test | oracle | 23 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_sql_injection` | oracle/test | oracle | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_sqlite_lock_safe_inspection` | oracle/test | oracle | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_sqlite_wal_reset_gate` | oracle/test | oracle | 413 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_stale_tool_call_marker_session_repair` | oracle/test | oracle | 262 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_stale_utils_module_import` | oracle/test | oracle | 90 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_state_db_malformed_repair` | oracle/test | oracle | 397 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_subprocess_home_isolation` | oracle/test | oracle | 261 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_telegram_polling_progress_ptb` | oracle/test | oracle | 304 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_termux_all_extra_compat` | oracle/test | oracle | 23 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_timezone` | oracle/test | oracle | 274 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tini_shim` | oracle/test | oracle | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_toolset_distributions` | oracle/test | oracle | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_toolsets` | oracle/test | oracle | 283 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_trajectory_compressor` | oracle/test | oracle | 496 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_trajectory_compressor_async` | oracle/test | oracle | 201 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_transform_llm_output_hook` | oracle/test | oracle | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_transform_tool_result_hook` | oracle/test | oracle | 166 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tui_entry_mcp_owner` | oracle/test | oracle | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tui_gateway_loop_noise` | oracle/test | oracle | 88 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tui_gateway_queue_on_busy` | oracle/test | oracle | 361 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tui_gateway_server` | oracle/test | oracle | 16,411 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tui_gateway_server_crash_history` | oracle/test | oracle | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tui_gateway_ws` | oracle/test | oracle | 230 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_tui_mcp_late_refresh` | oracle/test | oracle | 167 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_utils_truthy_values` | oracle/test | oracle | 29 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_voice_max_recording_seconds` | oracle/test | oracle | 32 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_wal_checkpoint_strategy` | oracle/test | oracle | 138 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_web_server` | oracle/test | oracle | 242 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_web_server_sessiondb_eventloop` | oracle/test | oracle | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_web_server_status_topology_cache` | oracle/test | oracle | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_windows_subprocess_no_window_flags` | oracle/test | oracle | 374 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_yaml_indent_consistency_31999` | oracle/test | oracle | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_yuanbao_integration` | oracle/test | oracle | 322 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_yuanbao_markdown` | oracle/test | oracle | 180 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_yuanbao_pipeline` | oracle/test | oracle | 1,348 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_yuanbao_proto` | oracle/test | oracle | 472 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_yuanbao_reconnect_set_active` | oracle/test | oracle | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_yuanbao_shutdown` | oracle/test | oracle | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.test_zeroed_state_db` | oracle/test | oracle | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.__init__` | oracle/test | P2 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.conftest` | oracle/test | P2 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_accretion_caps` | oracle/test | P2 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_ansi_strip` | oracle/test | P2 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_approval` | oracle/test | P2 | 1,570 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_approval_config_readonly` | oracle/test | P2 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_approval_deny_rules` | oracle/test | P2 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_approval_interrupt` | oracle/test | P2 | 160 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_approval_mode_parity` | oracle/test | P2 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_approval_plugin_hooks` | oracle/test | P2 | 345 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_approved_command_clean_slate` | oracle/test | P2 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_async_delegation` | oracle/test | P2 | 763 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_async_delegation_fd_leak` | oracle/test | P2 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_audio_container` | oracle/test | P2 | 166 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_base_environment` | oracle/test | P2 | 407 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_blocked_command_guidance` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_blueprints` | oracle/test | P2 | 164 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_camofox` | oracle/test | P2 | 364 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_camofox_auth` | oracle/test | P2 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_camofox_ensure_tab` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_camofox_persistence` | oracle/test | P2 | 319 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_camofox_private_page_guard` | oracle/test | P2 | 170 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_camofox_state` | oracle/test | P2 | 45 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_camofox_timeout` | oracle/test | P2 | 32 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_cdp_override` | oracle/test | P2 | 343 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_cdp_tool` | oracle/test | P2 | 420 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_chromium_autoinstall` | oracle/test | P2 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_chromium_check` | oracle/test | P2 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_cleanup` | oracle/test | P2 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_cloud_fallback` | oracle/test | P2 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_cloud_provider_cache` | oracle/test | P2 | 98 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_command_timeout_race` | oracle/test | P2 | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_console` | oracle/test | P2 | 412 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_console_ssrf` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_content_none_guard` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_eval_ssrf` | oracle/test | P2 | 284 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_eval_supervisor_path` | oracle/test | P2 | 305 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_get_images_ssrf` | oracle/test | P2 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_hardening` | oracle/test | P2 | 302 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_headed_mode` | oracle/test | P2 | 173 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_homebrew_paths` | oracle/test | P2 | 235 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_hybrid_routing` | oracle/test | P2 | 191 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_lightpanda` | oracle/test | P2 | 430 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_open_timeout` | oracle/test | P2 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_orphan_reaper` | oracle/test | P2 | 357 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_private_page_action_guard` | oracle/test | P2 | 165 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_secret_exfil` | oracle/test | P2 | 336 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_snapshot_ssrf` | oracle/test | P2 | 380 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_ssrf_local` | oracle/test | P2 | 350 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_supervisor` | oracle/test | P2 | 353 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_supervisor_healthcheck` | oracle/test | P2 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_type_redaction` | oracle/test | P2 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_browser_use_session_expiry` | oracle/test | P2 | 93 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_budget_config` | oracle/test | P2 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_build_subprocess_env` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_checkpoint_manager` | oracle/test | P2 | 1,046 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_clarify_gateway` | oracle/test | P2 | 343 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_clarify_tool` | oracle/test | P2 | 290 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_clipboard` | oracle/test | P2 | 561 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_code_execution` | oracle/test | P2 | 801 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_code_execution_modes` | oracle/test | P2 | 377 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_code_execution_windows_env` | oracle/test | P2 | 661 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_command_guards` | oracle/test | P2 | 436 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use` | oracle/test | P2 | 2,131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_approval_isolation` | oracle/test | P2 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_capture_routing` | oracle/test | P2 | 309 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_cua_0_10_permissions` | oracle/test | P2 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_cua_0_9` | oracle/test | P2 | 872 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_cua_backend_linux` | oracle/test | P2 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_delivery_ladder` | oracle/test | P2 | 370 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_null_pid_windows` | oracle/test | P2 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_computer_use_vision_routing` | oracle/test | P2 | 209 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_config_null_guard` | oracle/test | P2 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_container_cwd_sanitize` | oracle/test | P2 | 214 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_credential_files` | oracle/test | P2 | 574 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_credential_pool_env_fallback` | oracle/test | P2 | 284 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_cron_approval_mode` | oracle/test | P2 | 477 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_cron_prompt_injection` | oracle/test | P2 | 60 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_cronjob_run_background` | oracle/test | P2 | 308 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_cronjob_run_immediate` | oracle/test | P2 | 210 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_cronjob_tools` | oracle/test | P2 | 617 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_cross_profile_guard` | oracle/test | P2 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_daemon_pool` | oracle/test | P2 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_daytona_environment` | oracle/test | P2 | 325 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_debug_helpers` | oracle/test | P2 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate` | oracle/test | P2 | 1,752 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_apiserver_background` | oracle/test | P2 | 167 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_batch_validation` | oracle/test | P2 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_composite_toolsets` | oracle/test | P2 | 31 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_cost_footer` | oracle/test | P2 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_kanban_isolation` | oracle/test | P2 | 298 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_output_schema` | oracle/test | P2 | 363 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_subagent_timeout_diagnostic` | oracle/test | P2 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_summary_budget` | oracle/test | P2 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegate_toolset_scope` | oracle/test | P2 | 75 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_delegation_live_log` | oracle/test | P2 | 302 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_denial_circuit_breaker` | oracle/test | P2 | 230 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_desktop_ui` | oracle/test | P2 | 30 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_discord_send_message_caption` | oracle/test | P2 | 133 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_discord_tool` | oracle/test | P2 | 756 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_cgroup_limits` | oracle/test | P2 | 61 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_config_migrate` | oracle/test | P2 | 257 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_daemon_redirect` | oracle/test | P2 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_environment` | oracle/test | P2 | 1,490 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_find` | oracle/test | P2 | 48 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_network_config` | oracle/test | P2 | 116 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_orphan_reaper_integration` | oracle/test | P2 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_docker_rebootstrap_nous_session` | oracle/test | P2 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_dockerfile_immutable_install` | oracle/test | P2 | 117 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_dockerfile_node_modules_perms` | oracle/test | P2 | 22 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_dockerfile_pid1_reaping` | oracle/test | P2 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_ensure_task_env` | oracle/test | P2 | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_env_passthrough` | oracle/test | P2 | 413 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_env_probe` | oracle/test | P2 | 319 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_execute_code_approval_cluster` | oracle/test | P2 | 501 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_execution_flag_detection` | oracle/test | P2 | 297 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_fal_common` | oracle/test | P2 | 605 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_feishu_tools` | oracle/test | P2 | 42 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_operations` | oracle/test | P2 | 675 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_operations_edge_cases` | oracle/test | P2 | 312 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_ops_cwd_tracking` | oracle/test | P2 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_read_guards` | oracle/test | P2 | 767 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_staleness` | oracle/test | P2 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_state_registry` | oracle/test | P2 | 190 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_sync` | oracle/test | P2 | 412 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_sync_back` | oracle/test | P2 | 459 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_sync_perf` | oracle/test | P2 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_sync_sigint` | oracle/test | P2 | 56 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_tools` | oracle/test | P2 | 996 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_tools_container_config` | oracle/test | P2 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_tools_cwd_resolution` | oracle/test | P2 | 312 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_tools_live` | oracle/test | P2 | 263 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_tools_tilde_profile` | oracle/test | P2 | 86 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_file_write_safety` | oracle/test | P2 | 605 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_find_shell` | oracle/test | P2 | 253 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_flux3_video_tool` | oracle/test | P2 | 1,043 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_focus_pane_tool` | oracle/test | P2 | 38 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_force_dangerous_override` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_fuzzy_match` | oracle/test | P2 | 610 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_gateway_cwd_contract` | oracle/test | P2 | 43 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_gnu_long_option_abbreviation_bypass` | oracle/test | P2 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_hardline_blocklist` | oracle/test | P2 | 672 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_heartbeat_stale_thresholds` | oracle/test | P2 | 27 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_hermes_subprocess_env` | oracle/test | P2 | 214 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_hidden_dir_filter` | oracle/test | P2 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_homeassistant_tool` | oracle/test | P2 | 381 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_hook_output_spill` | oracle/test | P2 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_hub_lock_non_utf8_68053` | oracle/test | P2 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_image_generation` | oracle/test | P2 | 421 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_image_generation_artifacts` | oracle/test | P2 | 149 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_image_generation_env` | oracle/test | P2 | 46 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_image_generation_image_to_image` | oracle/test | P2 | 287 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_image_generation_plugin_dispatch` | oracle/test | P2 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_image_source` | oracle/test | P2 | 332 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_init_session_cwd_respect` | oracle/test | P2 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_interrupt` | oracle/test | P2 | 303 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_kanban_comment_injection` | oracle/test | P2 | 124 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_kanban_redaction` | oracle/test | P2 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_kanban_tools` | oracle/test | P2 | 1,025 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_lazy_deps` | oracle/test | P2 | 430 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_lazy_deps_durable_target` | oracle/test | P2 | 276 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_lazy_deps_managed` | oracle/test | P2 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_line_ending_preservation` | oracle/test | P2 | 177 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_llm_content_none_guard` | oracle/test | P2 | 179 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_background_child_hang` | oracle/test | P2 | 140 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_cwd_permission_fallback` | oracle/test | P2 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_env_blocklist` | oracle/test | P2 | 724 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_env_cwd_recovery` | oracle/test | P2 | 173 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_env_relative_cwd` | oracle/test | P2 | 28 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_env_session_leak` | oracle/test | P2 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_env_windows_msys` | oracle/test | P2 | 342 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_interrupt_cleanup` | oracle/test | P2 | 202 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_shell_init` | oracle/test | P2 | 226 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_local_tempdir` | oracle/test | P2 | 32 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_managed_browserbase_and_modal` | oracle/test | P2 | 308 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_managed_media_gateways` | oracle/test | P2 | 341 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_managed_modal_environment` | oracle/test | P2 | 195 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_managed_tool_gateway` | oracle/test | P2 | 412 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_bridge_single_failure` | oracle/test | P2 | 155 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_cancelled_error_propagation` | oracle/test | P2 | 91 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_capability_gating` | oracle/test | P2 | 293 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_circuit_breaker` | oracle/test | P2 | 571 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_client_cert` | oracle/test | P2 | 326 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_config_whitespace_warning` | oracle/test | P2 | 125 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_dashboard_oauth` | oracle/test | P2 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_discovery_cross_process` | oracle/test | P2 | 187 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_dynamic_discovery` | oracle/test | P2 | 129 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_elicitation` | oracle/test | P2 | 251 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_empty_error_message` | oracle/test | P2 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_failure_classification` | oracle/test | P2 | 169 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_identity_header` | oracle/test | P2 | 283 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_image_content` | oracle/test | P2 | 121 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_initial_connect_shutdown` | oracle/test | P2 | 273 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_invalid_url` | oracle/test | P2 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_lazy_start` | oracle/test | P2 | 331 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_list_pagination` | oracle/test | P2 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_loop_profile_override` | oracle/test | P2 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_oauth` | oracle/test | P2 | 836 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_oauth_bidirectional` | oracle/test | P2 | 210 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_oauth_cold_load_expiry` | oracle/test | P2 | 474 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_oauth_integration` | oracle/test | P2 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_oauth_manager` | oracle/test | P2 | 366 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_oauth_metadata` | oracle/test | P2 | 152 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_parked_self_probe` | oracle/test | P2 | 137 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_poll_loop_oom_integration` | oracle/test | P2 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_preflight_content_type` | oracle/test | P2 | 286 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_probe` | oracle/test | P2 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_rapid_drop_budget` | oracle/test | P2 | 207 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_reconnect_log_hygiene` | oracle/test | P2 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_reconnect_retry_reset` | oracle/test | P2 | 196 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_reconnect_signal` | oracle/test | P2 | 33 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_register_wakes_stale` | oracle/test | P2 | 62 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_resource_content` | oracle/test | P2 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_schema_cache` | oracle/test | P2 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_server_log_notifications` | oracle/test | P2 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_sse_transport` | oracle/test | P2 | 209 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_stability` | oracle/test | P2 | 710 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_stdio_encoding_handler` | oracle/test | P2 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_stdio_init_timeout` | oracle/test | P2 | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_stdio_watchdog` | oracle/test | P2 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_structured_content` | oracle/test | P2 | 109 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_tool` | oracle/test | P2 | 2,789 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_tool_401_handling` | oracle/test | P2 | 106 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_tool_issue_948` | oracle/test | P2 | 148 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_tool_session_expired` | oracle/test | P2 | 404 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_transport_group_reconnect` | oracle/test | P2 | 115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_trust_gating` | oracle/test | P2 | 247 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_mcp_utility_capability_gating` | oracle/test | P2 | 165 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_media_caption_split` | oracle/test | P2 | 47 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_memory_tool` | oracle/test | P2 | 629 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_memory_tool_import_fallback` | oracle/test | P2 | 31 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_memory_tool_schema` | oracle/test | P2 | 40 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_microsoft_graph_auth` | oracle/test | P2 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_microsoft_graph_client` | oracle/test | P2 | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_modal_bulk_upload` | oracle/test | P2 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_modal_sandbox_fixes` | oracle/test | P2 | 439 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_modal_snapshot_isolation` | oracle/test | P2 | 228 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_notify_on_complete` | oracle/test | P2 | 437 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_open_preview_tool` | oracle/test | P2 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_osv_check` | oracle/test | P2 | 219 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_parse_env_var` | oracle/test | P2 | 63 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_patch_already_applied` | oracle/test | P2 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_patch_failure_tracking` | oracle/test | P2 | 112 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_patch_multimatch_locations` | oracle/test | P2 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_patch_parser` | oracle/test | P2 | 915 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_patch_ws_diagnosis` | oracle/test | P2 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_pr_6656_regressions` | oracle/test | P2 | 271 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_process_registry` | oracle/test | P2 | 2,104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_process_wait_clarity` | oracle/test | P2 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_read_extract` | oracle/test | P2 | 489 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_read_loop_detection` | oracle/test | P2 | 244 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_read_preview_tool` | oracle/test | P2 | 65 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_refresh_agent_mcp_tools` | oracle/test | P2 | 226 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_registry` | oracle/test | P2 | 585 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_request_tool_approval` | oracle/test | P2 | 153 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_resolve_path` | oracle/test | P2 | 39 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_restored_delegation_ownership` | oracle/test | P2 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_sandbox_failure_hints` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_schema_sanitizer` | oracle/test | P2 | 519 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_search_auto_multiline` | oracle/test | P2 | 51 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_search_budget_truncation` | oracle/test | P2 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_search_error_guard` | oracle/test | P2 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_search_hidden_dirs` | oracle/test | P2 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_search_zero_match_and_multipath` | oracle/test | P2 | 161 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_send_message_missing_platforms` | oracle/test | P2 | 300 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_send_message_react` | oracle/test | P2 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_send_message_slack` | oracle/test | P2 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_send_message_target_parse` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_send_message_telegram_proxy` | oracle/test | P2 | 157 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_send_message_tool` | oracle/test | P2 | 1,826 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_session_cwd_store` | oracle/test | P2 | 208 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_session_search` | oracle/test | P2 | 825 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_shared_container_task_id` | oracle/test | P2 | 69 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_shell_bypass_denylist` | oracle/test | P2 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_signal_media` | oracle/test | P2 | 204 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_singularity_preflight` | oracle/test | P2 | 55 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_bundle_provenance` | oracle/test | P2 | 181 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_env_passthrough` | oracle/test | P2 | 80 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_improvements` | oracle/test | P2 | 100 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_manager_tool` | oracle/test | P2 | 949 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_provenance` | oracle/test | P2 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_size_limits` | oracle/test | P2 | 177 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_usage` | oracle/test | P2 | 555 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_view_dedup` | oracle/test | P2 | 94 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_view_path_check` | oracle/test | P2 | 104 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skill_view_traversal` | oracle/test | P2 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_ast_audit` | oracle/test | P2 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_guard` | oracle/test | P2 | 451 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_hub` | oracle/test | P2 | 1,626 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_hub_browse_sh` | oracle/test | P2 | 85 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_hub_clawhub` | oracle/test | P2 | 624 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_list_modified_diff` | oracle/test | P2 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_sync` | oracle/test | P2 | 924 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_sync_client` | oracle/test | P2 | 1,115 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_tool` | oracle/test | P2 | 948 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_tool_discovery_cache` | oracle/test | P2 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_skills_tool_profile_scope` | oracle/test | P2 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_slack_send_message_media` | oracle/test | P2 | 176 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_slash_confirm` | oracle/test | P2 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_smart_approval_injection` | oracle/test | P2 | 175 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_smart_approval_policy` | oracle/test | P2 | 114 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_snapshot_multiline_session_env_injection` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_snapshot_session_id_leak` | oracle/test | P2 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_spotify_client` | oracle/test | P2 | 120 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_ssh_bulk_upload` | oracle/test | P2 | 267 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_ssh_environment` | oracle/test | P2 | 233 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_stage2_hook_seed_one_symlinks` | oracle/test | P2 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_stage2_hook_symlink_chown` | oracle/test | P2 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_stt_cloud_trim` | oracle/test | P2 | 317 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_stt_default_language` | oracle/test | P2 | 22 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_stt_idle_unload` | oracle/test | P2 | 283 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_stt_language_resolution` | oracle/test | P2 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_stt_silence_hallucinations` | oracle/test | P2 | 134 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_subagent_steer` | oracle/test | P2 | 899 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_subprocess_stdin_guard` | oracle/test | P2 | 70 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_subprocess_utf8_encoding` | oracle/test | P2 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_symlink_prefix_confusion` | oracle/test | P2 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_sync_back_backends` | oracle/test | P2 | 412 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_telegram_send_message_caption` | oracle/test | P2 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_compound_background` | oracle/test | P2 | 102 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_config_env_sync` | oracle/test | P2 | 324 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_cwd_echo` | oracle/test | P2 | 68 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_degraded_mode` | oracle/test | P2 | 223 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_env_bridge` | oracle/test | P2 | 154 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_exit_semantics` | oracle/test | P2 | 81 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_foreground_timeout_cap` | oracle/test | P2 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_hints` | oracle/test | P2 | 122 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_none_command_guard` | oracle/test | P2 | 21 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_output_transform_hook` | oracle/test | P2 | 206 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_requirements` | oracle/test | P2 | 208 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_task_cwd` | oracle/test | P2 | 183 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_timeout_output` | oracle/test | P2 | 27 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_tool` | oracle/test | P2 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_tool_pty_fallback` | oracle/test | P2 | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_tool_requirements` | oracle/test | P2 | 362 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_terminal_truncation_spill` | oracle/test | P2 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_termux_api_detection` | oracle/test | P2 | 242 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_threaded_process_handle` | oracle/test | P2 | 108 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_threat_patterns` | oracle/test | P2 | 262 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tirith_security` | oracle/test | P2 | 716 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_todo_tool` | oracle/test | P2 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_todo_tool_type_coercion` | oracle/test | P2 | 79 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tool_backend_helpers` | oracle/test | P2 | 306 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tool_output_limits` | oracle/test | P2 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tool_result_storage` | oracle/test | P2 | 322 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tool_search` | oracle/test | P2 | 618 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tool_search_context_provider` | oracle/test | P2 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_transcription` | oracle/test | P2 | 290 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_transcription_command_providers` | oracle/test | P2 | 376 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_transcription_deepinfra` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_transcription_dotenv_fallback` | oracle/test | P2 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_transcription_plugin_dispatch` | oracle/test | P2 | 332 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_transcription_tools` | oracle/test | P2 | 1,232 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_command_providers` | oracle/test | P2 | 545 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_container_repair` | oracle/test | P2 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_deepinfra` | oracle/test | P2 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_dotenv_fallback` | oracle/test | P2 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_gemini` | oracle/test | P2 | 222 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_instructions` | oracle/test | P2 | 116 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_kittentts` | oracle/test | P2 | 131 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_macos_output` | oracle/test | P2 | 118 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_max_text_length` | oracle/test | P2 | 135 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_minimax_region` | oracle/test | P2 | 190 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_mistral` | oracle/test | P2 | 168 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_model_cache_lru` | oracle/test | P2 | 35 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_openai_config` | oracle/test | P2 | 83 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_opus_routing` | oracle/test | P2 | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_output_timestamp` | oracle/test | P2 | 29 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_path_traversal` | oracle/test | P2 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_piper` | oracle/test | P2 | 246 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_plugin_dispatch` | oracle/test | P2 | 227 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_prepare_spoken` | oracle/test | P2 | 141 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_provider_base_urls` | oracle/test | P2 | 66 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_pythonpath_fallback` | oracle/test | P2 | 147 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_response_body_cap` | oracle/test | P2 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_speed` | oracle/test | P2 | 278 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_streaming` | oracle/test | P2 | 917 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_streaming_e2e` | oracle/test | P2 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_text_normalize` | oracle/test | P2 | 49 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_tts_xai_speech_tags` | oracle/test | P2 | 248 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_url_safety` | oracle/test | P2 | 498 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_vercel_sandbox_environment` | oracle/test | P2 | 621 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_video_analyze` | oracle/test | P2 | 230 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_video_generation_dispatch` | oracle/test | P2 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_video_generation_dynamic_schema` | oracle/test | P2 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_video_generation_tool_surface_matrix` | oracle/test | P2 | 236 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_vision_native_fast_path` | oracle/test | P2 | 249 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_vision_region` | oracle/test | P2 | 197 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_vision_tools` | oracle/test | P2 | 995 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_cli_integration` | oracle/test | P2 | 675 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_credential_pool_resolution` | oracle/test | P2 | 194 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_mode` | oracle/test | P2 | 1,596 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_mode_playback_env_scrub` | oracle/test | P2 | 33 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_stop_phrase` | oracle/test | P2 | 219 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_thinking_sound` | oracle/test | P2 | 139 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_tts_echo_guard` | oracle/test | P2 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_voice_wsl_pipewire` | oracle/test | P2 | 58 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_wake_word` | oracle/test | P2 | 714 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_watch_patterns` | oracle/test | P2 | 338 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_extract_robustness` | oracle/test | P2 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_providers` | oracle/test | P2 | 472 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_providers_brave_free` | oracle/test | P2 | 180 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_providers_ddgs` | oracle/test | P2 | 293 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_providers_searxng` | oracle/test | P2 | 251 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_providers_xai` | oracle/test | P2 | 584 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_tools_config` | oracle/test | P2 | 648 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_tools_dict_urls` | oracle/test | P2 | 89 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_tools_tavily` | oracle/test | P2 | 192 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_web_tools_truncate` | oracle/test | P2 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_website_policy` | oracle/test | P2 | 303 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_whatsapp_send_message_media` | oracle/test | P2 | 157 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_windows_compat` | oracle/test | P2 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_windows_native_support` | oracle/test | P2 | 1,154 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_working_diff` | oracle/test | P2 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_write_approval` | oracle/test | P2 | 288 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_write_deny` | oracle/test | P2 | 95 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_write_file_syntax_gate` | oracle/test | P2 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_write_verification` | oracle/test | P2 | 73 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_x_search_tool` | oracle/test | P2 | 314 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_xai_http_credentials` | oracle/test | P2 | 57 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_xai_http_storage` | oracle/test | P2 | 59 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_yolo_mode` | oracle/test | P2 | 218 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tools.test_zombie_process_cleanup` | oracle/test | P2 | 532 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.__init__` | oracle/test | P5 | 0 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_auto_continue` | oracle/test | P5 | 376 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_billing_rpc` | oracle/test | P5 | 96 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_change_watcher` | oracle/test | P5 | 193 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_codex_app_server_live_events` | oracle/test | P5 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_cold_start_gil_stall` | oracle/test | P5 | 197 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_compaction_status` | oracle/test | P5 | 67 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_compress_lock_skip` | oracle/test | P5 | 172 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_compute_host` | oracle/test | P5 | 128 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_compute_host_phase1` | oracle/test | P5 | 307 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_custom_provider_session_persistence` | oracle/test | P5 | 345 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_delegation_session_lifecycle` | oracle/test | P5 | 158 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_entry_import_off_main_thread` | oracle/test | P5 | 76 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_entry_picker_prewarm` | oracle/test | P5 | 101 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_entry_sys_path` | oracle/test | P5 | 71 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_failed_turn_retention` | oracle/test | P5 | 291 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_fast_session_scope` | oracle/test | P5 | 126 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_finalize_session_persist` | oracle/test | P5 | 273 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_gateway_owned_session_reap` | oracle/test | P5 | 74 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_goal_command` | oracle/test | P5 | 156 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_gui_surface_toolsets` | oracle/test | P5 | 113 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_image_routing_stale_model` | oracle/test | P5 | 34 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_inline_rpc_gil_starvation` | oracle/test | P5 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_interim_assistant_callback` | oracle/test | P5 | 52 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_iso_certify_seam` | oracle/test | P5 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_kanban_notify_poller` | oracle/test | P5 | 322 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_make_agent_provider` | oracle/test | P5 | 142 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_mcp_late_refresh_thread_owner` | oracle/test | P5 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_mcp_reload_rev` | oracle/test | P5 | 186 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_moa_reference_emit` | oracle/test | P5 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_model_switch_marker_role` | oracle/test | P5 | 119 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_pet_generate_rpc` | oracle/test | P5 | 64 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_project_tree` | oracle/test | P5 | 563 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_projects_rpc` | oracle/test | P5 | 381 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_protocol` | oracle/test | P5 | 667 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_reasoning_config_per_model` | oracle/test | P5 | 82 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_reasoning_session_scope` | oracle/test | P5 | 103 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_render` | oracle/test | P5 | 31 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_review_summary_callback` | oracle/test | P5 | 144 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_session_cwd_follow` | oracle/test | P5 | 210 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_session_db_ownership_teardown` | oracle/test | P5 | 386 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_session_id_injection` | oracle/test | P5 | 72 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_session_images_dir` | oracle/test | P5 | 54 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_session_platform_resolution` | oracle/test | P5 | 99 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_session_reclaim_notify` | oracle/test | P5 | 87 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_session_resume_db_ownership` | oracle/test | P5 | 290 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_slash_worker_ansi` | oracle/test | P5 | 23 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_slash_worker_mcp_discovery` | oracle/test | P5 | 105 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_slash_worker_profile_home` | oracle/test | P5 | 37 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_slash_worker_sys_path` | oracle/test | P5 | 92 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_subagent_child_mirror` | oracle/test | P5 | 240 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_subprocess_encoding` | oracle/test | P5 | 132 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_undo_command` | oracle/test | P5 | 110 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.tui_gateway.test_wait_for_mcp_discovery` | oracle/test | P5 | 78 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.verify.test_environment_and_runner` | oracle/test | oracle | 189 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.verify.test_ledger_and_nudge_integration` | oracle/test | oracle | 232 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.verify.test_recipes` | oracle/test | oracle | 250 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tests.verify.test_verify_cmd` | oracle/test | oracle | 97 | ⬜ missing | Read the upstream test; add the matching Rust parity coverage and evidence. |
| `tools.__init__` | production | P2 | 25 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.ansi_strip` | production | P2 | 79 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.approval` | production | P2 | 4,557 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.async_delegation` | production | P2 | 1,515 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.audio_container` | production | P2 | 97 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.binary_extensions` | production | P2 | 42 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.blueprints` | production | P2 | 324 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.browser_camofox` | production | P2 | 953 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.browser_camofox_state` | production | P2 | 47 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.browser_cdp_tool` | production | P2 | 684 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.browser_dialog_tool` | production | P2 | 148 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.browser_supervisor` | production | P2 | 1,518 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.browser_tool` | production | P2 | 5,098 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.budget_config` | production | P2 | 114 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.checkpoint_manager` | production | P2 | 1,953 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.clarify_gateway` | production | P2 | 459 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.clarify_tool` | production | P2 | 266 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.close_terminal_tool` | production | P2 | 62 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.code_execution_tool` | production | P2 | 2,074 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.__init__` | production | P2 | 45 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.backend` | production | P2 | 249 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.browser_route` | production | P2 | 573 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.cua_backend` | production | P2 | 3,295 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.doctor` | production | P2 | 864 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.permissions` | production | P2 | 198 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.schema` | production | P2 | 353 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.computer_use.tool` | production | P2 | 1,341 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use.vision_routing` | production | P2 | 204 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.computer_use_tool` | production | P2 | 42 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.credential_files` | production | P2 | 530 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `tools.cronjob_tools` | production | P2 | 1,616 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.daemon_pool` | production | P2 | 64 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.debug_helpers` | production | P2 | 105 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.delegate_tool` | production | P2 | 4,342 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.delegation_live_log` | production | P2 | 424 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.delegation_output_schema` | production | P2 | 151 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `tools.desktop_ui` | production | P2 | 40 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.discord_tool` | production | P2 | 1,116 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.env_passthrough` | production | P2 | 223 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.env_probe` | production | P2 | 370 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.environments.__init__` | production | P2 | 14 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.base` | production | P2 | 1,396 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.daytona` | production | P2 | 270 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.docker` | production | P2 | 2,046 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.file_sync` | production | P2 | 484 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.local` | production | P2 | 1,687 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.managed_modal` | production | P2 | 282 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.modal` | production | P2 | 478 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.modal_utils` | production | P2 | 210 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.singularity` | production | P2 | 268 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.ssh` | production | P2 | 426 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.environments.vercel_sandbox` | production | P2 | 662 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.fal_common` | production | P2 | 163 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.feishu_doc_tool` | production | P2 | 138 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.feishu_drive_tool` | production | P2 | 431 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.file_operations` | production | P2 | 2,805 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.file_state` | production | P2 | 332 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.file_tools` | production | P2 | 2,579 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.flux3_video_tool` | production | P2 | 1,249 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.focus_pane_tool` | production | P2 | 64 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.fuzzy_match` | production | P2 | 1,108 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.homeassistant_tool` | production | P2 | 514 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.hook_output_spill` | production | P2 | 232 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.image_generation_tool` | production | P2 | 1,668 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.image_source` | production | P2 | 415 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.interrupt` | production | P2 | 113 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.kanban_tools` | production | P2 | 2,250 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.lazy_deps` | production | P2 | 1,208 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.managed_tool_gateway` | production | P2 | 452 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.mcp_dashboard_oauth` | production | P2 | 145 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.mcp_oauth` | production | P2 | 1,369 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.mcp_oauth_manager` | production | P2 | 785 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.mcp_schema_cache` | production | P2 | 121 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.mcp_stdio_watchdog` | production | P2 | 157 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.mcp_tool` | production | P2 | 7,530 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.memory_tool` | production | P2 | 1,240 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.microsoft_graph_auth` | production | P2 | 245 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.microsoft_graph_client` | production | P2 | 400 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.neutts_synth` | production | P2 | 110 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.open_preview_tool` | production | P2 | 92 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.openrouter_client` | production | P2 | 47 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.osv_check` | production | P2 | 218 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.patch_parser` | production | P2 | 729 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.path_security` | production | P2 | 43 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.process_registry` | production | P2 | 2,937 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.project_tools` | production | P2 | 189 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.react_to_message_tool` | production | P2 | 166 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.read_extract` | production | P2 | 346 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.read_preview_tool` | production | P2 | 94 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.read_terminal_tool` | production | P2 | 89 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.registry` | production | P2 | 956 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.schema_sanitizer` | production | P2 | 687 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.send_message_tool` | production | P2 | 2,116 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.session_search_tool` | production | P2 | 1,161 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.skill_manager_tool` | production | P2 | 1,781 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skill_provenance` | production | P2 | 78 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skill_usage` | production | P2 | 1,340 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skills_ast_audit` | production | P2 | 133 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skills_guard` | production | P2 | 1,161 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skills_hub` | production | P2 | 4,432 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skills_sync` | production | P2 | 1,410 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skills_sync_client` | production | P2 | 2,187 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.skills_tool` | production | P2 | 2,051 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.slash_confirm` | production | P2 | 167 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.terminal_hints` | production | P2 | 170 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.terminal_tool` | production | P2 | 3,580 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.thread_context` | production | P2 | 120 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.threat_patterns` | production | P2 | 284 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `tools.tirith_security` | production | P2 | 872 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.todo_tool` | production | P2 | 335 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `tools.tool_backend_helpers` | production | P2 | 311 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `tools.tool_output_limits` | production | P2 | 110 | 🟡 partial | Close every documented seam, add parity evidence, then promote to done. |
| `tools.tool_result_storage` | production | P2 | 254 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.tool_search` | production | P2 | 1,078 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.transcription_tools` | production | P2 | 3,016 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.tts_streaming` | production | P2 | 488 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.tts_text_normalize` | production | P2 | 278 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.tts_tool` | production | P2 | 3,964 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.url_safety` | production | P2 | 874 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.video_generation_tool` | production | P2 | 575 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.vision_tools` | production | P2 | 2,082 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.voice_mode` | production | P2 | 2,379 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.wake_word` | production | P2 | 1,464 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.web_tools` | production | P2 | 1,237 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.website_policy` | production | P2 | 283 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.working_diff` | production | P2 | 130 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `tools.write_approval` | production | P2 | 493 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.x_search_tool` | production | P2 | 552 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.xai_http` | production | P2 | 329 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.xai_video_tools` | production | P2 | 209 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tools.yuanbao_tools` | production | P2 | 737 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `toolset_distributions` | production | P2 | 358 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `toolsets` | production | P2 | 1,029 | ✅ done | Maintain parity evidence; no remaining task in this row. |
| `trajectory_compressor` | production | P2 | 1,598 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.__init__` | production | P5 | 0 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway._stdin_recovery` | production | P5 | 151 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.compute_host` | production | P5 | 893 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.entry` | production | P5 | 490 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.event_publisher` | production | P5 | 126 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.git_probe` | production | P5 | 191 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.host_supervisor` | production | P5 | 577 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.loop_noise` | production | P5 | 83 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.method_ctx` | production | P5 | 53 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.methods_complete` | production | P5 | 484 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.methods_config` | production | P5 | 422 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.methods_prompt` | production | P5 | 983 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.methods_session` | production | P5 | 3,259 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.methods_tools` | production | P5 | 1,923 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.project_tree` | production | P5 | 768 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.render` | production | P5 | 49 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.server` | production | P5 | 14,150 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.slash_worker` | production | P5 | 196 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.synthetic_turn` | production | P5 | 231 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.transport` | production | P5 | 219 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.turn_marker` | production | P5 | 159 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `tui_gateway.ws` | production | P5 | 476 | ⬜ missing | Read upstream module + tests; write tests first, implement, audit, build, test, ledger, commit. |
| `utils` | production | P1 | 666 | ✅ done | Maintain parity evidence; no remaining task in this row. |

---

Generated by `tools/conversion_ledger.py`; do not hand-edit the generated rows.
