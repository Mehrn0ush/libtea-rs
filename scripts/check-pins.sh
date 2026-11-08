#!/usr/bin/env bash
# Fail while PIN_ME_* placeholders remain in the release workflow or the Action.
#
#   check-pins.sh            development: passes with a notice if
#                            LIBTEA_ALLOW_PIN_PLACEHOLDERS=1 (exactly "1")
#   check-pins.sh --release  release: never honors the flag
#
# The release workflow runs it with --release as its first step, so a tag cannot
# produce a release while a placeholder remains. See docs/release-pins.md for the
# commands that replace them.
#
# Exit status: 0 no placeholder (or allowed in development), 1 placeholder found.

set -euo pipefail

ROOT="${LIBTEA_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
release=false
case "${1:-}" in
  "") ;;
  --release) release=true ;;
  *) printf 'usage: check-pins.sh [--release]\n' >&2; exit 2 ;;
esac

found=()
for file in .github/workflows/release.yml action.yml; do
  [[ -f "$ROOT/$file" ]] || { printf 'check-pins: missing %s\n' "$file" >&2; exit 2; }
  while IFS= read -r line; do
    found+=("$file:$line")
  done < <(grep -n 'PIN_ME_' "$ROOT/$file" | grep -v -E '^[0-9]+:[[:space:]]*#' || true)
done

if [[ ${#found[@]} -eq 0 ]]; then
  exit 0
fi

if [[ "$release" == false && "${LIBTEA_ALLOW_PIN_PLACEHOLDERS:-}" == "1" ]]; then
  printf 'check-pins: %d placeholder(s) remain; allowed because LIBTEA_ALLOW_PIN_PLACEHOLDERS=1 (development only)\n' "${#found[@]}" >&2
  exit 0
fi

printf 'check-pins: unresolved placeholders (a release is blocked until docs/release-pins.md is followed):\n' >&2
printf '  %s\n' "${found[@]}" >&2
exit 1
