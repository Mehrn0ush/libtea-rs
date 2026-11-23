# Installing and verifying the `libtea` CLI

> **Status.** Releases are published on GitHub Releases (0.1.0 was the first;
> 0.1.1 to 0.1.8 and 0.2.0 to 0.2.2 follow it). This page describes what `.github/workflows/release.yml`
> produces. The verification commands below were run against each release from
> 0.1.0 to 0.2.0 with gh 2.103.3, and against the 0.2.1 and 0.2.2
> `aarch64-apple-darwin` archives.

`libtea` is distributed as a GitHub release. From 0.2.0 it
is also on crates.io as `libtea-cli`, which has no checksum file or attestation
(see "From crates.io" below). Each GitHub release carries, for version `X.Y.Z`:

| File | Contents |
| --- | --- |
| `libtea-X.Y.Z-<target>.tar.gz` / `.zip` | the binary, `LICENSE`, `README.md` |
| `libtea-X.Y.Z.cdx.json` | CycloneDX 1.5 SBOM of the CLI's dependency tree |
| `SHA256SUMS` | SHA-256 of every file above |
| `libtea-X.Y.Z.sigstore.json` | the build-provenance attestation as a Sigstore bundle (after 0.2.2) |
| `libtea-X.Y.Z.intoto.jsonl` | the same attestation's signed in-toto statement (DSSE envelope), one line (after 0.2.2) |

Targets: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-gnu`, `aarch64-unknown-linux-musl`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
(`.zip`). The macOS x86_64 binary is cross-compiled and is not run by the
release smoke test.

**Linux: gnu or musl.** The gnu archives are built on `ubuntu-24.04` and link
the system glibc. Measured on the v0.1.0 archives, both gnu archives need
**glibc 2.34 or newer** (the highest `GLIBC_x.y` version they reference is
2.34; found by scanning the binary's version strings, as `readelf` was not
available). That covers Ubuntu 22.04 and later, Debian 12 and later, and
RHEL 9 and later; it does not cover Ubuntu 20.04, Debian 11 or RHEL 8. The musl
archives are statically linked and are the portable choice: use them on older
distributions, on Alpine, and in minimal containers. The baseline is the runner
image named in `release.yml`; a later release may raise it and will say so in
the changelog.

Every file is also covered by a GitHub build-provenance attestation. From the
first release after 0.2.2 the attestation is also attached to the release, so
it can be checked without fetching it from GitHub (below). The two attestation
files are not listed in `SHA256SUMS`; the attestation signs `SHA256SUMS` and
every file listed in it. A release
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
libtea --version
```

`libtea --version` prints `libtea X.Y.Z`; it should show the version you downloaded.

The `gh attestation verify` command above was verified with gh 2.103.3 and needs
a `gh` that has `--source-ref`. With an older `gh`, check the signing workflow
and tag with `--cert-identity` instead (it cannot be combined with
`--signer-workflow`, so use one or the other):

```sh
gh attestation verify "libtea-$VERSION-$TARGET.tar.gz" \
  --repo Mehrn0ush/libtea-rs \
  --cert-identity "https://github.com/Mehrn0ush/libtea-rs/.github/workflows/release.yml@refs/tags/v$VERSION"
```

From the first release after 0.2.2, the same check works with the attestation
attached to the release instead of the one GitHub stores (for example where
only the release files were mirrored). `gh` still needs network access for
Sigstore's trusted root:

```sh
curl -fLO "$BASE/libtea-$VERSION.sigstore.json"
gh attestation verify "libtea-$VERSION-$TARGET.tar.gz" \
  --repo Mehrn0ush/libtea-rs \
  --bundle "libtea-$VERSION.sigstore.json" \
  --signer-workflow Mehrn0ush/libtea-rs/.github/workflows/release.yml \
  --source-ref "refs/tags/v$VERSION"
```

The `.intoto.jsonl` file holds the same signed statement for tools that read
in-toto envelopes, such as OpenSSF Scorecard; `gh` uses the `.sigstore.json`.

On macOS, a binary downloaded through a browser may be quarantined and refused
at first run. After verifying it, remove the flag with
`xattr -d com.apple.quarantine <file>`. Files fetched with `curl` as above are
not quarantined.

