#!/usr/bin/env bash
#
# Assemble the Ritrovo plugin overlay: the one directory tree a Trovato
# kernel's PLUGINS_DIR search path can point at.
#
# Trovato discovers plugins as <dir>/<name>/<name>.wasm plus <name>.info.toml,
# but `cargo build` writes every module flat into target/. This script bridges
# the two, so nothing has to be copied into the Trovato checkout — which is the
# whole point: Ritrovo installs as a pure overlay, with zero edits to Trovato.
#
# The output is build output, not source, so it is gitignored and rebuilt
# rather than committed. Run it after `cargo build --target wasm32-wasip1
# --release`.
#
# Usage: scripts/assemble-overlay.sh [output-dir]   (default: overlay/plugins)
#
# CARGO_TARGET_DIR is honoured, because the demo's container build sets it: the
# repository is mounted read-only there and the artifacts land on a volume, so
# `$root/target` is not where they are.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build="${CARGO_TARGET_DIR:-$root/target}/wasm32-wasip1/release"
out="${1:-$root/overlay/plugins}"

rm -rf "$out"
mkdir -p "$out"

count=0
for src in "$root"/plugins/*/; do
    name="$(basename "$src")"
    wasm="$build/$name.wasm"

    if [[ ! -f "$wasm" ]]; then
        echo "error: $wasm not found — run:" >&2
        echo "  cargo build --target wasm32-wasip1 --release" >&2
        exit 1
    fi

    mkdir -p "$out/$name"
    cp "$wasm" "$out/$name/$name.wasm"
    cp "$src/$name.info.toml" "$out/$name/$name.info.toml"
    [[ -d "$src/migrations" ]] && cp -R "$src/migrations" "$out/$name/migrations"

    echo "  $name"
    count=$((count + 1))
done

echo "assembled $count plugins into $out"
