# Container image: design

**Status:** design for issue #17, accepted 2026-11-26 with the four
recommended decisions (end of this note). Step 1 of the plan, the workflow and
its tests, is done (2026-11-28). It changed how the image is built: a script
writes it instead of `docker buildx` (see "Changed while building it"). No
image has been pushed yet; the first comes with the next release. The Homebrew
part of #17 shipped in 0.2.2 (`docs/design/distribution.md`). This note
replaces that note's "Docker image" section where they differ.

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
   `crates/tea-cli/tests/workflow_policy.rs` bans `docker push`,
   `docker login` and `skopeo`). The image is made by a separate workflow, so
   the most sensitive file in the repository gets no new permission.
2. **The image follows the published release, never the draft.** Users can
   only see an image for a release that the maintainer has already checked and
   published (step 14 of `docs/release-checklist.md`).
3. **The image job verifies before it builds.** It checks what users download
   (the published release files), with the same commands `docs/installing.md`
   gives users.
4. **No build tool and no third-party code runs with `packages: write`.** The
   job runs `gh`, `sha256sum`, `awk`, `skopeo`, `podman` and `docker login`,
   all from the runner image, and one file from the repository:
   `scripts/make-oci-image.py`, standard-library Python, from the tagged
   commit, run with `python3 -I`. No `cargo`, no package manager, nothing
   downloaded and run.
5. **No third-party action is added.** The only actions are `actions/checkout`
   and `actions/attest-build-provenance`, with the same pins as `release.yml`.

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

A prerelease gets no image: the job has
`if: github.event.release.prerelease == false`, so it is skipped and asks for
no approval.

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
   `${{ }}` expression in a `run:` body. Check that `GITHUB_REF` is
   `refs/tags/<it>` and that it matches `^v[0-9]+\.[0-9]+\.[0-9]+$`, that
   `gh release view` reports the release as neither a draft nor a prerelease,
   and that the tagged commit is on `main` (the compare API reports
   `identical` or `behind`). Stop on anything else.
2. Download `SHA256SUMS`, the two musl archives and
   `libtea-X.Y.Z.sigstore.json` into a new directory with
   `gh release download`.
3. Take the line for each archive from `SHA256SUMS`, which must list it
   exactly once with a 64-hex hash, and run `sha256sum --check --strict` on
   those two lines. There is no `--ignore-missing`, which would accept a
   `SHA256SUMS` that lists neither archive.
4. Verify `SHA256SUMS` and both archives with
   `gh attestation verify --bundle libtea-X.Y.Z.sigstore.json --repo
   Mehrn0ush/libtea-rs --signer-workflow
   Mehrn0ush/libtea-rs/.github/workflows/release.yml --source-ref
   refs/tags/vX.Y.Z --deny-self-hosted-runners`. The bundle is the one
   attached to the release, so this checks the same thing a user's offline
   check does.
5. Check out only `scripts/make-oci-image.py` (sparse checkout,
   `persist-credentials: false`).
6. Build the image twice into two new directories and stop unless the digests
   agree.
7. Load it into `podman` from the OCI layout, check that its user is
   `65532:65532`, and check that `--version` (amd64, no network) prints
   `libtea X.Y.Z`. The arm64 image is not run: there is no emulation on the
   runner.
8. `docker login ghcr.io` with the job token on stdin. If the tag `X.Y.Z`
   already exists, it must already be this digest (a re-run), or the job
   fails: tags are never moved. Otherwise
   `skopeo copy --all --preserve-digests` pushes the layout as is. Then the
   registry must serve this digest for the tag.
9. `actions/attest-build-provenance` with `subject-name:
   ghcr.io/mehrn0ush/libtea`, `subject-digest:` the digest from step 6, and
   `push-to-registry: true`.
10. Run the commands from the Goal section on the pushed digest:
    `gh attestation verify oci://ghcr.io/mehrn0ush/libtea@<digest>` and
    `podman run ... --version`. The job fails if either fails. By then the
    image is pushed, so a failure here means the image must be deleted by
    hand (see Rollback).
