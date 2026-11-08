#!/usr/bin/env bash
# Produce a CycloneDX JSON SBOM of the libtea CLI's dependency tree.
#
#   make-sbom.sh <output-file>
#
# Needs `cargo-cyclonedx` on PATH (the release workflow installs a pinned
# version) and python3. The tool is run for crates/tea-cli, whose tree contains
# libtea-core and libtea-client. Normal and build dependencies are listed; dev
# dependencies are not. The result is checked to be a CycloneDX JSON document
# before it is moved to <output-file>.
#
# This lists what Cargo.lock resolves for the CLI. It is not a statement about
# the safety of those components.

set -euo pipefail

die() {
  printf 'make-sbom: %s\n' "$*" >&2
  exit 1
}

[[ $# -eq 1 ]] || die "usage: make-sbom.sh <output-file>"
out="$1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
command -v cargo-cyclonedx >/dev/null 2>&1 || die "cargo-cyclonedx is not installed"
command -v python3 >/dev/null 2>&1 || die "python3 is not installed"

work="$(mktemp -d "${TMPDIR:-/tmp}/libtea-sbom.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The tool writes next to the manifest. Remember what was there before.
find "$ROOT/crates" -maxdepth 2 \( -name '*.cdx.json' -o -name '*bom*.json' \) >"$work/before" || true

(cd "$ROOT" && cargo cyclonedx --manifest-path crates/tea-cli/Cargo.toml --format json --spec-version 1.5 >"$work/tool.log" 2>&1) ||
  { cat "$work/tool.log" >&2; die "cargo cyclonedx failed"; }

find "$ROOT/crates" -maxdepth 2 \( -name '*.cdx.json' -o -name '*bom*.json' \) >"$work/after" || true
sort "$work/before" >"$work/before.s"
sort "$work/after" >"$work/after.s"
comm -13 "$work/before.s" "$work/after.s" >"$work/new"
count="$(wc -l <"$work/new" | tr -d ' ')"
[[ "$count" == "1" ]] || {
  cat "$work/tool.log" >&2
  die "expected exactly one new SBOM file, found $count"
}
generated="$(cat "$work/new")"

python3 -I - "$generated" <<'PY' || die "the generated file is not a CycloneDX JSON document"
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    doc = json.load(handle)
assert doc.get("bomFormat") == "CycloneDX", "bomFormat"
assert str(doc.get("specVersion", "")).startswith("1."), "specVersion"
assert isinstance(doc.get("components"), list) and doc["components"], "components"
PY

mkdir -p "$(dirname "$out")"
mv "$generated" "$out"
printf '%s\n' "$out"
