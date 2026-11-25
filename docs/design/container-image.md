# Container image: design

**Status:** proposal for issue #17, for review before any code. Nothing is
built or pushed. The Homebrew part of #17 shipped in 0.2.2
(`docs/design/distribution.md`). This note replaces that note's "Docker image"
section where they differ.

## Goal

`ghcr.io/mehrn0ush/libtea:X.Y.Z`, a multi-platform image (`linux/amd64`,
`linux/arm64`) holding only the `libtea` binary from the release archives
`libtea-X.Y.Z-x86_64-unknown-linux-musl.tar.gz` and
`libtea-X.Y.Z-aarch64-unknown-linux-musl.tar.gz`, after they pass the
`SHA256SUMS` and attestation checks. The image has its own build-provenance
attestation, so this works:

```sh
gh attestation verify oci://ghcr.io/mehrn0ush/libtea:X.Y.Z --repo Mehrn0ush/libtea-rs \
  --signer-workflow Mehrn0ush/libtea-rs/.github/workflows/image.yml --source-ref refs/tags/vX.Y.Z
docker run --rm ghcr.io/mehrn0ush/libtea:X.Y.Z --version
```

Nothing is compiled for the image. The binary in it is byte-for-byte the one
in the attested archive, so the image adds a package format, not a new build.

## Properties the design keeps

1. **`release.yml` does not change.** It keeps its rule that it never
   publishes anything (`release_never_publishes_to_a_registry` in
   `crates/tea-cli/tests/workflow_policy.rs` bans `docker push`). The image is
   made by a separate workflow, so the most sensitive file in the repository
   gets no new permission.
2. **The image follows the published release, never the draft.** Users can
   only see an image for a release that the maintainer has already checked and
   published (step 14 of `docs/release-checklist.md`).
3. **The image job verifies before it builds.** It checks what users download
   (the published release files), with the same commands `docs/installing.md`
   gives users.
4. **No repository code runs with `packages: write`.** The job runs `sha256sum`,
   `tar`, `gh` and `docker`, never `cargo` or a script from the repository.
   This is the rule `release_privileged_jobs_run_no_build_tools` applies to the
   attest job.
5. **No third-party action is added.** `docker buildx` and `gh` are on the
   runner image. The only action used is `actions/attest-build-provenance`,
   which is already pinned.

## The workflow: `.github/workflows/image.yml`

**Trigger.** Only `release: types: [published]`. Publishing the draft with
the maintainer's own `gh release edit --draft=false` starts it. An event made
with the `GITHUB_TOKEN` would not, but no workflow publishes releases. The run
uses `image.yml` from the tagged commit, and `GITHUB_REF` is
`refs/tags/vX.Y.Z`, which is what `--source-ref` checks.

There is no `workflow_dispatch`. A dispatched run would use the workflow from
the branch it is started on, so its attestation would name
`refs/heads/main`, not the tag, and the verify command above would reject it.
A failed run is retried with "Re-run jobs" on the same run, which keeps the
tag. It also follows that the first image is for the first release whose
tagged commit contains `image.yml`. Releases before it, 0.2.3 included, get no
image (decision 3).

A prerelease (a tag with a `-` suffix, which includes dry runs) gets no image:
the job ends before it builds.

**One job, `image`.**

```yaml
permissions: {}
jobs:
  image:
    runs-on: ubuntu-24.04
    environment: release
    permissions:
      contents: read
      packages: write
      id-token: write
      attestations: write
```

The `release` Environment's required reviewer approves the run, as for the
attest and draft-release jobs. No other job and no secret: `docker login`
reads the job's `GITHUB_TOKEN` from stdin.

**Steps.**

1. Take the tag from the `GITHUB_REF_NAME` environment variable, never from a
   `${{ }}` expression in a `run:` body (the rule
   `release_run_bodies_interpolate_no_expressions` sets for `release.yml`).
   Check that `GITHUB_REF` is `refs/tags/<it>`, and that it matches
   `^v[0-9]+\.[0-9]+\.[0-9]+$`. Stop on anything else, prereleases included.
