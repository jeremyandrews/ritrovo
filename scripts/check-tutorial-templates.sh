#!/usr/bin/env bash
#
# Diff the vendored tutorial templates against the Trovato release the demo runs,
# and report where the kernel's copy of the configuration set has drifted from
# Ritrovo's.
#
# THE CONFIGURATION SET RUNS THE OTHER WAY NOW
# demo/config used to be a couple of files the tutorial set did not carry, and
# tests/host/tutorial-config was a subset of the kernel's copy that this script
# held to the release byte for byte. Both are gone: the whole set lives in
# demo/config and Ritrovo's copy is the source of truth.
#
# So the comparison inverts. The kernel still ships docs/tutorial/config in its
# image and its tutorial still imports it, and while both copies exist they can
# disagree — which is the point, because Ritrovo's is where the brief's model is
# being built. This script therefore reports that difference and does NOT fail on
# it: a warning, listing the files, so nobody has to find out from a demo that
# renders something else. It goes back to being an error, or disappears, when the
# kernel stops shipping its copy.
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
# The cost of that choice is a copy going stale, so it is checked instead of
# hoped for. This script is that check, and it runs in CI.
#
# Nothing here modifies Trovato. It fetches a published tarball, reads nine files
# out of it, and diffs.
#
# Usage:
#   scripts/check-tutorial-templates.sh [--update]
#
#   --update   overwrite the vendored TEMPLATES with the release's, then report
#              what changed. Review the diff before committing it: a change here
#              is a change to what the demo renders. It does not touch
#              demo/config: that set is Ritrovo's, and taking the kernel's
#              version of it would undo the model this repository defines.

set -euo pipefail

# The release the demo runs against. GENERATED: this line is written from
# kernel-release.toml by scripts/sync-kernel-release.sh, and the demo_wiring tests
# fail if it disagrees. Move the pin there, not here.
RELEASE="v0.103.0"
REPO="https://codeload.github.com/jeremyandrews/trovato/tar.gz/refs/tags"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDORED="$ROOT/docs/tutorial/templates"
DEMO_CONFIG="$ROOT/demo/config"
UPDATE=0
[ "${1:-}" = "--update" ] && UPDATE=1

# Templates this repository has taken OWNERSHIP of, by path under
# docs/tutorial/templates. The release is no longer their source, so a difference
# from it is not drift and this script neither reports it nor overwrites it with
# --update. Everything not listed here is still a vendored copy and is still
# checked byte for byte.
#
# Keep this list short and say why each entry is on it. An entry is a decision to
# stop taking the kernel's version of a file, which is the opposite of what the
# rest of this script is for.
#
#   elements/item--conference.html
#   elements/item--speaker.html
#       Rewritten to render their own fields in their own places. The release's
#       copies render `children`, which is the kernel's generic `label: value`
#       dump of every scalar field, so the pages printed every field twice and
#       leaked internal ones. They also follow the model this repository now
#       defines: multi-value venue photos, a schedule PDF, speakers as a forward
#       reference from the conference, and a headshot rather than a photo.
#
#   gather/includes/conf-card.html
#   gather/query--ritrovo.all_speakers.html
#       Follow the same model change: the logo and language on a card, the
#       headshot on a speaker, and no `field_company`, which the type no longer
#       declares.
#
#   gather/query--ritrovo.by_country.html
#   gather/query--ritrovo.by_city.html
#   gather/query--ritrovo.conferences_this_month.html
#   gather/query--ritrovo.cfps_closing_soon.html
#       New here and absent from the release. The first two replace the kernel's
#       generic gather table, which printed every column of every row including
#       `search_vector`, on the location pages a visitor reaches from a
#       conference. The last two render the brief's tile gathers through their own
#       routes, because the tiles themselves render nothing on this kernel.
OWNED_TEMPLATES="
elements/item--conference.html
elements/item--speaker.html
gather/includes/conf-card.html
gather/query--ritrovo.all_speakers.html
gather/query--ritrovo.by_country.html
gather/query--ritrovo.by_city.html
gather/query--ritrovo.conferences_this_month.html
gather/query--ritrovo.cfps_closing_soon.html
"

