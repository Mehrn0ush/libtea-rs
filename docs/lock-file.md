# Lock file (`schemaVersion` 1)

*Experimental until 0.9.* Written by `libtea sync`, compared by
`libtea sync --check` and `libtea diff`. The schema is
`docs/schemas/lock-v1.schema.json`; `crates/tea-cli/tests/lock_schema.rs` fails
if the schema and the library disagree on key names or requiredness.

## What it is, and what it is not

A lock records what a TEI resolved to and the **checksum-integrity** outcome of
every artifact format at the time it was fetched. It is not signed. Whoever can
edit the file can edit the record, so commit it to a place where changes are
reviewed. A matching lock does not show that the publisher is who they claim to
be: no signature is checked, and `Verified` means declared digests matched the
decoded bytes.

It holds no secrets. `server` is the API root with userinfo, query and
fragment removed, and the library refuses to write or read a lock whose
`server` has any of them. Credentials are never written.

## Layout

```json
{
  "schemaVersion": 1,
  "tei": "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
  "server": "https://tea.example.com/",
  "apiVersion": "1.0.0",
  "specCommit": "962a0228af1bfcc1232ac3f79239317ff33fdb04",
  "productRelease": "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
  "collectionVersion": 2,
  "artifacts": [
    {
      "uuid": "11111111-1111-4111-8111-111111111111",
      "revision": 1,
      "type": "BOM",
      "mediaType": "application/vnd.cyclonedx+json",
      "size": 11,
      "checksums": [{"algorithm": "SHA-256", "value": "..."}],
      "integrity": "Verified",
      "fetchedAt": "2026-11-08T00:00:00Z"
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `tei` | The TEI that was resolved, as given on the command line. |
| `server` | The API root that served the release (redacted as above). |
| `apiVersion` | The advertised API version that was used. |
| `specCommit` | The TEA specification commit this client is pinned to. |
| `productRelease` | Product release UUID. |
| `collectionVersion` | The collection version that was locked (the latest unless `--collection-version`). |
| `artifacts[]` | One entry per artifact **format**, unique by `(uuid, mediaType)`. |
| `uuid`, `revision` | The artifact and its revision. |
| `type` | The artifact type as published (for example `BOM`). |
| `mediaType` | The format's media type. |
| `size` | Decoded bytes, counted while downloading. |
| `checksums` | The declared checksums (`algorithm`, lowercase hex `value`), sorted. |
| `integrity` | `Verified` (every declared checksum matched, at least one is not weak), `VerifiedWeakOnly` (all matched but all are SHA-1 and `--allow-weak-checksums` was given), or `NotDeclared` (the format declares none, so nothing was checked). A failed or partial check never reaches a lock: `sync` fails instead. |
| `fetchedAt` | RFC 3339 UTC with `Z`. Ignored when locks are compared. |

## Determinism

- Artifacts are sorted by `uuid`, then `mediaType`; checksums by algorithm,
  then value. Input order never changes the bytes.
- Keys are in a fixed order, two-space indent, one trailing newline.
- `sync` leaves an up-to-date lock untouched (it does not even refresh
  `fetchedAt`), so a re-run produces no version-control diff. Pass
  `--fetched-at` for a reproducible timestamp.
- Writes are atomic: a finished temporary file in the same directory is renamed
  over the lock. The temporary file has an unpredictable name and is created
  with `create_new`, so a file or link planted there, or left by a killed run,
  cannot make `sync` fail or redirect the write. A symbolic link at the lock
  path is refused.

## Text in the lock

`tei`, `apiVersion`, each artifact `type` and `mediaType`, and each checksum
algorithm name are bounded (2048, 64, 128, 255 and 64 characters), must not be
empty, and must be one line free of control characters (C0, DEL, C1, bidi
overrides). `server` supplies most of this text, so a server that sends a
control character or an absurd length makes `sync` fail (exit 1) without
writing a lock, and a hand-edited lock that has one is refused on reading.

## Reading

Unknown fields, an unsupported `schemaVersion`, duplicate
`(uuid, mediaType)` entries, malformed identifiers or timestamps, invalid
checksums, and a `server` that could carry a secret are all errors (`diff`
and `sync` exit 2). Unknown fields are rejected rather than ignored because
the format is version-gated: ignoring a field a newer writer added would drop
it on the next write. A lock larger than 16 MiB is refused.

## Commands

```sh
libtea sync <TEI> --lock tea.lock                 # resolve, fetch, verify, write
libtea sync <TEI> --lock tea.lock --check         # exit 7 if the server differs
libtea sync <TEI> --lock tea.lock --artifacts-dir store/   # also keep the bytes
libtea diff old.lock new.lock                     # offline; exit 7 if they differ
```

- `sync` resolves the TEI like `discover`, then takes the latest collection of
  the product release and downloads **every format** of every artifact in it,
  streaming and hashing (`docs/cli-exit-codes.md` for statuses). If the TEI
  resolves to several product releases, pass `--product-release`.
- It fails closed. If any artifact cannot be fetched and checked, nothing is
  written and an existing lock is left as it was. A checksum mismatch is exit 6,
  also under `--check` (tampering is not reported as a diff).
- `--check` fetches and verifies too, because size and integrity are properties
  of the bytes. It never writes. Differences are printed (text or `--json`,
  see `docs/cli-json.md`) and the exit status is 7.
- `--artifacts-dir` keeps each verified format as
  `<uuid>-r<revision>-<media type with unsafe characters replaced by _>-<8 hex>`,
  where the last part hashes the exact media type, so `application/x+y` and
  `application/x_y` get different files. The name is never taken from the
  server; `sync` refuses to write two formats to one name; existing files are
  replaced. Files are private (mode 0600 on Unix).
- Artifacts are fetched one after another.

## What is compared

`diff` and `--check` compare the header (`tei`, `server`, `apiVersion`,
`specCommit`, `productRelease`, `collectionVersion`) and, per `(uuid,
mediaType)`: `revision`, `type`, `size`, `checksums`, `integrity`. A new
revision of an artifact is a change, not a removal plus an addition.
`fetchedAt` is ignored.
