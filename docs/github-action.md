# GitHub Action

> **Status.** Written for 0.1.x. The scripts are tested against stubs in CI, and
> the Action was run from a workflow on GitHub-hosted runners (see Notes and
> limits). Use 0.1.2 or later: the 0.1.0 and 0.1.1 tags have a SHA-256 check
> that fails on Windows. The Action installs a released `libtea` binary
> (`docs/installing.md`). Its one third-party dependency, `actions/cache`, is
> pinned by commit SHA (`docs/release-pins.md`). The examples use `parse-tei`
> in the test runs; the commands that talk to a server were not part of them.

The action (`action.yml` at the repository root) downloads the `libtea` release
for the runner, checks it, caches it, and runs one command. Use it to keep a
lock file honest in CI:

```yaml
- uses: Mehrn0ush/libtea-rs@<commit-sha-of-a-release-tag>
  with:
    version: "0.1.6"
    command: sync
    tei: tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
    lock-file: tea.lock
    check: "true"
```

## Inputs

| Input | Meaning |
| --- | --- |
| `version` (required) | The libtea version to install, for example `0.1.6`. There is no `latest`: pin it. |
| `command` (required) | `sync`, `diff`, `discover`, `download`, `product-release`, `collections`, `parse-tei`, `spec` or `conformance`. |
| `tei` | A `tei://` identifier, for `sync`, `discover`, `parse-tei`. |
| `lock-file` | Lock path for `sync`; the old lock for `diff`. |
| `lock-file-new` | The new lock for `diff`. |
| `base-url` | API base for `download`, `product-release`, `collections` (becomes `--base`). |
| `check` | `"true"` runs `sync --check`: exit 7 when the server differs from the lock. |
| `api-key-id`, `api-key-secret` | The API key, from secrets; see Credentials. Optional for servers that need no key. |
| `extra-args` | More `libtea` arguments, **one per line**, passed as-is with no shell (so `--artifact`, UUID, `--output`, `--allow-origin`, `--json`, ...). |
| `working-directory` | Directory to run in. |
| `libc` | Linux only: `gnu` (default) or `musl`. The action does not detect the distribution: on a runner or container with glibc older than 2.34 (for example Ubuntu 20.04) or without glibc (Alpine) set `musl`. See `docs/installing.md`. |
| `repository` | Whose releases to use; default `Mehrn0ush/libtea-rs`. |
| `attestation` | `auto` (default: verify when `gh` is installed), `required` (fail without `gh`), `skip`. |
| `cache` | `"true"` (default) caches the archive between runs. |

Outputs: `exit-code` (libtea's status, see `docs/cli-exit-codes.md`; the step
also fails unless it is 0) and `attestation` (`verified` or `not-checked`).

## Credentials

Pass the API key with the `api-key-id` and `api-key-secret` inputs, from
repository secrets:

```yaml
with:
  api-key-id: ${{ secrets.TEA_API_KEY_ID }}
  api-key-secret: ${{ secrets.TEA_API_KEY_SECRET }}
```

Both inputs are read in exactly one place: the environment of the final step
that runs `libtea` (as `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET`). No other step
of the action references them.

**Do not set `TEA_API_KEY_ID` or `TEA_API_KEY_SECRET` in `env:`** of the calling
step, the job or the workflow. A composite action's steps inherit that
environment, so every step inside this action, third-party ones included (the
action uses `actions/cache`), would receive the secret. The action's first step
checks for those variables and refuses to run (exit 2, names only, never values)
if it finds a non-empty one.

- `extra-args` is refused if it contains `--api-key-id` or `--api-key-secret`
  (a secret on a command line shows up in process lists and logs).
- Only the final `libtea` step has the credentials. The steps that download and
  verify also unset both variables defensively.
- An input is masked in the logs only if its value comes from a secret (or is
  registered with `::add-mask::`); pass secrets, not literals.
- The action echoes only the command name, never argument values.
- Do not use `pull_request_target` with this action. It runs workflow code from
  the base branch with secrets while handling data from the pull request.
  Fork pull requests do not receive secrets under `pull_request`; skip the job
  for them as in the first example.

## What the action checks

1. **SHA-256.** `SHA256SUMS` is fetched from the release on every run. A fresh
   download must match it; a cached archive is used only if it matches too, so
   a poisoned cache entry is discarded and replaced. **This check is not
   independent of the archive:** `SHA256SUMS` comes from the same release as the
   archive, so anyone who can replace one can replace the other. It catches
   corruption and a poisoned cache, not a compromised release. The attestation
   below is the independent check; without it (`attestation: skip`, or `auto`
   on a runner without `gh`) the SHA-256 is all there is.
2. **Attestation.** With `gh` present (hosted runners have it), the archive's
   build-provenance attestation is verified against this repository and its
   `release.yml` workflow. A failure fails the step. Without `gh`, `auto`
   warns and continues on the SHA-256 check alone; `required` fails.