2. Check that the release is published. `gh release view "$TAG" --json
   isDraft,isPrerelease` must give `false,false`.
3. In a new, empty directory, download `SHA256SUMS`, the two musl archives and
   `libtea-X.Y.Z.sigstore.json` with `gh release download`.
4. `sha256sum --check --strict --ignore-missing SHA256SUMS`, and check that
   both archives are listed in it. `--ignore-missing` without the second check
   would accept a download of nothing.
5. Verify both archives and `SHA256SUMS` with
   `gh attestation verify --bundle libtea-X.Y.Z.sigstore.json --repo
   Mehrn0ush/libtea-rs --signer-workflow
   Mehrn0ush/libtea-rs/.github/workflows/release.yml --source-ref
   refs/tags/vX.Y.Z`. The bundle is the one attached to the release, so this
   checks the same thing a user's offline check does.
6. Unpack each archive into `context/linux/amd64/` or `context/linux/arm64/`.
   Each archive must hold exactly the files `scripts/package-release.py`
   writes, with no links, absolute paths or `..`. Then run the unpacked
   `linux/amd64` binary's `--version` and check it prints `libtea X.Y.Z`. The
   runner is amd64, so the arm64 binary is not run.
7. Build and push with `docker buildx build --platform linux/amd64,linux/arm64`
   from the `Dockerfile` below. The build context is only `context/`. The build
   is reproducible:
   `SOURCE_DATE_EPOCH` is the tagged commit's time (`git log -1
   --format=%ct` in the checkout), and the output uses
   `rewrite-timestamp=true`. It pushes one tag, `X.Y.Z`, and records the
   manifest-list digest.
8. `actions/attest-build-provenance` with `subject-name:
   ghcr.io/mehrn0ush/libtea`, `subject-digest:` the digest from step 7, and
   `push-to-registry: true`.
9. Pull the image by digest, run `--version` on `linux/amd64`, and run
   `gh attestation verify oci://ghcr.io/mehrn0ush/libtea@<digest>` with the
   flags from the Goal section. The job fails if either fails. By then the
   image is pushed, so a failure here means the image must be deleted by hand
   (see Rollback).

## The `Dockerfile`

```dockerfile
FROM scratch
ARG TARGETPLATFORM
COPY --chmod=0555 ${TARGETPLATFORM}/libtea /libtea
USER 65532:65532
ENTRYPOINT ["/libtea"]
```

It has no `RUN`, so building for arm64 on an amd64 runner needs no emulation.
The repository keeps it at `packaging/container/Dockerfile`. The image job
checks out the tagged commit with `persist-credentials: false` and a sparse
checkout of that one file, so it reads the file and runs nothing from the
repository. Labels (`org.opencontainers.image.source`, `.version`,
`.revision`, `.licenses=Apache-2.0`, `.description`) are set with `--label`
on the command line, so the `Dockerfile` does not change between releases.

Why `scratch` works:

- The musl binaries are statically linked.
- TLS uses rustls with the `webpki-roots` certificates built into the binary
  (`reqwest` feature `rustls-tls`), so the image needs no
  `/etc/ssl/certs`.
- DNS uses the `/etc/resolv.conf` the container runtime provides.
- The binary writes nothing unless asked. The cache is off unless
  `--cache-dir` or `LIBTEA_CACHE_DIR` is set, and `lock`, `sync` and `fetch`
  write where they are told.

User 65532 is unprivileged. To write to a mounted directory you need
`docker run --user "$(id -u):$(id -g)" -v "$PWD:/work" -w /work ...`. The
installing doc will show that.

## What the image does not give you

- **The CA list is the one built into the binary.** A CA that is revoked or
  added reaches users only in a new release. This is true of the archives too.
- **The image's attestation is a second attestation.** It says that
  `image.yml` built this digest from tag `vX.Y.Z`. That the binary inside came
  from `release.yml` is shown by step 5 of the job, and by rebuilding the image:
  anyone with the two archives and the `Dockerfile` can rebuild it and compare
  digests (see "Checking it yourself"). The attestation of the archive does not
  cover the image digest.
