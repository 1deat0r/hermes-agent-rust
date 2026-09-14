#!/usr/bin/env bash
# Regenerate the inventory ledger from the pinned upstream clone.
set -euo pipefail
UPSTREAM="${HERMES_UPSTREAM:-/run/media/its1deat0r/Projects/Research/hermes-agent-repo}"
cd "$(dirname "$0")/.."
python3 tools/inventory.py "$UPSTREAM" --out tools/inventory.json