`docs/installing.md` says what these checks do and do not prove: integrity of
the download and that GitHub Actions built it from this repository, not that the
source is free of defects. Pin the action itself by commit SHA, and `version`
by number.

## Examples

Pull-request check that the lock still matches the server (forks skipped):

```yaml
name: tea-lock
on:
  pull_request:
permissions:
  contents: read
jobs:
  check:
    if: github.event.pull_request.head.repo.full_name == github.repository
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<commit-sha>
        with:
          persist-credentials: false
      - uses: Mehrn0ush/libtea-rs@<commit-sha>
        with:
          api-key-id: ${{ secrets.TEA_API_KEY_ID }}
          api-key-secret: ${{ secrets.TEA_API_KEY_SECRET }}
          version: "0.1.6"
          command: sync
          check: "true"
          tei: tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
          lock-file: tea.lock
```

Scheduled drift check (the job fails, and you are notified, when upstream
changes):

```yaml
name: tea-drift
on:
  schedule:
    - cron: "23 5 * * 1"
permissions:
  contents: read
jobs:
  drift:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<commit-sha>
        with:
          persist-credentials: false
      - uses: Mehrn0ush/libtea-rs@<commit-sha>
        with:
          api-key-id: ${{ secrets.TEA_API_KEY_ID }}
          api-key-secret: ${{ secrets.TEA_API_KEY_SECRET }}
          version: "0.1.6"
          command: sync
          check: "true"
          tei: tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
          lock-file: tea.lock
          extra-args: |
            --json
```

Refresh the lock on demand and show what changed (no write permission; open
the pull request yourself or with a tool you trust):

```yaml
name: tea-refresh
on:
  workflow_dispatch:
permissions:
  contents: read
jobs:
  refresh:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<commit-sha>
        with:
          persist-credentials: false
      - uses: Mehrn0ush/libtea-rs@<commit-sha>
        with:
          api-key-id: ${{ secrets.TEA_API_KEY_ID }}
          api-key-secret: ${{ secrets.TEA_API_KEY_SECRET }}
          version: "0.1.6"
          command: sync
          tei: tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
          lock-file: tea.lock
      - name: Show the change
        run: git diff -- tea.lock
```

Fetch one verified artifact (checksum integrity of the decoded bytes; exit 6
if it does not match, 4 if only SHA-1 checksums were declared):

```yaml
name: tea-sbom
on:
  workflow_dispatch:
permissions:
  contents: read
jobs:
  fetch:
    runs-on: ubuntu-latest
    steps:
      - uses: Mehrn0ush/libtea-rs@<commit-sha>
        with:
          api-key-id: ${{ secrets.TEA_API_KEY_ID }}
          api-key-secret: ${{ secrets.TEA_API_KEY_SECRET }}
          version: "0.1.6"
          command: download
          base-url: https://api.example.com
          extra-args: |
            --artifact
            11111111-1111-4111-8111-111111111111
            --revision
            1
            --media-type
            application/vnd.cyclonedx+json
            --require-checksums
            --output
            sbom.json
```

## Notes and limits

- The scripts are exercised against stubs on a Linux runner in CI. The Action
  itself was run from a workflow on GitHub-hosted `ubuntu-24.04` (gnu and
  musl), `ubuntu-24.04-arm`, `macos-15` and `windows-2022` runners, and in an
  `alpine:3.20` container with `libc: musl`, each with `parse-tei` and the
  attestation check set to `required` (skipped in the container, which has no
  `gh`). A job that sets `TEA_API_KEY_ID` in `env:` was refused as intended.
  The first run failed on Windows and in Alpine; both were fixed in 0.1.2.
- Commands that contact a server were run through the Action (`v0.1.5`) on
  2026-11-12, scratch-repository run 43160787117: `discover`,
  `product-release`, `sync` and `sync` with `check: "true"` against the
  public Vulnetix server on `ubuntu-24.04`, `macos-15` and `windows-2022`, and
  `discover` and `sync` against the public ReARM demo on `ubuntu-24.04`, all
  with `attestation: required`. All passed (`docs/interoperability.md`). A
  job that set `api-key-id` and `api-key-secret` from secrets ran the offline
  `spec` command: `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET` were set only in
  the environment of the step that runs `libtea` (the install and attestation
  steps had neither), they were masked, and the raw values do not occur in the
  log. That
  checks the plumbing only; no token was sent to a server, and no server that
  needs credentials has been tried.
- Runners: Linux, macOS and Windows (x86_64 and arm64 where the release has a
  build).
- The cache holds only the archive, keyed by version and target. It is
  re-verified on every run, so it saves a download, not a check.
- A self-hosted runner, or a job container, needs `bash`, `curl`, `tar` and
  `python3`. A minimal image such as Alpine has none of `bash` and `python3`:
  install them first (`apk add --no-cache bash curl tar python3`) and use
  `libc: musl`. Without `python3` the action stops with "python is needed to
  unpack the archive". `gh` is needed only for the attestation check.
- The action never installs anything from a package registry and never builds
  from source.