is_owned() {
    printf '%s\n' "$OWNED_TEMPLATES" | grep -qx -- "$1"
}

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
    for file in $(cd "$upstream" && find . -type f | sed 's|^\./||'); do
        if is_owned "$file"; then
            echo "    skipping $file: Ritrovo owns it"
            continue
        fi
        mkdir -p "$(dirname "$VENDORED/$file")"
        cp "$upstream/$file" "$VENDORED/$file"
    done
    echo "    demo/config is Ritrovo's own and is never overwritten from the release"
    git -C "$ROOT" --no-pager diff --stat -- docs/tutorial/templates
    exit 0
fi

drift=0
owned_seen=0
# File by file rather than `diff -ru` over the two trees, so that a template this
# repository has taken ownership of can be skipped by path without also skipping a
# same-named file somewhere else in the tree.
for file in $(cd "$upstream" && find . -type f | sed 's|^\./||'); do
    if is_owned "$file"; then
        owned_seen=$((owned_seen + 1))
        continue
    fi
    if [ ! -f "$VENDORED/$file" ]; then
        echo "docs/tutorial/templates/$file is in Trovato $RELEASE and not in this repository"
        drift=1
    elif ! diff -u "$upstream/$file" "$VENDORED/$file"; then
        drift=1
    fi
done
for file in $(cd "$VENDORED" && find . -type f | sed 's|^\./||'); do
    # An owned file is allowed to exist here and nowhere in the release: four of
    # them are new templates this repository wrote.
    is_owned "$file" && continue
    if [ ! -f "$upstream/$file" ]; then
        echo "docs/tutorial/templates/$file is not in Trovato $RELEASE"
        drift=1
    fi
done
# The configuration set, compared the other way round and WITHOUT failing.
#
# Ritrovo's copy is the source of truth. The kernel still ships its own and its
# tutorial still imports it, so the two can disagree, and while the brief's model
# is being built here they are supposed to. What nobody should have to discover
# from a demo rendering the wrong thing is WHICH files disagree, so they are
# listed. This becomes an error, or goes away, when the kernel stops shipping a
# copy — see docs/ritrovo/STATUS.md, "Where the content model lives".
config_drift=0
config_only_kernel=0
for file in "$upstream_config"/*.yml; do
    name="$(basename "$file")"
    if [ ! -f "$DEMO_CONFIG/$name" ]; then
        config_only_kernel=$((config_only_kernel + 1))
    elif ! diff -q "$file" "$DEMO_CONFIG/$name" >/dev/null; then
        config_drift=$((config_drift + 1))
    fi
done
config_only_ritrovo=0
for file in "$DEMO_CONFIG"/*.yml; do
    name="$(basename "$file")"
    [ -f "$upstream_config/$name" ] || config_only_ritrovo=$((config_only_ritrovo + 1))
done

report_config_drift() {
    echo "==> the configuration set: Ritrovo's copy is the source of truth"
    echo "    $(find "$DEMO_CONFIG" -maxdepth 1 -name '*.yml' | wc -l | tr -d ' ') file(s) in demo/config"
    if [ "$config_drift" = "0" ] && [ "$config_only_kernel" = "0" ] && [ "$config_only_ritrovo" = "0" ]; then
        echo "    the kernel's copy in Trovato $RELEASE is identical"
        return
    fi
    echo "    the kernel's copy in Trovato $RELEASE has drifted, which is expected"
    echo "    while the brief's model is built here and the kernel still ships its own:"
    [ "$config_drift" = "0" ] || echo "      $config_drift file(s) differ"
    [ "$config_only_kernel" = "0" ] || echo "      $config_only_kernel file(s) only the kernel has"
    [ "$config_only_ritrovo" = "0" ] || echo "      $config_only_ritrovo file(s) only Ritrovo has"
    echo "    not a failure. See docs/ritrovo/STATUS.md, \"Where the content model lives\"."
}

if [ "$drift" = "0" ]; then
    templates="$(find "$VENDORED" -type f | wc -l | tr -d ' ')"
    echo "==> $((templates - owned_seen)) vendored template(s) are identical to Trovato $RELEASE"
    if [ "$owned_seen" -gt 0 ]; then
        echo "==> $owned_seen template(s) are Ritrovo's own and were not compared:"
        printf '%s\n' "$OWNED_TEMPLATES" | grep -v '^$' | sed 's/^/      /'
    fi
    report_config_drift
    exit 0
fi

report_config_drift

cat >&2 <<EOF

The vendored tutorial TEMPLATES have drifted from Trovato $RELEASE.

That is not automatically wrong — the release may have changed them — but the demo
renders them, so somebody has to decide.
To take the release's version:

    scripts/check-tutorial-templates.sh --update

EOF
exit 1
