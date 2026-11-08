#!/usr/bin/env bash
# Validate the tag that triggered the release workflow and print, for
# $GITHUB_OUTPUT: version=<x.y.z> and dry_run=<true|false>.
#
#   check-release-tag.sh <tag>
#
# The tag must be `v<version>` (a real release) or `v<version>-<suffix>` (a dry
# run, published as a draft prerelease), where <version> equals the version of
# crates/tea-cli/Cargo.toml. The tag text is checked against a strict pattern
# before it is used anywhere, because it is attacker-influenced input.
#
# Exit status: 0 valid, 1 invalid tag, 2 setup error.

set -euo pipefail

die() {
  printf 'check-release-tag: %s\n' "$*" >&2
  exit "${2:-1}"
}

[[ $# -eq 1 ]] || die "usage: check-release-tag.sh <tag>" 2
tag="$1"
ROOT="${LIBTEA_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"

[[ "$tag" =~ ^v([0-9]+\.[0-9]+\.[0-9]+)(-[0-9A-Za-z.]+)?$ ]] ||
  die "tag must look like v1.2.3 or v1.2.3-dryrun.1, got a different shape"
tag_version="${BASH_REMATCH[1]}"
suffix="${BASH_REMATCH[2]:-}"

manifest="$ROOT/crates/tea-cli/Cargo.toml"
[[ -f "$manifest" ]] || die "missing $manifest" 2
crate_version="$(sed -n 's/^version = "\([0-9][0-9.]*\)"$/\1/p' "$manifest" | head -n 1)"
[[ -n "$crate_version" ]] || die "no version in $manifest" 2

[[ "$tag_version" == "$crate_version" ]] ||
  die "tag version $tag_version does not match crates/tea-cli version $crate_version"

if [[ -n "$suffix" ]]; then
  dry_run=true
else
  dry_run=false
fi
printf 'version=%s\n' "$crate_version"
printf 'dry_run=%s\n' "$dry_run"