11. `docker logout ghcr.io`, even after a failure.

## The image: `scripts/make-oci-image.py`

```sh
python3 -I scripts/make-oci-image.py --version X.Y.Z --revision <commit> \
  --amd64 libtea-X.Y.Z-x86_64-unknown-linux-musl.tar.gz \
  --arm64 libtea-X.Y.Z-aarch64-unknown-linux-musl.tar.gz --out <new directory>
```

It writes an OCI image layout and prints the image digest. For each platform:

- **One layer:** an uncompressed ustar archive with a single entry, `libtea`,
  mode 0555, uid and gid 0, no user or group names, mtime 0. The bytes are
  the binary from the archive, unchanged.
- **The config:** `Entrypoint: ["/libtea"]`, `User: "65532:65532"`, and the
  labels `org.opencontainers.image.source`, `.version`, `.revision`,
  `.licenses` (Apache-2.0), `.title` and `.description`. It has no `created`
  time and no history.
- **The index:** both manifests, with the same labels as annotations. GHCR
  reads them to link the package to the repository.

All JSON is written with sorted keys and no spaces. So the same archives,
version and revision give the same bytes on any machine and any Python 3. The
layer is uncompressed because gzip output depends on the zlib in use.

Before it writes anything, the script checks:

- each archive's name;
- that each archive holds exactly the four entries `package-release.py` writes
  (the directory, `LICENSE`, `README.md` and `libtea`), all regular files or
  the directory;
- that each binary is a 64-bit little-endian ELF file for its architecture
  (`e_machine` 62 or 183).

It refuses an output directory that already exists, and a version or revision
of the wrong shape.

Why an image with nothing but the binary works:

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

## Changed while building it (2026-11-28)

The accepted design built the image with `docker buildx build --platform
linux/amd64,linux/arm64` from a `Dockerfile`. On a GitHub runner, a
multi-platform buildx build needs the `docker-container` driver, and that
driver pulls a BuildKit image from Docker Hub at run time. That image would be
unpinned third-party code running in the job that holds `packages: write` and
`id-token: write`, which property 4 rules out.

So `scripts/make-oci-image.py` writes the image itself, and `skopeo` and
`podman`, which are on the runner image, push it and run it. There is no
`Dockerfile`. The image is the same as the `Dockerfile` would have made: one
file, the same user and entrypoint. It is also reproducible by construction,
where buildx needed `SOURCE_DATE_EPOCH` and `rewrite-timestamp`.

## What the image does not give you

- **The CA list is the one built into the binary.** A CA that is revoked or
  added reaches users only in a new release. This is true of the archives too.
- **The image's attestation is a second attestation.** It says that
  `image.yml` built this digest from tag `vX.Y.Z`. That the binary inside came
  from `release.yml` is shown by step 4 of the job, and by rebuilding the
  image: anyone with the two archives can rebuild it and compare digests (see
  "Checking it yourself"). The attestation of the archive does not cover the
  image digest.
- **No `latest` tag** (decision 2).
- **No SBOM attestation on the image** in the first version. The release's
  CycloneDX SBOM describes the same binary. `actions/attest-sbom` could attach
  it later.

## Checking it yourself

`docs/installing.md` gets a "With Docker" section, from the first release
with an image:

- the `gh attestation verify oci://...` command from the Goal section;
- pinning by digest (`ghcr.io/mehrn0ush/libtea@sha256:...`);
- rebuilding from the verified archives: run `scripts/make-oci-image.py` from
  the tagged commit with the version, the tagged commit's SHA and the two
  archives, and compare the digest it prints with the pushed one.

The script was checked this way on 2026-11-28 with the 0.2.3 archives: they
were verified with `SHA256SUMS` and `gh attestation verify --bundle`, built
twice, and gave the same digest
(`sha256:ddb7958cb8ae890e609427bcece716d45f50f03cc904cc7182c3ea5f9fd75840`
with revision `17a972f5`). Nothing was pushed. The job compares two builds on
every run as well.

