# Homebrew and Docker distribution: evaluation

**Status:** evaluation for issue #17. Recommendation at the end. Nothing is set up.

Today the CLI ships as release archives for seven targets, each covered by
`SHA256SUMS` and a GitHub build-provenance attestation (`docs/installing.md`),
and as `cargo install --git ... --tag vX.Y.Z` from source. Both new channels
would repackage the release archives; neither should build from source on its
own, or it would ship a binary the attestation does not cover.

## Homebrew tap

**What it takes.** A separate public repository `Mehrn0ush/homebrew-tap` with
`Formula/libtea.rb`: per-platform `url` (the release archive) and `sha256`
(from `SHA256SUMS`), `bin.install "libtea"`, and a `test do` that runs
`libtea --version`. Users run `brew install Mehrn0ush/tap/libtea`. Getting into
homebrew-core instead needs notability the project does not have yet, and
homebrew-core builds from source.

**Per release.** The formula's four `url`/`sha256` pairs (macOS arm64 and
x86_64, Linux arm64 and x86_64) must be updated after the GitHub release is
published. Either by hand (a few minutes), or by a workflow in the tap that
reads `SHA256SUMS`, which needs a token with write access to the tap from the
release workflow, or a scheduled job in the tap. Adding a write token to the
release workflow widens what that workflow can do; a job in the tap that pulls
is the safer shape.

**Supply chain.** Homebrew checks the formula's `sha256`, which is only as good
as the formula's author copying it from `SHA256SUMS`. Homebrew does not verify
GitHub attestations for third-party taps. So a tap user gets integrity (the
archive is the one whose hash the tap names) but not provenance, unless they run
`gh attestation verify` themselves. The tap repository becomes a second place an
attacker could change what users install; it needs the same branch protection
as this repository.

**macOS quarantine.** Not an issue: Homebrew downloads with `curl`, so the
binary is not quarantined, but it is also not code-signed or notarized (same as
the archives).

**Cost.** One repository, branch protection, a formula, about 5 minutes per
release by hand. Low.

## Docker image

**What it takes.** A `Dockerfile` that copies the verified musl binary into
`FROM scratch` (the binary bundles its root certificates through
`webpki-roots`, so it needs no OS certificate store), built in a workflow job
after the attest job, pushed to `ghcr.io/mehrn0ush/libtea`. That job needs
`packages: write` in the release workflow, and an image attestation
(`actions/attest-build-provenance` with `push-to-registry: true`, which also
needs `attestations: write` and `id-token: write`; the release workflow already
grants those to its attest job).

**Per release.** Automatic once set up. But the release workflow's permissions
grow (`packages: write`), its policy tests (`workflow_policy.rs`) need new
rules, and the image tag (`0.1.5`, never `latest` only) must match the release.

**Supply chain.** Users can verify the image with `gh attestation verify
oci://ghcr.io/mehrn0ush/libtea:<tag>`; digests pin it. A `scratch` image has no
shell or package manager, so the attack surface is the binary itself. Root
certificates are fixed at build time (true of the archives too): a revoked or
added CA reaches users only with a new release.

**Use case.** The GitHub Action already installs a verified binary for CI.
A container is mainly useful for CI systems other than GitHub Actions and for
users who run tools only from images. No user has asked for it.

**Cost.** One workflow job, a `Dockerfile`, new permissions and tests, and a
GHCR package to keep public. Medium, and it touches the release workflow, the
most sensitive file in the repository.

## A third option: `cargo-binstall`

Once `libtea-cli` is on crates.io (issue #14), adding
`[package.metadata.binstall]` with the release-archive URL pattern lets
`cargo binstall libtea-cli` download the release archive instead of building.
No new repository, no new workflow permission; `cargo-binstall` checks the
crate's published metadata and can verify the archive's checksum. It depends on
#14.

## Recommendation

**Neither for now.** No user has asked; both add a channel to keep correct on
every release, and the Docker image adds permissions to the release workflow.
If one is wanted:

1. First, `cargo-binstall` metadata after #14 (cheapest, no new
   infrastructure).
2. Then a Homebrew tap if macOS users ask, updated by hand or by a pull job in
   the tap, never with a write token in this repository's release workflow.
3. A Docker image only on concrete demand from a non-GitHub CI user.
