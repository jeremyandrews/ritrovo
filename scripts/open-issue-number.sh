#!/usr/bin/env bash
#
# Print the number of the open issue titled exactly TITLE, or nothing.
#
# Usage: scripts/open-issue-number.sh TITLE      (GH_TOKEN and GH_REPO set)
#
# The kernel nightly (.github/workflows/kernel-nightly.yml) keeps one issue,
# "Kernel main breaks Ritrovo", and finds it with this before it comments on it,
# closes it, or opens it. Every open issue is read, page by page, however many
# there are: a lookup that read only the first page would miss the issue on a
# repository with more open issues than that, and open a second one every
# night. Pull requests come back from the issues endpoint too, and are skipped.
#
# Covered by demo/checks/tests/open_issue_number.rs, against a stub gh.

set -euo pipefail

[ $# -eq 1 ] && [ -n "$1" ] || { echo "usage: $0 TITLE" >&2; exit 2; }
: "${GH_REPO:?GH_REPO must name the repository, owner/name}"

# --paginate prints each page's array in turn; jq reads them as a stream and
# stops at the first match, so nothing downstream closes the pipe early.
gh api --paginate "repos/$GH_REPO/issues?state=open&per_page=100" \
    | jq -rn --arg title "$1" \
        'first(inputs[] | select(.pull_request == null and .title == $title) | .number) // empty'