## Tests

`crates/tea-cli/tests/oci_image.rs` runs the script on archives made by
`scripts/package-release.py`. It checks:

- that two builds give the same digest and the same files;
- every blob's digest and size;
- the platforms, the config and the labels;
- that the layer holds exactly the binary, with the mode, owner and mtime
  above.

It also checks that the script refuses:

- swapped archives;
- a binary for the wrong architecture, or a file that is not ELF;
- an extra or a missing file;
- a bad version or revision;
- an existing output directory.

`crates/tea-cli/tests/workflow_policy.rs` checks `image.yml`:

- **Triggers:** only `release: [published]`, with no `workflow_dispatch` or
  other trigger, and the prerelease `if:`.
- **Permissions:** top-level `permissions: {}`, one job with exactly the four
  permissions above, and `environment: release`.
- **The token:** no `secrets.`, and no `${{ }}` in a `run:` body. The token
  reaches exactly five steps, as `GH_TOKEN` or `REGISTRY_TOKEN` in `env:`. The
  login step is exactly `docker login --password-stdin`.
- **Step order:** checksums, release attestation, build, run, push, attest,
  verify. Nothing reaches the registry before the build, and there is no
  `--ignore-missing`.
- **Commands:** no build tool, package manager, `curl`, `sudo` or shell
  script. The only Python is `python3 -I scripts/make-oci-image.py`. The one
  checkout is sparse and has `persist-credentials: false`.
- **Actions:** only `actions/checkout` and `actions/attest-build-provenance`,
  each pinned exactly as in `release.yml`.
- **Tags:** one tag, `$image:$VERSION`, never `latest`. An existing tag must
  already be this digest. The attestation subject is the built digest.

The `SHA256SUMS` step also runs in the tests. Each archive listed once
passes. It fails on:

- an archive that is missing;
- an archive listed twice;
- a wrong or short hash;
- a `SHA256SUMS` that lists only other files, or nothing;
- an archive missing from the download.

## Settings you change, once

- After the first push, make the GHCR package `libtea` public (package
  settings, "Change visibility"). A package pushed from a workflow starts
  private. The `org.opencontainers.image.source` annotation links it to the
  repository.
- Nothing else. The `release` Environment and its reviewer already exist.

## Rollback

A bad image is deleted by the maintainer in the package settings, or with
`gh api -X DELETE` on the package version. Tags are never moved to a different
digest. A fixed image is a new release.

## Steps (one PR each)

1. **Done (2026-11-28):** `scripts/make-oci-image.py`, `image.yml`, and the
   tests. The workflow cannot run until a release is published, so the PR was
   checked with the tests, `actionlint` and the local 0.2.3 build above.
2. The next release (0.2.4 or later) is the first with an image. After
   publishing it, you approve the Environment for the image run, make the
   package public, and check the commands from the Goal section.
3. Docs: `docs/installing.md` (Docker section), `docs/release-checklist.md`
   (approving the image run after step 14 and checking it), README, ROADMAP,
   and `docs/design/distribution.md` marked done. Then close #17.

## Decisions (2026-11-26)

1. **Trigger:** a separate `image.yml` on `release: published`, as above. A
   job in `release.yml` would have pushed the image while the release was
   still a draft, and given the release workflow `packages: write`.
2. **Tags:** only `X.Y.Z`. No `X.Y` and no `latest`: a moving tag means a pull
   can change under you. Users who want updates follow the releases.
3. **No backfill:** the first image comes with the next release. An image for
   0.2.3 could only be made by a dispatched run, whose attestation names a
   branch, not the tag, and so would need a different verify command from
   every later image.
4. **Environment approval:** the image job keeps `environment: release`. It
   is one more approval per release, but nothing gets `packages: write`
   without a person approving.