On Windows, download the `.zip`, check it with
`(Get-FileHash .\libtea-X.Y.Z-x86_64-pc-windows-msvc.zip).Hash` against
`SHA256SUMS`, run the same `gh attestation verify` (including `--source-ref`),
and unzip.

To read the SBOM: `libtea-X.Y.Z.cdx.json` is plain JSON. It names the
supplier and links to the security contact and to the VEX document
[`vex/libtea.vex.cdx.json`](../vex/libtea.vex.cdx.json), which says whether
advisories against the CLI's dependencies affect a release.

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
  tag, or a branch, would satisfy the check, and so would an older
  release's genuine archive renamed to the version you asked for. With
  `--source-ref "refs/tags/v$VERSION"` the attestation must be for the tag you
  are installing. Archives and the SBOM carry the full tag version in their
  names (a tag `v1.2.3-rc.1` gives `libtea-1.2.3-rc.1-...`), and
  the release workflow refuses to attest any other set of file names.
- **The SBOM** lists what `Cargo.lock` resolves for the CLI at the tagged
  commit. It is an inventory, not a vulnerability assessment; the VEX
  document records the assessments that were made. Third-party components
  carry what `cargo-cyclonedx` writes, which includes no supplier.
- **Not provided:** the binaries are not code-signed or notarized. macOS
  Gatekeeper and Windows SmartScreen may warn about them. The build is not
  claimed to be bit-for-bit reproducible (archives are deterministic given the
  binary; the compiler output is not verified to be).

## In GitHub Actions

`docs/github-action.md` describes an action that does the download, the
SHA-256 check and the attestation check for you and then runs a command.

## From crates.io (0.2.0 and later)

From 0.2.0, `libtea-cli` is published to crates.io. Versions
0.1.x are not; for them use the archives above or build from a tag (below).
It needs Rust 1.89 or newer.

```sh
cargo install libtea-cli --version X.Y.Z --locked
libtea --version
```

`--locked` builds with the `Cargo.lock` packaged with the crate, that is the
dependency versions recorded when it was packaged; without it Cargo resolves the
newest compatible versions instead. This route **has none of the checks above**:
there is no `SHA256SUMS` and no build-provenance attestation. Cargo checks each
downloaded crate against the checksum in the crates.io index, which proves only
that you got what crates.io serves, not which commit or workflow it was built
from. The crate is uploaded from a maintainer's machine, not by the release
workflow. You compile the binary yourself, so nothing ties it to the attested
archives. If you need the attestation, use the archives.

## With cargo-binstall (after 0.2.2)

From the first CLI release after 0.2.2, the `libtea-cli` crate tells
[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) where the
release archives are, so it downloads the binary instead of compiling it:

```sh
cargo binstall libtea-cli --version X.Y.Z
libtea --version
```

binstall takes the archive for your target from the GitHub release `vX.Y.Z`
and installs the `libtea` binary from it. It does **not** check `SHA256SUMS`
or the build-provenance attestation. To verify, download the archive and run
the checks above instead. The crate turns off binstall's quick-install
strategy, which would fetch a binary built by a third party rather than by
`release.yml`. If no archive matches your target, binstall falls back to
`cargo install`, which is the crates.io route above. For 0.2.2 and earlier the
crate has no binstall metadata, and binstall finds no archive.

## With Homebrew (0.2.2 and later)

On macOS and Linux:

```sh
brew install mehrn0ush/tap/libtea
libtea --version
```

The formula in `Mehrn0ush/homebrew-tap` downloads the release archive for
your platform: darwin on macOS, and the statically linked musl build on
Linux. Homebrew checks the archive against the SHA-256 written in the
formula. Those values are copied from the release's `SHA256SUMS` after it
passes `gh attestation verify`, but Homebrew does not check the attestation
itself. To check it, verify the archive as described above. The tap is
updated by hand after each release, so it can lag behind a new release.

## Build from source instead

```sh
git clone https://github.com/Mehrn0ush/libtea-rs && cd libtea-rs
git checkout vX.Y.Z
cargo build --release --locked -p libtea-cli     # needs Rust 1.89 or newer
```
