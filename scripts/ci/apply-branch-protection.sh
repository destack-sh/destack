#!/usr/bin/env bash
set -euo pipefail

print_usage() {
    cat <<'USAGE'
usage: apply-branch-protection.sh [--repo owner/name] [--branch name] [--dry-run]

options:
  --repo      github repository in owner/name form
  --branch    branch name, defaults to main
  --dry-run   print payload and exit
  -h, --help  show this help
USAGE
}

script_directory="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
required_checks_file="${script_directory}/required-checks.txt"
repository="${DESTACK_GITHUB_REPO:-}"
branch="${DESTACK_GITHUB_BRANCH:-main}"
is_dry_run="false"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --repo)
            if [ "$#" -lt 2 ]; then
                echo "missing value for --repo" >&2
                exit 1
            fi
            repository="$2"
            shift
            ;;
        --branch)
            if [ "$#" -lt 2 ]; then
                echo "missing value for --branch" >&2
                exit 1
            fi
            branch="$2"
            shift
            ;;
        --dry-run)
            is_dry_run="true"
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "unsupported argument: $1" >&2
            print_usage >&2
            exit 1
            ;;
    esac
    shift
done

if ! command -v gh >/dev/null 2>&1; then
    echo "missing gh cli: install github cli first" >&2
    exit 1
fi

if ! gh auth status -h github.com >/dev/null 2>&1; then
    echo "missing gh auth: run gh auth login first" >&2
    exit 1
fi

if [ -z "${repository}" ]; then
    repository="$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || true)"
fi

if [ -z "${repository}" ]; then
    echo "missing repository: pass --repo owner/name or set DESTACK_GITHUB_REPO" >&2
    exit 1
fi

if [ ! -f "${required_checks_file}" ]; then
    echo "missing required checks file: ${required_checks_file}" >&2
    exit 1
fi

checks_json_entries=""
while IFS= read -r check_name; do
    if [ -z "${check_name}" ]; then
        continue
    fi

    # escape context strings for json payload values
    escaped_check_name="$(printf '%s' "${check_name}" | sed 's/\\/\\\\/g; s/"/\\"/g')"
    checks_json_entries="${checks_json_entries}{\"context\":\"${escaped_check_name}\"},"
done < "${required_checks_file}"

checks_json="[${checks_json_entries%,}]"

payload="$(cat <<'JSON'
{
  "required_status_checks": {
    "strict": true,
    "checks": __REQUIRED_CHECKS_JSON__
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1,
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "require_last_push_approval": false
  },
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": true,
  "lock_branch": false,
  "allow_fork_syncing": true
}
JSON
)"

payload="${payload/__REQUIRED_CHECKS_JSON__/${checks_json}}"

if [ "${is_dry_run}" = "true" ]; then
    echo "target repository: ${repository}"
    echo "target branch: ${branch}"
    echo ""
    echo "${payload}"
    exit 0
fi

gh api \
    --method PUT \
    -H "Accept: application/vnd.github+json" \
    "repos/${repository}/branches/${branch}/protection" \
    --input - <<<"${payload}"

echo "applied branch protection to ${repository}:${branch}"
