#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
hook_dir="${repo_root}/.githooks"

if [[ ! -x "${hook_dir}/pre-commit" || ! -x "${hook_dir}/post-commit" ]]; then
  echo "tracked hooks are missing or not executable under ${hook_dir}" >&2
  exit 1
fi

git -C "$repo_root" config core.hooksPath .githooks
git -C "$repo_root" config hermes.githubRepository "${HERMES_GITHUB_REPOSITORY:-1deat0r/hermes-agent-rust}"
git -C "$repo_root" config hermes.githubBranch "${HERMES_GITHUB_BRANCH:-main}"

if [[ "${1:-}" == "--strict" ]]; then
  git -C "$repo_root" config hermes.githubSyncRequired true
fi

echo "installed tracked hooks from .githooks"
echo "pre-commit refreshes inventory/ledger/README status and requires PLAN.md + HANDOFF.md for source changes"
echo "post-commit synchronizes the GitHub description and verifies README.md"
if [[ "${1:-}" == "--strict" ]]; then
  echo "strict GitHub synchronization is enabled"
else
  echo "GitHub synchronization is best-effort; pass --strict to require credentials and a clean remote check"
fi
