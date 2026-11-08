#!/usr/bin/env bash
# Refuse to run while API-key credentials sit in the ambient environment.
#
# A calling step's `env:` (and the job's and workflow's) is inherited by EVERY
# step inside a composite action, including third-party actions such as
# actions/cache. Credentials must therefore not be passed that way. They are
# passed as the `api-key-id` and `api-key-secret` inputs, which only the final
# "Run libtea" step turns into environment variables.
#
# Fails (status 2) if any exported variable whose name starts with TEA_API_KEY
# has a non-empty value. Only names are printed, never values.

set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/action/common.sh
. "$here/common.sh"

found=()
for name in $(compgen -e); do
  case "$name" in
    TEA_API_KEY*)
      if [[ -n "${!name:-}" ]]; then
        found+=("$name")
      fi
      ;;
  esac
done

if [[ ${#found[@]} -gt 0 ]]; then
  DIE_STATUS=2
  die "${found[*]} is set in the environment of this step, so every step of this action (including third-party ones) would inherit it. Remove it from env: and pass the api-key-id and api-key-secret inputs instead (with: api-key-secret: \${{ secrets.NAME }})."
fi
