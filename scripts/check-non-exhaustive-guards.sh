#!/usr/bin/env bash
# Mutation check for the `#[non_exhaustive]` compile_fail guards.
#
# Copies the workspace to a temporary directory (the repository is not
# modified), deletes every `#[non_exhaustive]` attribute line from
# crates/*/src, and runs `cargo test --workspace --doc`. In that copy:
#
#   - every `compile_fail` guard must FAIL, because its snippet now compiles;
#     a guard that still "passes" fails for some other reason (it is vacuous);
#   - every other doctest (the `Control...` items and the README snippets)
#     must still PASS.
#
# Run it after adding a variant or field to a guarded item, and in CI.
#
# Env: CARGO_TARGET_DIR  Optional; default is a directory inside the copy.
# Exit status: 0 when the guards are meaningful, 1 otherwise, 2 on setup error.

set -euo pipefail

die() {
  printf 'check-non-exhaustive-guards: %s\n' "$*" >&2
  exit 2
}

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/libtea-guards.XXXXXX")" || die "mktemp failed"
trap 'rm -rf "$TMP"' EXIT

expected=0
for file in "$ROOT"/crates/*/src/non_exhaustive_guards.rs; do
  [[ -f "$file" ]] || continue
  count="$(grep -c '^/// ```compile_fail' "$file" || true)"
  expected=$((expected + count))
done
[[ "$expected" -gt 0 ]] || die "no compile_fail guards found"

(cd "$ROOT" && tar cf - \
  --exclude=./target --exclude=./.git --exclude=./fuzz --exclude=./interop-results \
  --exclude=./.claude .) | (cd "$TMP" && tar xf -) || die "copy failed"

stripped=0
while IFS= read -r -d '' file; do
  before="$(grep -c '^[[:space:]]*#\[non_exhaustive\][[:space:]]*$' "$file" || true)"
  if [[ "$before" -gt 0 ]]; then
    perl -ni -e 'print unless /^\s*#\[non_exhaustive\]\s*$/' "$file"
    stripped=$((stripped + before))
  fi
done < <(find "$TMP/crates" -path '*/src/*' -name '*.rs' -print0)
[[ "$stripped" -gt 0 ]] || die "found no #[non_exhaustive] attribute to strip"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$TMP/target}"
OUT="$TMP/doctest.out"
# Failures are expected in the mutated copy; the exit status is not used.
(cd "$TMP" && cargo test --workspace --doc --locked --no-fail-fast >"$OUT" 2>&1) || true

vacuous="$(grep -c ' - compile fail \.\.\. ok$' "$OUT" || true)"
failing="$(grep -c ' - compile fail \.\.\. FAILED$' "$OUT" || true)"
other_failed="$(grep -E '^test ' "$OUT" | grep -v ' - compile fail ' | grep -c ' \.\.\. FAILED$' || true)"

printf 'attributes stripped:                 %s\n' "$stripped"
printf 'guards defined (compile_fail):       %s\n' "$expected"
printf 'guards that failed once stripped:    %s  (must equal the line above)\n' "$failing"
printf 'guards still passing (vacuous):      %s  (must be 0)\n' "$vacuous"
printf 'controls / README doctests failing:  %s  (must be 0)\n' "$other_failed"

if [[ "$vacuous" -ne 0 ]]; then
  echo "Vacuous guards (they fail for a reason other than #[non_exhaustive]):" >&2
  grep ' - compile fail \.\.\. ok$' "$OUT" >&2
fi
if [[ "$other_failed" -ne 0 ]]; then
  grep -E '^test ' "$OUT" | grep -v ' - compile fail ' | grep ' \.\.\. FAILED$' >&2
fi
if [[ "$vacuous" -eq 0 && "$failing" -eq "$expected" && "$other_failed" -eq 0 ]]; then
  echo "OK: every guard depends on #[non_exhaustive]."
  exit 0
fi
echo "FAIL: some guards do not depend on #[non_exhaustive], or a control broke." >&2
exit 1
