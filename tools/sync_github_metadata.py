#!/usr/bin/env python3
"""Synchronize the repository description and verify the GitHub README.

The README is a normal repository file, so the exact local/remote commit
mirror remains responsible for publishing it. This tool verifies that the
remote default branch has the same bytes and can optionally update it through
the Contents API when explicitly requested. The repository description is
metadata, so it can be patched without manufacturing an unrelated commit.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import subprocess
import sys
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REPOSITORY = "1deat0r/hermes-agent-rust"
API_ROOT = "https://api.github.com"


class GitHubRequestError(RuntimeError):
    pass


def truthy(value: str | None) -> bool:
    return (value or "").strip().lower() in {"1", "true", "yes", "on"}


def git_config(key: str) -> str | None:
    result = subprocess.run(
        ["git", "config", "--get", key],
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
    )
    value = result.stdout.strip()
    return value or None


def repository_name() -> str:
    configured = os.environ.get("HERMES_GITHUB_REPOSITORY") or git_config(
        "hermes.githubRepository"
    )
    if configured:
        return configured.strip().removesuffix(".git")

    remote = git_config("remote.origin.url") or ""
    if remote.startswith("git@github.com:"):
        return remote.removeprefix("git@github.com:").removesuffix(".git")
    marker = "github.com/"
    if marker in remote:
        return remote.split(marker, 1)[1].removesuffix(".git")
    return DEFAULT_REPOSITORY


def branch_name() -> str:
    configured = os.environ.get("HERMES_GITHUB_BRANCH") or git_config(
        "hermes.githubBranch"
    )
    if configured:
        return configured.strip()
    result = subprocess.run(
        ["git", "branch", "--show-current"],
        cwd=ROOT,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
    )
    return result.stdout.strip() or "main"


def token() -> str | None:
    for name in ("HERMES_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"):
        value = os.environ.get(name)
        if value:
            return value
    return None


def api_request(
    access_token: str,
    method: str,
    path: str,
    payload: dict[str, object] | None = None,
) -> dict[str, object]:
    body = None
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {access_token}",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "hermes-agent-rust-metadata-sync",
    }
    if payload is not None:
        body = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    request = Request(API_ROOT + path, data=body, headers=headers, method=method)
    try:
        with urlopen(request, timeout=20) as response:
            raw = response.read()
    except (HTTPError, URLError) as error:
        detail = ""
        if isinstance(error, HTTPError):
            detail = error.read().decode("utf-8", errors="replace")[:500]
        raise GitHubRequestError(
            f"GitHub {method} {path} failed: {detail or error}"
        ) from error
    if not raw:
        return {}
    try:
        value = json.loads(raw.decode("utf-8"))
    except json.JSONDecodeError as error:
        raise GitHubRequestError(
            f"GitHub returned invalid JSON for {method} {path}"
        ) from error
    if not isinstance(value, dict):
        raise GitHubRequestError(
            f"GitHub returned an unexpected response for {method} {path}"
        )
    return value


def read_description() -> str:
    path = ROOT / ".github" / "repository-description.txt"
    description = path.read_text(encoding="utf-8").strip()
    if not description or "\n" in description or len(description) > 350:
        raise RuntimeError(
            ".github/repository-description.txt must contain one non-empty line "
            "of at most 350 characters"
        )
    return description


def readme_bytes() -> bytes:
    return (ROOT / "README.md").read_bytes()


def remote_readme(
    access_token: str, repository: str, branch: str
) -> tuple[bytes, str]:
    encoded_repo = quote(repository, safe="/")
    encoded_branch = quote(branch, safe="")
    value = api_request(
        access_token,
        "GET",
        f"/repos/{encoded_repo}/contents/README.md?ref={encoded_branch}",
    )
    content = value.get("content")
    sha = value.get("sha")
    if not isinstance(content, str) or not isinstance(sha, str):
        raise GitHubRequestError(
            "GitHub README response did not contain content and sha"
        )
    try:
        decoded = base64.b64decode("".join(content.split()), validate=True)
    except ValueError as error:
        raise GitHubRequestError("GitHub README content was not valid base64") from error
    return decoded, sha


def sync(
    *,
    access_token: str,
    repository: str,
    branch: str,
    description_mode: str,
    readme_mode: str,
    commit_message: str,
) -> int:
    if "/" not in repository or repository.startswith("/") or repository.endswith("/"):
        raise RuntimeError(f"invalid GitHub repository name: {repository!r}")
    encoded_repo = quote(repository, safe="/")
    local_description = read_description()
    remote_repo = api_request(access_token, "GET", f"/repos/{encoded_repo}")
    remote_description = remote_repo.get("description")
    if remote_description != local_description:
        if description_mode == "sync":
            api_request(
                access_token,
                "PATCH",
                f"/repos/{encoded_repo}",
                {"description": local_description},
            )
            print("GitHub repository description synchronized")
        elif description_mode == "verify":
            raise RuntimeError(
                "GitHub repository description differs from "
                ".github/repository-description.txt"
            )

    if readme_mode == "skip":
        print("GitHub README check skipped")
        return 0

    local_readme = readme_bytes()
    try:
        remote_bytes, remote_sha = remote_readme(access_token, repository, branch)
    except GitHubRequestError as error:
        detail = str(error)
        if "404" in detail:
            raise RuntimeError(f"GitHub README.md is missing on {repository}:{branch}") from error
        raise

    if remote_bytes == local_readme:
        print("GitHub README is synchronized")
        return 0

    if readme_mode == "verify":
        raise RuntimeError(
            "GitHub README.md differs from the checked-out README.md; push or mirror "
            "the local commit before retrying"
        )
    if readme_mode != "sync":
        raise RuntimeError(f"unknown README mode: {readme_mode}")

    payload = {
        "message": commit_message,
        "content": base64.b64encode(local_readme).decode("ascii"),
        "sha": remote_sha,
        "branch": branch,
    }
    api_request(
        access_token,
        "PUT",
        f"/repos/{encoded_repo}/contents/README.md",
        payload,
    )
    print("GitHub README synchronized through the Contents API")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--description-mode",
        choices=("verify", "sync", "skip"),
        default=os.environ.get("HERMES_GITHUB_DESCRIPTION_MODE", "sync"),
    )
    parser.add_argument(
        "--readme-mode",
        choices=("verify", "sync", "skip"),
        default=os.environ.get(
            "HERMES_GITHUB_README_MODE",
            git_config("hermes.githubReadmeMode") or "verify",
        ),
    )
    parser.add_argument(
        "--message",
        default="docs: synchronize GitHub README",
        help="commit message used only when --readme-mode=sync updates README.md",
    )
    args = parser.parse_args()

    access_token = token()
    required = truthy(os.environ.get("HERMES_GITHUB_SYNC_REQUIRED")) or truthy(
        git_config("hermes.githubSyncRequired")
    )
    if not access_token:
        message = (
            "GitHub metadata sync skipped: set HERMES_GITHUB_TOKEN, GH_TOKEN, or "
            "GITHUB_TOKEN"
        )
        if required:
            print(f"post-commit: {message}", file=sys.stderr)
            return 2
        print(f"post-commit: {message}")
        return 0

    try:
        return sync(
            access_token=access_token,
            repository=repository_name(),
            branch=branch_name(),
            description_mode=args.description_mode,
            readme_mode=args.readme_mode,
            commit_message=args.message,
        )
    except (GitHubRequestError, OSError, RuntimeError) as error:
        print(f"post-commit: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
