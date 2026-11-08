# GitHub Action

> **Status.** Written, tested against stubs, **not run on GitHub**. It installs a
> released `libtea` binary, and no release exists yet (`docs/installing.md`).
> Its one third-party dependency, `actions/cache`, is a placeholder pin until
> `docs/release-pins.md` is followed. Treat the examples as the intended use,
> not as something exercised end to end.

The action (`action.yml` at the repository root) downloads the `libtea` release
for the runner, checks it, caches it, and runs one command. Use it to keep a
lock file honest in CI:

```yaml
- uses: Mehrn0ush/libtea-rs@<commit-sha-of-a-release-tag>
  with:
    version: "0.1.0"
    command: sync
    tei: tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
    lock-file: tea.lock
    check: "true"
```

## Inputs

| Input | Meaning |
| --- | --- |
| `version` (required) | The libtea version to install, for example `0.1.0`. There is no `latest`: pin it. |
| `command` (required) | `sync`, `diff`, `discover`, `download`, `product-release`, `collections`, `parse-tei`, `spec` or `conformance`. |
| `tei` | A `tei://` identifier, for `sync`, `discover`, `parse-tei`. |
| `lock-file` | Lock path for `sync`; the old lock for `diff`. |
| `lock-file-new` | The new lock for `diff`. |
| `base-url` | API base for `download`, `product-release`, `collections` (becomes `--base`). |
| `check` | `"true"` runs `sync --check`: exit 7 when the server differs from the lock. |
| `api-key-id`, `api-key-secret` | The API key, from secrets; see Credentials. Optional for servers that need no key. |
| `extra-args` | More `libtea` arguments, **one per line**, passed as-is with no shell (so `--artifact`, UUID, `--output`, `--allow-origin`, `--json`, ...). |
| `working-directory` | Directory to run in. |
| `libc` | Linux only: `gnu` (default) or `musl`. |
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
   a poisoned cache entry is discarded and replaced.
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
          version: "0.1.0"
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
          version: "0.1.0"
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
          version: "0.1.0"
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
          version: "0.1.0"
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

- Runners: Linux, macOS and Windows (x86_64 and arm64 where the release has a
  build). Windows path handling in the scripts is untested.
- The cache holds only the archive, keyed by version and target. It is
  re-verified on every run, so it saves a download, not a check.
- A self-hosted runner needs `bash`, `curl` and `python3`.
- The action never installs anything from a package registry and never builds
  from source.
