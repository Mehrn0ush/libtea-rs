# Shared helpers for the GitHub Action scripts. Sourced, never executed.
# shellcheck shell=bash

die() {
  printf '::error::%s\n' "$*" >&2
  exit "${DIE_STATUS:-1}"
}

warn() {
  printf '::warning::%s\n' "$*" >&2
}

# Credentials are for libtea only. The scripts that download and verify never
# need them, so drop them before anything is fetched.
drop_credentials() {
  unset TEA_API_KEY_ID TEA_API_KEY_SECRET
}

valid_version() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]]
}

valid_repo() {
  [[ "$1" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] || return 1
  # "." and ".." would turn the release URL into a path walk.
  local owner="${1%%/*}" name="${1#*/}"
  [[ "$owner" != "." && "$owner" != ".." && "$name" != "." && "$name" != ".." ]]
}

# Print the lowercase SHA-256 of a file. The file is read from standard input:
# GNU sha256sum starts its output line with a backslash when the file name
# contains one (every Windows path does), which is not part of the hash.
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum <"$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 <"$1" | awk '{print $1}'
  else
    python3 -I -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"
  fi
}

# Append name=value to $GITHUB_OUTPUT when it is set. Values never contain
# newlines (callers pass fixed strings and validated tokens).
set_output() {
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    printf '%s=%s\n' "$1" "$2" >>"$GITHUB_OUTPUT"
  fi
}
