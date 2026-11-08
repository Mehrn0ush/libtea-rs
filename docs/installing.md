# Installing and verifying the `libtea` CLI

> **Status.** No release has been published yet. This page describes what
> `.github/workflows/release.yml` produces; the workflow has not been run, and
> the verification commands below have not been tried against a real release.

`libtea` is distributed as a GitHub release, not on crates.io (`libtea-cli` is
`publish = false`). Each release carries, for version `X.Y.Z`:

| File | Contents |
| --- | --- |
| `libtea-X.Y.Z-<target>.tar.gz` / `.zip` | the binary, `LICENSE`, `README.md` |
| `libtea-X.Y.Z.cdx.json` | CycloneDX 1.5 SBOM of the CLI's dependency tree |
| `SHA256SUMS` | SHA-256 of every file above |

Targets: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
(`.zip`). The musl builds are for minimal containers; use gnu elsewhere. The
macOS x86_64 binary is cross-compiled and is not run by the release smoke test.

Every file is also covered by a GitHub build-provenance attestation. A release
is created as a **draft** by the workflow and published by a maintainer by hand
after checking it; only published releases are meant for installation.

## Verify, then install

```sh
VERSION=X.Y.Z
TARGET=x86_64-unknown-linux-gnu
BASE=https://github.com/Mehrn0ush/libtea-rs/releases/download/v$VERSION
curl -fLO "$BASE/libtea-$VERSION-$TARGET.tar.gz"
curl -fLO "$BASE/SHA256SUMS"

# 1. The download is the file that was released (corruption, truncation).
sha256sum --check --ignore-missing SHA256SUMS      # macOS: shasum -a 256 -c
# 2. The file was produced by this repository's release workflow, for this tag.
gh attestation verify "libtea-$VERSION-$TARGET.tar.gz" \
  --repo Mehrn0ush/libtea-rs \
  --signer-workflow Mehrn0ush/libtea-rs/.github/workflows/release.yml \
  --source-ref "refs/tags/v$VERSION"

tar -xzf "libtea-$VERSION-$TARGET.tar.gz"
install -m 0755 "libtea-$VERSION-$TARGET/libtea" ~/.local/bin/libtea
libtea --help
```

On Windows, download the `.zip`, check it with
`(Get-FileHash .\libtea-X.Y.Z-x86_64-pc-windows-msvc.zip).Hash` against
`SHA256SUMS`, run the same `gh attestation verify` (including `--source-ref`),
and unzip.

To read the SBOM: `libtea-X.Y.Z.cdx.json` is plain JSON.

## What each check proves, and what it does not

- **`SHA256SUMS`** is served from the same release as the archives. It catches
  a damaged or partial download. It is **not** an independent proof of origin:
  whoever could replace the archive could replace this file.
- **The attestation** states that GitHub Actions ran this repository's release
  workflow at a recorded commit and produced a file with that digest. It
  does not say the source is free of defects or malicious changes, that the
  maintainers reviewed it, or that the toolchain and runner were trustworthy.
  `gh attestation verify` checks it against GitHub's records. Pass
  `--signer-workflow` and `--source-ref` as shown: without them an
  attestation made by some other workflow, or by the same workflow for another
  tag, a branch or a dry run, would satisfy the check, and so would an older
  release's genuine archive renamed to the version you asked for. With
  `--source-ref "refs/tags/v$VERSION"` the attestation must be for the tag you
  are installing. Archives and the SBOM carry the full tag version in their
  names (a dry-run tag `v1.2.3-dryrun.1` gives `libtea-1.2.3-dryrun.1-...`), and
  the release workflow refuses to attest any other set of file names.
- **The SBOM** lists what `Cargo.lock` resolves for the CLI at the tagged
  commit. It is an inventory, not a vulnerability assessment.
- **Not provided:** the binaries are not code-signed or notarized. macOS
  Gatekeeper and Windows SmartScreen may warn about them. The build is not
  claimed to be bit-for-bit reproducible (archives are deterministic given the
  binary; the compiler output is not verified to be).

## In GitHub Actions

`docs/github-action.md` describes an action that does the download, the
SHA-256 check and the attestation check for you and then runs a command.

## Build from source instead

```sh
git clone https://github.com/Mehrn0ush/libtea-rs && cd libtea-rs
git checkout vX.Y.Z
cargo build --release --locked -p libtea-cli     # needs Rust 1.89 or newer
```
