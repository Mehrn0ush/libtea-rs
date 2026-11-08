# CLI exit codes

Stable for the `libtea` CLI (`libtea-cli`, unpublished). The numbers are part of
the CLI contract: a change needs a CHANGELOG entry. The same table is in
`libtea --help`. Tests: `crates/tea-cli/tests/json_and_exit_codes.rs`,
`crates/tea-cli/tests/output_file.rs`.

| Code | Name | When |
| --- | --- | --- |
| 0 | success | The command finished. For `download`, every declared checksum matched the decoded bytes, or none was declared and `--require-checksums` was not given (stderr says so). |
| 1 | failure | Anything not listed below: network or TLS failure, timeout, size limit, a server error status, the server asking to slow down (`429`/`503`, after the retries), `--offline` with the response not in the cache, an unsafe `--cache-dir`, a refused `--output` path (exists without `--force`, symbolic link, not a regular file), failed conformance smoke checks. |
| 2 | usage | Bad command line, `--offline` or `--cache-authenticated` without a cache directory, an unreadable, invalid or oversized lock file (including an unsupported `schemaVersion`), `sync --check` with no lock file, a TEI that resolves to several product releases without `--product-release` (clap reports these as 2), a malformed TEI or UUID, a half-set credential pair, `--json` on `download` without `--output`, an artifact with several formats and no `--media-type`. |
| 3 | auth | No credential where one is required, a rejected credential, `403`, a failed token exchange, or HTTP `401`/`403` from the API. |
| 4 | weak-only | Every declared checksum matched but all use weak algorithms (SHA-1) and `--allow-weak-checksums` was not given. Nothing is written. |
| 5 | not found | HTTP `404`, or `/discovery` did not resolve the identifier. A `404` from discovery is not evidence that no updates exist. |
| 6 | integrity | A checksum did not match (also during `sync` and `sync --check`; no lock is written), could not be computed, matched only in part, or `--require-checksums` was given and the format declares none. Nothing is written. |
| 7 | changes detected | `libtea sync --check` found that the server differs from the lock, or `libtea diff` found two locks that differ (`fetchedAt` is ignored). A checksum mismatch during `--check` is 6, not 7. |

Notes:

- Precedence when several apply: the first matching row of the failing
  operation, in the order the error arises. Integrity failures are never
  reported as 1.
- Network failures and destination-policy refusals (for example a redirect to
  another origin) are both 1. They are not separated yet.
- Code 0 for `download` is checksum integrity of the decoded bytes. It is not
  publisher authenticity, and no signature is checked.
- With `--json` a failure also prints one JSON object on stderr; see
  `docs/cli-json.md`. The exit code is the same.