- **No `latest` tag** (decision 2).
- **No SBOM attestation on the image** in the first version. The release's
  CycloneDX SBOM describes the same binary. `actions/attest-sbom` could attach
  it later.

## Checking it yourself

`docs/installing.md` gets a "With Docker (0.2.4 and later)" section:

- the `gh attestation verify oci://...` command from the Goal section;
- pinning by digest (`ghcr.io/mehrn0ush/libtea@sha256:...`);
- rebuilding from the verified archives: unpack them into `linux/amd64/` and
  `linux/arm64/`, run the `docker buildx build` command from the workflow with
  the same `SOURCE_DATE_EPOCH`, and compare digests.

The rebuild is also how this design gets tested before the first push. If the
digests do not match on a second machine, the image is not reproducible, and
the installing doc must not say it is.

## Tests (`workflow_policy.rs`)

New tests for `image.yml`, modelled on the `release.yml` ones:

- The only trigger is `release: [published]`. No `workflow_dispatch`, no
  inputs.
- Top-level `permissions: {}`. One job, `image`, with exactly the four
  permissions above, and `environment: release`.
- No `secrets.` anywhere. `GITHUB_TOKEN` reaches only the `gh` and
  `docker login` steps, through `env:`.
- No `cargo`, `python`, `scripts/` or `./` command in any step. Checkouts use
  `persist-credentials: false` and are sparse.
- The verify steps (`sha256sum --check`, `gh attestation verify --bundle` with
  `--signer-workflow .../release.yml` and `--source-ref`) come before the
  build-and-push step.
- The only `uses:` are `actions/checkout` and
  `actions/attest-build-provenance`, both pinned by SHA.
- No `${{ }}` expressions in `run:` bodies.
- The pushed tag is the version, never `latest`.

`release_never_publishes_to_a_registry` stays as it is: `release.yml` still
pushes nothing.

Shell parts that can run offline (tag check, archive listing check) get tests
that run them, like `the_attest_job_rejects_any_other_set_of_files`.

## Settings you change, once

- After the first push, make the GHCR package `libtea` public and link it to
  the repository (package settings, "Change visibility"). A package pushed
  from a workflow starts private.
- Nothing else. The `release` Environment and its reviewer already exist.

## Rollback

A bad image is deleted by the maintainer in the package settings, or with
`gh api -X DELETE` on the package version. Tags are never moved to a different
digest. A fixed image is a new release.

## Steps (one PR each)

1. `packaging/container/Dockerfile`, `image.yml`, and the policy tests. The
   workflow cannot run until a release is published, so the PR is checked by
   the tests, `actionlint` and a local build from the verified 0.2.3 archives
   (built twice, digests compared; nothing pushed).
2. The next release (0.2.4 or later) is the first with an image. After
   publishing it, you approve the Environment for the image run, make the
   package public, and check the commands from the Goal section.
3. Docs: `docs/installing.md` (Docker section), `docs/release-checklist.md`
   (approving the image run after step 14 and checking it), README, ROADMAP,
   `docs/release-pins.md` (no new pin, but `image.yml` joins the
   `actions/checkout` and `actions/attest-build-provenance` rows), and
   `docs/design/distribution.md` marked done. Then close #17.

## Decisions for you

1. **Trigger.** A separate `image.yml` on `release: published` (recommended,
   as above), or a job in `release.yml`. A job in `release.yml` would push the
   image while the release is still a draft, and give the release workflow
   `packages: write`.
2. **Tags.** Only `X.Y.Z` (recommended), or also `X.Y` and `latest`. Moving
   tags are convenient but mean a pull can change under you. Users who want
   updates can follow the releases.
3. **No backfill.** The first image comes with the next release (recommended).
   An image for 0.2.3 could only be made by a dispatched run, whose
   attestation names a branch, not the tag, and so needs a different verify
   command from every later image. If you want 0.2.3 anyway, the docs would
   have to give that command separately.
4. **Environment approval.** Keep `environment: release` on the image job
   (recommended). It means one more approval per release, but nothing gets
   `packages: write` without a person approving.
