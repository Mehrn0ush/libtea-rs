# Using libtea from agents and automation

This guide is for AI coding agents and scripts that call the `libtea` CLI or the
`libtea-core` / `libtea-client` crates. It is derived from the code and the
tests in this repository. If it disagrees with `libtea --help`, the help wins.

## When to use it

Use libtea to read a Transparency Exchange API (TEA) server: resolve a TEI,
fetch a product release or its collection, download an artifact and check its
declared checksums, and record the result in a lock file that CI can compare.

libtea does not run a server, publish, sign, verify signatures, scan for
vulnerabilities or decide whether a component is trustworthy.

## Calling the CLI without a terminal

- Every command is non-interactive. It never prompts.
- `libtea --version` prints `libtea <version>`; use it to check what is installed.
- Add `--json` where the command accepts it. Output is then JSON on stdout.
  A failure is one JSON object on stderr: `{"error":{"exit_code":5,"kind":"not_found","message":"..."},"schema_version":1}`.
- Field names and shapes are documented in `docs/cli-json.md`. The lock file
  schema is `docs/schemas/lock-v1.schema.json`.
- `download --json` requires `--output`, so stdout carries only the JSON.
- Branch on the exit status, not on message text. Messages can change.

## Exit status

The numbers are fixed by the CLI contract (`docs/cli-exit-codes.md`).

| Code | Name | Meaning |
| --- | --- | --- |
| 0 | success | The command finished. |
| 1 | failure | Network, TLS, timeout, size limit, server error, refused path, or any other failure. |
| 2 | usage | Bad command line or input, unreadable lock file, half-set credential pair. |
| 3 | auth | Credential missing or rejected, or HTTP 401/403. |
| 4 | weak-only | Every declared checksum matched but all are SHA-1. Nothing is written. |
| 5 | not found | HTTP 404, or the identifier did not resolve. |
| 6 | integrity | A checksum did not match, could not be computed, or none was declared with `--require-checksums`. Nothing is written. |
| 7 | changes detected | `sync --check` or `diff` found a difference. |

Exit 7 is a normal result for drift checks, not an error in libtea.

## Safe defaults

- Credentials: set `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET` in the
  environment. Reference them by name in scripts. Do not pass
  `--api-key-secret` (it shows up in process lists), and never print, log or
  commit a key or a bearer token.
- Destinations: by default only HTTPS to public addresses is allowed. Cleartext,
  loopback and metadata addresses are refused. Do not add `--allow-origin` for a
  server you do not control. Use it only for a local test server.
- Do not add `--allow-discovered-credentials` unless the discovery hosts are
  allowed to receive the key.
- Weak checksums: exit 4 means the artifact is not verified. Do not add
  `--allow-weak-checksums` to make a pipeline pass.
- Use `--require-checksums` when an artifact without a declared checksum must
  fail. Without it, a format with no checksum is reported but exits 0.
- Checksum integrity is not authenticity. A matching checksum shows the bytes
  equal what the server declared. It does not show who published them.
- `download --output` refuses an existing file unless `--force` is given. Do not
  add `--force` without a reason.

## Lock files in CI

- `libtea sync TEI --lock tea.lock` fetches the latest collection, checks every
  artifact format and writes a deterministic lock. It writes nothing if a check
  fails.
- `libtea sync TEI --lock tea.lock --check` writes nothing and exits 7 if the
  server differs from the lock. A checksum mismatch during `--check` is exit 6.
- `libtea diff old.lock new.lock` works offline and exits 7 if the two differ.
  The `fetchedAt` field is ignored.
- A lock records checksums seen at fetch time. It is not signed.
- `--cache-dir DIR` (or `LIBTEA_CACHE_DIR`) turns on a private response cache.
  `--offline` then answers only from it. `sync --check --offline` compares the
  lock with cached responses and says so; it does not contact the server.
- The GitHub Action (`docs/github-action.md`) runs these commands. Pass keys
  through its `api-key-id` and `api-key-secret` inputs, not through `env:`.

## Library use

- Build a client with `libtea_client::Client::builder()`. Certificate
  verification cannot be turned off.
- `IntegrityOutcome::Verified` means every declared checksum matched the decoded
  bytes. `Weak` (SHA-1 only) is an error unless `allow_weak_checksums(true)` is
  set. Treat `Weak` as not verified.
- Public structs and enums are `#[non_exhaustive]`: use constructors and
  `with_*` setters, and keep a wildcard arm when matching.
- The cache, offline mode, streaming downloads and lock API are experimental
  until 0.9 and may change in a minor release.

## What libtea does not do

- It does not verify signatures or establish who published an artifact.
- It does not follow every TEA server's quirks. Evidence of interoperability is
  limited to the deployments named in `docs/interoperability.md`.
- It does not claim conformance or certification.
- It does not fetch from a URL a server returns unless the destination policy
  allows it.
