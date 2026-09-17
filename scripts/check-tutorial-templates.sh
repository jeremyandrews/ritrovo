#!/usr/bin/env bash
#
# Diff the vendored tutorial templates, and the tutorial config the tests import,
# against the Trovato release the demo runs.
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
# tests/host/tutorial-config holds a second, smaller copy for the same reason: the
# host-in-the-loop suites import the slice of docs/tutorial/config they depend on
# (the topics taxonomy, the conference type, the editorial stages), and a test
# database has no kernel image to get it from. It is a subset, so it is checked
# file by file against the release rather than as a whole tree.
#
# The cost of both choices is a copy going stale, so they are checked instead of
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
#              change to what the demo renders, or to what the tests import.

set -euo pipefail

# The release the demo runs against. Bump this and the image tag in
# docker-compose.demo.yml together; the demo_wiring tests fail if they disagree.
RELEASE="v0.102.0"
REPO="https://codeload.github.com/jeremyandrews/trovato/tar.gz/refs/tags"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDORED="$ROOT/docs/tutorial/templates"
CONFIG_SUBSET="$ROOT/tests/host/tutorial-config"
UPDATE=0
[ "${1:-}" = "--update" ] && UPDATE=1

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "==> fetching docs/tutorial/{templates,config} from Trovato $RELEASE"
curl -fsSL "$REPO/$RELEASE" -o "$work/trovato.tar.gz"

# The tarball's top-level directory is named for the tag with the leading v
# stripped, which is why this untars by pattern rather than by a path it guessed.
# GNU tar needs --wildcards to glob a member name; bsdtar rejects the flag and
# globs anyway. Detect rather than guess: the demo has to work on both.
tar_glob=""
tar --version 2>/dev/null | head -n 1 | grep -q GNU && tar_glob="--wildcards"
# shellcheck disable=SC2086  # tar_glob is a flag or nothing, deliberately unquoted
tar -xzf "$work/trovato.tar.gz" -C "$work" $tar_glob \
    '*/docs/tutorial/templates/*' '*/docs/tutorial/config/*'
upstream="$(find "$work" -type d -path '*/docs/tutorial/templates' | head -n 1)"
upstream_config="$(find "$work" -type d -path '*/docs/tutorial/config' | head -n 1)"
if [ -z "$upstream" ] || [ -z "$upstream_config" ]; then
    echo "error: $RELEASE has no docs/tutorial/templates or docs/tutorial/config" >&2
    exit 1
fi

if [ "$UPDATE" = "1" ]; then
    echo "==> updating the vendored copies"
    rm -rf "$VENDORED"
    cp -R "$upstream" "$VENDORED"
    for file in "$CONFIG_SUBSET"/*.yml; do
        name="$(basename "$file")"
        if [ -f "$upstream_config/$name" ]; then
            cp "$upstream_config/$name" "$file"
        else
            echo "    $name is no longer in the release; remove it or replace it" >&2
        fi
    done
    git -C "$ROOT" --no-pager diff --stat -- docs/tutorial/templates tests/host/tutorial-config
    exit 0
fi

drift=0
if ! diff -ru "$upstream" "$VENDORED"; then
    drift=1
fi
for file in "$CONFIG_SUBSET"/*.yml; do
    name="$(basename "$file")"
    if [ ! -f "$upstream_config/$name" ]; then
        echo "tests/host/tutorial-config/$name is not in Trovato $RELEASE"
        drift=1
    elif ! diff -u "$upstream_config/$name" "$file"; then
        drift=1
    fi
done

if [ "$drift" = "0" ]; then
    templates="$(find "$VENDORED" -type f | wc -l | tr -d ' ')"
    config="$(find "$CONFIG_SUBSET" -name '*.yml' | wc -l | tr -d ' ')"
    echo "==> $templates vendored template(s) and $config tutorial config file(s) are identical to Trovato $RELEASE"
    exit 0
fi

cat >&2 <<EOF

The vendored tutorial files have drifted from Trovato $RELEASE.

That is not automatically wrong — the release may have changed them — but the demo
renders the templates and the tests import the config, so somebody has to decide.
To take the release's version:

    scripts/check-tutorial-templates.sh --update

EOF
exit 1
