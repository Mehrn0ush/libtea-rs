#!/usr/bin/env bash
# Download a libtea release archive, check its SHA-256 against the release's
# SHA256SUMS, and install the binary.
#
# Env:  VERSION      x.y.z (no leading v)                         required
#       TARGET       from resolve-target.sh                       required
#       ARCHIVE      archive file name from resolve-target.sh     required
#       EXE          "" or ".exe"                                  required
#       REPO         owner/name, default Mehrn0ush/libtea-rs
#       INSTALL_DIR  where the binary goes                        required
#       CACHE_DIR    where archives are kept between runs         required
#       RELEASE_BASE_URL  override of https://github.com/$REPO/releases/download/v$VERSION
#
# The SHA256SUMS file is fetched on every run. A cached archive is used only if
# its hash equals the fresh sums; otherwise it is discarded and downloaded again.
# A checksum is integrity of the download, not an independent proof of origin;
# verify-attestation.sh is the second check.
#
# Exit status: 0 ok, 1 any failure (nothing is installed on failure).

set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/action/common.sh
. "$here/common.sh"
drop_credentials

for name in VERSION TARGET ARCHIVE INSTALL_DIR CACHE_DIR; do
  [[ -n "${!name:-}" ]] || die "$name is not set"
done
valid_version "$VERSION" || die "invalid version"
repo="${REPO:-Mehrn0ush/libtea-rs}"
valid_repo "$repo" || die "repo must look like owner/name"
[[ "$ARCHIVE" =~ ^libtea-[0-9A-Za-z._-]+\.(tar\.gz|zip)$ ]] || die "unexpected archive name"
[[ "$TARGET" =~ ^[A-Za-z0-9_]+(-[A-Za-z0-9_]+){2,3}$ ]] || die "unexpected target"
exe="${EXE:-}"
[[ -z "$exe" || "$exe" == ".exe" ]] || die "unexpected exe suffix"

base="${RELEASE_BASE_URL:-https://github.com/$repo/releases/download/v$VERSION}"
[[ "$base" == https://* ]] || die "the release URL must be https"

work="$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/libtea-fetch.XXXXXX")"
trap 'rm -rf "$work"' EXIT
mkdir -p "$CACHE_DIR" "$INSTALL_DIR"

fetch() { # url out max-bytes
  curl --fail --silent --show-error --location \
    --proto '=https' --proto-redir '=https' --tlsv1.2 \
    --max-filesize "$3" --retry 2 --connect-timeout 20 --max-time 600 \
    --output "$2" -- "$1"
}

fetch "$base/SHA256SUMS" "$work/SHA256SUMS" 1048576 || die "could not download SHA256SUMS for v$VERSION"
expected="$(awk -v name="$ARCHIVE" '$2 == name {print $1}' "$work/SHA256SUMS")"
[[ "$(printf '%s\n' "$expected" | grep -c .)" == "1" ]] || die "SHA256SUMS has no unique entry for $ARCHIVE"
[[ "$expected" =~ ^[0-9a-f]{64}$ ]] || die "SHA256SUMS entry for $ARCHIVE is not a SHA-256"

archive_path="$CACHE_DIR/$ARCHIVE"
if [[ -f "$archive_path" && "$(sha256_of "$archive_path")" == "$expected" ]]; then
  printf 'cache hit for %s (hash matches SHA256SUMS)\n' "$ARCHIVE"
else
  [[ ! -e "$archive_path" ]] || warn "cached $ARCHIVE does not match SHA256SUMS; downloading again"
  rm -f -- "$archive_path"
  fetch "$base/$ARCHIVE" "$work/$ARCHIVE" 268435456 || die "could not download $ARCHIVE"
  actual="$(sha256_of "$work/$ARCHIVE")"
  [[ "$actual" == "$expected" ]] || die "SHA-256 of $ARCHIVE does not match SHA256SUMS"
  mv -- "$work/$ARCHIVE" "$archive_path"
fi

root="${ARCHIVE%.tar.gz}"
root="${root%.zip}"
py="$(command -v python3 || command -v python)" || die "python is needed to unpack the archive"
"$py" -I "$here/extract-member.py" "$archive_path" "$root/libtea$exe" "$INSTALL_DIR/libtea$exe" ||
  die "could not unpack libtea from $ARCHIVE"

set_output archive-path "$archive_path"
set_output install-dir "$INSTALL_DIR"
if [[ -n "${GITHUB_PATH:-}" ]]; then
  printf '%s\n' "$INSTALL_DIR" >>"$GITHUB_PATH"
fi
printf 'installed libtea %s (%s) from a release archive whose SHA-256 matches SHA256SUMS\n' "$VERSION" "$TARGET"
