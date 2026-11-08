#!/usr/bin/env bash
# Map the runner to a release target and archive name.
#
# Env:  VERSION      x.y.z or x.y.z-suffix (no leading v)
#       RUNNER_OS    Linux | macOS | Windows        (set by GitHub)
#       RUNNER_ARCH  X64 | ARM64                    (set by GitHub)
#       LIBC         gnu | musl (Linux only; default gnu)
# Prints (and writes to $GITHUB_OUTPUT): target=, archive=, exe=
#
# Exit status: 0 ok, 1 unsupported or invalid input.

set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/action/common.sh
. "$here/common.sh"

version="${VERSION:-}"
valid_version "$version" || die "version must look like 1.2.3 (no leading v), got an invalid value"
libc="${LIBC:-gnu}"
[[ "$libc" == "gnu" || "$libc" == "musl" ]] || die "libc must be gnu or musl"

case "${RUNNER_OS:-}/${RUNNER_ARCH:-}" in
  Linux/X64) target="x86_64-unknown-linux-$libc"; exe="" ;;
  Linux/ARM64) target="aarch64-unknown-linux-$libc"; exe="" ;;
  macOS/X64) target="x86_64-apple-darwin"; exe="" ;;
  macOS/ARM64) target="aarch64-apple-darwin"; exe="" ;;
  Windows/X64) target="x86_64-pc-windows-msvc"; exe=".exe" ;;
  *) die "no libtea release for runner ${RUNNER_OS:-?}/${RUNNER_ARCH:-?}" ;;
esac
if [[ "$libc" == "musl" && "${RUNNER_OS:-}" != "Linux" ]]; then
  die "libc=musl is only available on Linux runners"
fi

case "$target" in
  *windows*) archive="libtea-$version-$target.zip" ;;
  *) archive="libtea-$version-$target.tar.gz" ;;
esac

printf 'target=%s\narchive=%s\nexe=%s\n' "$target" "$archive" "$exe"
set_output target "$target"
set_output archive "$archive"
set_output exe "$exe"
