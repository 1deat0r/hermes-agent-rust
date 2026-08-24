# Gates: complete hermes-agent to hermes-agent-rust conversion

OWNS: crates/**, tools/**, scripts/**, examples/**, upstream/**, PLAN.md, HANDOFF.md, README.md, CONVERSION-LEDGER.md

Scope: finish and verify the full pinned `hermes-agent` @ `b9aa928` conversion, including implementation, parity evidence, integration, documentation, and exact local/GitHub publication.

## Depth tree

The conversion is decomposed into these dependency-ordered branches:

1. Foundation: constants, time, utilities, logging, and state.
2. Agent core: toolsets, model tools, `run_agent`, `agent/`, `tools/`, and batch.
3. CLI: `cli.py`, `hermes_cli/`, configuration, auth, and entry points.
4. Integrations: providers, plugins/platforms, gateway, and cron.
5. Surfaces: TUI, ACP, scripts, bundled skills, and remaining top-level modules.
6. Root integration: full-workspace tests, live/mock contract checks, docs/ledger closure, and local/GitHub mirror verification.

The active leaves are Agent core → config discovery and Providers → Z.AI
endpoint chooser. Both current units are transport-neutral explicit-input
boundaries; their next dependency-safe work is full merged config loading and
concrete Z.AI HTTP/cache integration. Later leaves must not be marked verified
while a lower-layer contract remains partial.

- [ ] G1: every tracked upstream module is marked done in the generated inventory
  CHECK: /usr/bin/python3 -c "import json; s=json.load(open('tools/inventory.json', encoding='utf-8'))['summary']; assert s['modules'] == 3882 and s['status_counts'] == {'done': 3882}, s; assert s['production_modules'] == 1103 and s['prod_status_counts'] == {'done': 1103}, s; print('inventory closure passed')"
  EXPECT: inventory closure passed
  EVIDENCE: pending

- [ ] G2: the complete workspace builds and all active Rust tests pass serially
  CHECK: /home/mustbearnold/.cargo/bin/cargo test --workspace -- --test-threads=1
  EXPECT: test result: ok
  EVIDENCE: pending

- [ ] G3: the final workspace is formatted and documentation hooks remain valid
  CHECK: bash -n .githooks/pre-commit .githooks/post-commit tools/install_hooks.sh && PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -c "from pathlib import Path; [compile(p.read_text(encoding='utf-8'), str(p), 'exec') for p in (Path('tools/pre_commit_docs.py'), Path('tools/refresh_docs.py'), Path('tools/sync_github_metadata.py')]; print('documentation hooks passed')" && /home/mustbearnold/.cargo/bin/cargo fmt --all -- --check
  EXPECT: documentation hooks passed
  EVIDENCE: pending

- [ ] G4: the generated ledger and README are refreshed from the pinned upstream checkout and contain no stale completion snapshot
  CHECK: HERMES_UPSTREAM=/run/media/mustbearnold/Projects/Research/hermes-agent-repo /usr/bin/python3 tools/refresh_docs.py --upstream /run/media/mustbearnold/Projects/Research/hermes-agent-repo && git diff --exit-code -- tools/inventory.json CONVERSION-LEDGER.md README.md && echo documentation snapshot passed
  EXPECT: documentation snapshot passed
  EVIDENCE: pending

- [ ] G5: every implemented module has source-derived parity tests or an explicit reviewed live/mock evidence record
  EVIDENCE: pending

- [ ] G6: the final local `main` commit and GitHub `main` commit have identical recursive trees, modes, and blob SHAs, and every logical commit was mirrored immediately
  EVIDENCE: pending

- [ ] G7: end-to-end CLI, provider, tool, state, gateway, TUI, ACP, and live-platform contracts have been reviewed against the pinned upstream behavior
  EVIDENCE: pending
