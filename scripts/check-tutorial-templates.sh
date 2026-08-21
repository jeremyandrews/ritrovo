#!/usr/bin/env bash
#
# Diff the vendored tutorial templates against the Trovato release the demo runs.
#
# WHY THE TEMPLATES ARE VENDORED AT ALL
# docs/tutorial/templates holds nine files that belong to Trovato, not to Ritrovo:
# they are its tutorial's own templates, and the tutorial reader gets them from a
# Trovato checkout. The released container image does not ship them — it ships
# templates/, static/ and docs/tutorial/config/, and stops there. A demo that runs
# on the image alone therefore has to get them from somewhere, and copying nine
# small files into this repository is the option that leaves a stranger needing
# nothing but Docker.
#
# The cost of that choice is the copy going stale, so it is checked instead of
# hoped for. This script is that check, and it runs in CI.
#
# Nothing here modifies Trovato. It fetches a published tarball, reads nine files
# out of it, and diffs.
#
# Usage:
#   scripts/check-tutorial-templates.sh [--update]
#
#   --update   overwrite the vendored copies with the release's, then report what
#              changed. Review the diff before committing it: a change here is a
#              change to what the demo renders.

set -euo pipefail

# The release the demo runs against. Bump this and the image tag in
# docker-compose.demo.yml together; the demo_wiring tests fail if they disagree.
RELEASE="v0.101.0"
REPO="https://codeload.github.com/jeremyandrews/trovato/tar.gz/refs/tags"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDORED="$ROOT/docs/tutorial/templates"
UPDATE=0
[ "${1:-}" = "--update" ] && UPDATE=1

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "==> fetching docs/tutorial/templates from Trovato $RELEASE"
curl -fsSL "$REPO/$RELEASE" -o "$work/trovato.tar.gz"

# The tarball's top-level directory is named for the tag with the leading v
# stripped, which is why this untars by pattern rather than by a path it guessed.
# GNU tar needs --wildcards to glob a member name; bsdtar rejects the flag and
# globs anyway. Detect rather than guess: the demo has to work on both.
tar_glob=""
tar --version 2>/dev/null | head -n 1 | grep -q GNU && tar_glob="--wildcards"
# shellcheck disable=SC2086  # tar_glob is a flag or nothing, deliberately unquoted
tar -xzf "$work/trovato.tar.gz" -C "$work" $tar_glob '*/docs/tutorial/templates/*'
upstream="$(find "$work" -type d -path '*/docs/tutorial/templates' | head -n 1)"
if [ -z "$upstream" ]; then
    echo "error: $RELEASE has no docs/tutorial/templates" >&2
    exit 1
fi

if [ "$UPDATE" = "1" ]; then
    echo "==> updating the vendored copies"
    rm -rf "$VENDORED"
    cp -R "$upstream" "$VENDORED"
    git -C "$ROOT" --no-pager diff --stat -- docs/tutorial/templates
    exit 0
fi

if diff -ru "$upstream" "$VENDORED"; then
    count="$(find "$VENDORED" -type f | wc -l | tr -d ' ')"
    echo "==> $count vendored template(s) are identical to Trovato $RELEASE"
    exit 0
fi

cat >&2 <<EOF

The vendored tutorial templates have drifted from Trovato $RELEASE.

That is not automatically wrong — the release may have changed them — but the demo
renders these files, so somebody has to decide. To take the release's version:

    scripts/check-tutorial-templates.sh --update

EOF
exit 1
