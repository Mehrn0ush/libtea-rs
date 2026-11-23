#!/usr/bin/env bash
# Produce a CycloneDX JSON SBOM of the libtea CLI's dependency tree.
#
#   make-sbom.sh <output-file>
#
# Needs `cargo-cyclonedx` on PATH (the release workflow installs a pinned
# version) and python3. The tool is pointed at crates/tea-cli, whose tree contains
# libtea-core and libtea-client; the SBOMs it also writes for the other workspace
# members are removed. Normal and build dependencies are listed; dev
# dependencies are not. The result is checked to be a CycloneDX JSON document,
# given the project's supplier and contact links, and moved to <output-file>.
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

# cargo-cyclonedx writes one SBOM per workspace member, next to each manifest,
# whichever manifest it is pointed at. Keep the CLI's and remove the rest, which
# this run created.
generated="$ROOT/crates/tea-cli/libtea-cli.cdx.json"
if ! grep -qxF "$generated" "$work/new"; then
  cat "$work/tool.log" >&2
  die "cargo cyclonedx did not write $generated"
fi
while IFS= read -r created; do
  [[ "$created" == "$generated" ]] || rm -f -- "$created"
done <"$work/new"

# cargo-cyclonedx records no supplier or SBOM author, which the NTIA minimum
# elements and BSI TR-03183-2 ask for. Add them for the document and for this
# workspace's own crates (purl `pkg:cargo/libtea-...` built from a path), plus
# where to report a vulnerability and where the VEX document is. Third-party
# components are left as the tool wrote them.
python3 -I - "$generated" <<'PY' || die "the generated file is not a CycloneDX JSON document"
import json, sys

REPO = "https://github.com/Mehrn0ush/libtea-rs"
SUPPLIER = {"name": "libtea-rs project", "url": [REPO]}
AUTHORS = [{"name": "libtea-rs maintainers"}]
REFERENCES = [
    {"type": "security-contact", "url": REPO + "/security/advisories/new"},
    {"type": "advisories", "url": REPO + "/security/advisories"},
    {
        "type": "exploitability-statement",
        "url": "https://raw.githubusercontent.com/Mehrn0ush/libtea-rs/main/vex/libtea.vex.cdx.json",
    },
]

path = sys.argv[1]
with open(path, encoding="utf-8") as handle:
    doc = json.load(handle)
assert doc.get("bomFormat") == "CycloneDX", "bomFormat"
assert str(doc.get("specVersion", "")).startswith("1."), "specVersion"
assert isinstance(doc.get("components"), list) and doc["components"], "components"

def first_party(component):
    purl = component.get("purl", "")
    return purl.startswith("pkg:cargo/libtea-") and "download_url=file" in purl

def mark(component):
    component["supplier"] = SUPPLIER
    for child in component.get("components", []):
        mark(child)

metadata = doc.setdefault("metadata", {})
metadata["supplier"] = SUPPLIER
metadata["authors"] = AUTHORS
if "component" in metadata:
    product = metadata["component"]
    mark(product)
    refs = product.setdefault("externalReferences", [])
    for ref in REFERENCES:
        if ref not in refs:
            refs.append(ref)
for component in doc["components"]:
    if first_party(component):
        mark(component)

with open(path, "w", encoding="utf-8") as handle:
    json.dump(doc, handle, indent=2)
    handle.write("\n")
PY

mkdir -p "$(dirname "$out")"
mv "$generated" "$out"
printf '%s\n' "$out"
