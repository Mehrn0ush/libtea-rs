#!/usr/bin/env bash
# Verify the build-provenance attestation of the downloaded archive.
#
# Env:  ARCHIVE_PATH  the verified archive                        required
#       VERSION       the release version (no leading v)        required
#       REPO          owner/name, default Mehrn0ush/libtea-rs
#       ATTESTATION   auto | required | skip   (default auto)
#       GH_TOKEN      read-only token for the GitHub API (optional)
#
#   auto      verify when `gh` is installed; fail if verification fails;
#             warn and continue only when `gh` is missing
#   required  as auto, but a missing `gh` is a failure
#   skip      do not verify
#
# What a pass means: GitHub's records say this repository's release workflow,
# running for the tag refs/tags/v$VERSION, produced a file with this digest.
# The tag ref is part of the check, so an attestation made for another tag, a
# branch or a dry run does not satisfy it, and neither does an older release's
# archive renamed to this version. It does not say the source is safe.
#
# Exit status: 0 verified or skipped by policy, 1 failure.

set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=scripts/action/common.sh
. "$here/common.sh"
unset TEA_API_KEY_ID TEA_API_KEY_SECRET

mode="${ATTESTATION:-auto}"
case "$mode" in auto | required | skip) ;; *) die "attestation must be auto, required or skip" ;; esac
[[ "$mode" != "skip" ]] || { warn "attestation verification skipped by request"; exit 0; }

archive="${ARCHIVE_PATH:-}"
[[ -f "$archive" ]] || die "ARCHIVE_PATH is not a file"
repo="${REPO:-Mehrn0ush/libtea-rs}"
valid_repo "$repo" || die "repo must look like owner/name"
version="${VERSION:-}"
valid_version "$version" || die "VERSION must be set to the release version (for example 1.2.3)"
# The archive name carries the version; it must be the version being verified.
name="$(basename "$archive")"
rest="${name#"libtea-$version-"}"
# What follows the version must be the target, so libtea-1.2.3-dryrun.1-... is
# not accepted as version 1.2.3.
if [[ "$rest" == "$name" || ! ( "$rest" == x86_64-* || "$rest" == aarch64-* ) ]]; then
  die "the archive name does not carry version $version"
fi

if ! command -v gh >/dev/null 2>&1; then
  [[ "$mode" != "required" ]] || die "gh is not installed and attestation=required"
  warn "gh is not installed; the attestation was not verified (the SHA-256 check still passed)"
  set_output attestation "not-checked"
  exit 0
fi

gh attestation verify "$archive" --repo "$repo" \
  --signer-workflow "$repo/.github/workflows/release.yml" \
  --source-ref "refs/tags/v$version" >/dev/null ||
  die "attestation verification failed for $(basename "$archive")"
printf 'attestation verified for %s\n' "$(basename "$archive")"
set_output attestation "verified"
