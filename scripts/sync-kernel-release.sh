#!/usr/bin/env bash
#
# Write the pinned Trovato release into every place this repository repeats it.
#
# kernel-release.toml holds two authored fields, `version` and `rev`, and is the
# only place either is written by hand. Everything else derives from them, and
# this script is the derivation:
#
#   Cargo.toml                          the rev on trovato-sdk and trovato-kernel,
#                                       and the recorded pin block
#   plugins/*/*.info.toml               api_version, the major.minor of version
#   docker-compose.demo.yml             the kernel image tag, on both services
#   scripts/check-tutorial-templates.sh RELEASE, the git tag
#   README.md, docs/INSTALL.md          the kernel-release marker blocks
#
# demo/checks/tests/demo_wiring.rs checks every one of them against
# kernel-release.toml, so drift fails `cargo test --workspace`, which CI already
# runs. This script and that test are the two halves: one writes, one guards.
#
# Usage:
#   scripts/sync-kernel-release.sh                     rewrite from kernel-release.toml
#   scripts/sync-kernel-release.sh --set-version X.Y.Z resolve tag vX.Y.Z against the
#                                                      Trovato repository, write both
#                                                      fields, then rewrite
#
# The version is never taken from the command line on trust: --set-version asks
# github for the commit the tag points at and refuses if there is no such tag.
# Nothing outside the locations listed above is ever touched.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACT="$ROOT/kernel-release.toml"
TROVATO_REPO="https://github.com/jeremyandrews/trovato.git"
IMAGE_REPO="ghcr.io/jeremyandrews/trovato"

usage() {
    cat <<'EOF'
Write the pinned Trovato release into every place this repository repeats it.

  scripts/sync-kernel-release.sh                     rewrite from kernel-release.toml
  scripts/sync-kernel-release.sh --set-version X.Y.Z resolve tag vX.Y.Z against the
                                                     Trovato repository, write both
                                                     fields, then rewrite

kernel-release.toml holds the two authored fields. The header of this script
lists every location the rewrite owns, and demo/checks/tests/demo_wiring.rs
checks all of them, so drift fails `cargo test --workspace`.
EOF
}

die() {
    echo "error: $*" >&2
    exit 1
}

# Read one key out of kernel-release.toml.
#
# A regex, and deliberately: a general TOML reader is a dependency this
# repository does not otherwise have, and the file being read is written by this
# same script and holds exactly two keys, both always double-quoted on their own
# line. The match is anchored to the line start and to the double quotes, so a
# value quoted any other way does not match at all and the caller fails loudly
# rather than reading half of it. Do not reuse this on a TOML file the repository
# does not itself author: Cargo.toml is edited below by substitution on a known
# line, never parsed by this.
contract_get() {
    local key="$1" value
    value="$(sed -n "s/^${key} *= *\"\\([^\"]*\\)\" *\$/\\1/p" "$CONTRACT" | head -n 1)"
    [ -n "$value" ] || die "kernel-release.toml has no $key = \"...\" line"
    printf '%s' "$value"
}

# Replace a file's contents only if they changed, and say so.
#
# Every rewrite below builds the new file in $work and comes through here, so the
# script is idempotent, reports one line per file it actually moved, and never
# rewrites a mtime for nothing.
install_if_changed() {
    local new="$1" target="$2"
    if cmp -s "$new" "$target"; then
        return 0
    fi
    cat "$new" > "$target"
    echo "    ${target#"$ROOT"/}"
}

# sed on one file, anchored, with the result installed through install_if_changed.
#
# The expression must match at least one line: a rewrite that silently matches
# nothing is how a generator quietly stops owning a location, which is the exact
# failure this whole change exists to remove.
sub() {
    local target="$1" match="$2" expression="$3"
    [ -f "$target" ] || die "no such file: $target"
    grep -q -- "$match" "$target" \
        || die "${target#"$ROOT"/} has no line matching '$match'"
    sed "$expression" "$target" > "$work/out"
    install_if_changed "$work/out" "$target"
}

# Replace the body of a named marker block, keeping the markers themselves.
#
# A block opens with a line whose trimmed text ends in `kernel-release:begin
# <name>` and closes with the matching `:end <name>`, in a comment syntax the
# host file already uses: `<!-- ... -->` in markdown, `#` in TOML. Everything
# between the two lines is this script's to write; everything outside is prose
# and is left alone.
sub_block() {
    local target="$1" name="$2" body="$3"
    [ -f "$target" ] || die "no such file: $target"
    awk -v name="$name" -v body="$body" '
        index($0, "kernel-release:begin " name) { print; inside = 1; seen = 1
            while ((getline line < body) > 0) print line
            close(body)
            next }
        index($0, "kernel-release:end " name) { inside = 0 }
        !inside { print }
        END { if (!seen) exit 3 }
    ' "$target" > "$work/out" \
        || die "${target#"$ROOT"/} has no kernel-release:$name block"
    install_if_changed "$work/out" "$target"
}

