#!/usr/bin/env bash
# Run libtea with arguments built from the action's inputs.
#
# Env:  COMMAND       sync | diff | discover | download | product-release |
#                     collections | parse-tei | spec | conformance      required
#       TEI           for sync, discover, parse-tei
#       LOCK_FILE     for sync and diff (the old lock for diff)
#       LOCK_FILE_NEW for diff
#       BASE_URL      for download, product-release, collections (--base)
#       CHECK         true: sync --check
#       EXTRA_ARGS    more arguments, ONE PER LINE, passed verbatim (no shell)
#       WORKDIR       directory to run in (default: current)
#       TEA_API_KEY_ID, TEA_API_KEY_SECRET   from the caller's env (secrets)
#
# Credentials come from the environment only. EXTRA_ARGS may not carry them.
# No input is ever interpreted by a shell: arguments are built as an array.
# Only the command name is echoed, never an argument value.
#
# Exit status: libtea's own (see docs/cli-exit-codes.md); 2 for a bad input
# here. The status is also written to $GITHUB_OUTPUT as exit-code.

set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/action/common.sh
. "$here/common.sh"

DIE_STATUS=2
command_name="${COMMAND:-}"
args=()

need() { # name value
  [[ -n "$2" ]] || die "input '$1' is required for command '$command_name'"
  [[ "$2" != -* ]] || die "input '$1' must not start with '-'"
}

case "$command_name" in
  sync)
    need tei "${TEI:-}"
    need lock-file "${LOCK_FILE:-}"
    [[ "$TEI" == tei://* ]] || die "input 'tei' must start with tei://"
    args=(sync "$TEI" --lock "$LOCK_FILE")
    case "${CHECK:-false}" in
      true) args+=(--check) ;;
      false | "") ;;
      *) die "input 'check' must be true or false" ;;
    esac
    ;;
  diff)
    need lock-file "${LOCK_FILE:-}"
    need lock-file-new "${LOCK_FILE_NEW:-}"
    args=(diff "$LOCK_FILE" "$LOCK_FILE_NEW")
    ;;
  discover | parse-tei)
    need tei "${TEI:-}"
    [[ "$TEI" == tei://* ]] || die "input 'tei' must start with tei://"
    args=("$command_name" "$TEI")
    ;;
  download | product-release | collections)
    need base-url "${BASE_URL:-}"
    [[ "$BASE_URL" == https://* || "$BASE_URL" == http://* ]] || die "input 'base-url' must be an http(s) URL"
    args=("$command_name" --base "$BASE_URL")
    ;;
  spec | conformance)
    args=("$command_name")
    ;;
  "") die "input 'command' is required" ;;
  *) die "unsupported command" ;;
esac

if [[ -n "${EXTRA_ARGS:-}" ]]; then
  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    [[ -n "$line" ]] || continue
    case "$line" in
      --api-key-id | --api-key-id=* | --api-key-secret | --api-key-secret=*)
        die "credentials must come from TEA_API_KEY_ID and TEA_API_KEY_SECRET in the environment, not from arguments"
        ;;
    esac
    args+=("$line")
  done <<<"$EXTRA_ARGS"
fi

if [[ -n "${WORKDIR:-}" ]]; then
  [[ "$WORKDIR" != -* ]] || die "input 'working-directory' must not start with '-'"
  cd -- "$WORKDIR" || die "cannot enter the working directory"
fi

bin="libtea"
if ! command -v "$bin" >/dev/null 2>&1; then
  DIE_STATUS=1 die "libtea is not on PATH"
fi

printf 'running: libtea %s\n' "$command_name"
"$bin" "${args[@]}"
status=$?
set_output exit-code "$status"
exit "$status"