case "${1:-}" in
    -h | --help)
        usage
        exit 0
        ;;
    --set-version)
        [ -n "${2:-}" ] || die "--set-version needs a version, for example 0.103.0"
        set_version="$2"
        ;;
    "") ;;
    *)
        die "unknown argument: $1 (try --help)"
        ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [ -n "${set_version:-}" ]; then
    tag="v$set_version"
    echo "==> resolving $tag in $TROVATO_REPO"
    # The dereferenced ref, `^{}`, is the COMMIT an annotated tag points at. The
    # plain ref is the tag object itself, which is not a tree cargo can check out
    # and is not what the image was built from. A lightweight tag has no `^{}`
    # line, so fall back to the plain ref and keep refusing when neither exists.
    refs="$(git ls-remote "$TROVATO_REPO" "refs/tags/$tag" "refs/tags/$tag^{}")" \
        || die "cannot reach $TROVATO_REPO"
    resolved="$(printf '%s\n' "$refs" | awk -v t="refs/tags/$tag^{}" '$2 == t { print $1 }')"
    if [ -z "$resolved" ]; then
        resolved="$(printf '%s\n' "$refs" | awk -v t="refs/tags/$tag" '$2 == t { print $1 }')"
    fi
    [ -n "$resolved" ] || die "$TROVATO_REPO has no tag $tag"
    echo "    $tag is $resolved"
    sub "$CONTRACT" '^version = ' "s|^version = \".*\"\$|version = \"$set_version\"|"
    sub "$CONTRACT" '^rev = ' "s|^rev = \".*\"\$|rev = \"$resolved\"|"
fi

version="$(contract_get version)"
rev="$(contract_get rev)"

case "$version" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *) die "version \"$version\" is not X.Y.Z" ;;
esac
case "$rev" in
    [0-9a-f][0-9a-f]*) ;;
    *) die "rev \"$rev\" is not a hex commit id" ;;
esac

tag="v$version"
major="${version%%.*}"
minor="${version#*.}"
minor="${minor%%.*}"
api="$major.$minor"
api_pair="($major, $minor)"
image="$IMAGE_REPO:$version"

echo "==> Trovato $version"
echo "    tag                 $tag"
echo "    rev                 $rev"
echo "    api_version         $api"
echo "    KERNEL_API_VERSION  $api_pair"
echo "    demo image          $image"
echo "==> rewriting"

# Cargo.toml: both git dependencies carry the literal sha, because cargo cannot
# read a `rev` from anywhere else. They must be the same sha, which
# demo/checks/src/lib.rs asserts on its own: a plugin compiled against one
# contract and tested on another kernel proves nothing about what ships.
sub "$ROOT/Cargo.toml" '^trovato-sdk = ' \
    "s|^\\(trovato-sdk = .*rev = \"\\)[0-9a-f]*\\(\".*\\)\$|\\1$rev\\2|"
sub "$ROOT/Cargo.toml" '^trovato-kernel = ' \
    "s|^\\(trovato-kernel = .*rev = \"\\)[0-9a-f]*\\(\".*\\)\$|\\1$rev\\2|"

cat > "$work/cargo-pin" <<EOF
#   tag                 $tag
#   rev                 $rev
#   Trovato version     $version
#   KERNEL_API_VERSION  $api_pair
#
# The rev is the commit the $tag tag points at: the tree the
# $image image the demo runs was published from.
# The plugin manifests declare \`api_version = "$api"\` to match.
EOF
sub_block "$ROOT/Cargo.toml" pin "$work/cargo-pin"

for manifest in "$ROOT"/plugins/*/*.info.toml; do
    sub "$manifest" '^api_version = ' "s|^api_version = \".*\"\$|api_version = \"$api\"|"
done

sub "$ROOT/docker-compose.demo.yml" "image: $IMAGE_REPO:" \
    "s|image: $IMAGE_REPO:.*\$|image: $image|"

sub "$ROOT/scripts/check-tutorial-templates.sh" '^RELEASE=' \
    "s|^RELEASE=\".*\"\$|RELEASE=\"$tag\"|"

cat > "$work/readme-pin" <<EOF
| | |
|---|---|
| tag | \`$tag\` |
| \`rev\` | \`$rev\` |
| Trovato version | $version |
| \`KERNEL_API_VERSION\` | ($major, $minor) |

The \`rev\` is the commit the \`$tag\` tag points at, which is the tree the
\`$image\` image was published from, so the SDK the
plugins compile against and the kernel the demo runs are the same code. The
plugin manifests declare \`api_version = "$api"\` to match.
EOF
sub_block "$ROOT/README.md" pin "$work/readme-pin"

cat > "$work/install-image" <<EOF
    $image
EOF
sub_block "$ROOT/docs/INSTALL.md" image "$work/install-image"

cat > "$work/install-api" <<EOF
Each plugin declares \`api_version = "$api"\` in its \`.info.toml\`, matching the
\`KERNEL_API_VERSION\` of $api_pair at the commit the SDK is pinned to, the
\`$tag\` tag of Trovato $version.
EOF
sub_block "$ROOT/docs/INSTALL.md" api "$work/install-api"

echo "==> done. Review the diff, then run: cargo test --workspace"
