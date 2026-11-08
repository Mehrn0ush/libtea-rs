# Persistent cache and offline mode

*Experimental until 0.9.* Off by default. Library: `ClientBuilder::cache_dir(path)`
or `ClientBuilder::cache(CacheConfig)`, plus `ClientBuilder::offline(true)`.
CLI: `--cache-dir DIR` (or `LIBTEA_CACHE_DIR`), `--offline`,
`--cache-authenticated` on `discover`, `product-release`, `collections`,
`download` and `sync`; `libtea cache stat` and `libtea cache clear`.

## What it does

- Stores successful (`200`) `GET` responses: JSON metadata, the discovery
  documents, and artifact bytes (decoded). `HEAD`, `POST` (the token exchange)
  and requests with a caller-supplied `If-None-Match` are never cached.
- Revalidates on every online use with `If-None-Match` (`ETag`) and
  `If-Modified-Since` (`Last-Modified`). A `304` serves the stored body and
  restarts the entry's age. A `200` replaces it. A `404` or `410` removes it.
- **Offline** (`--offline`) answers only from the cache and opens no
  connection. A response that is not cached, or is older than the age limit,
  fails with `ClientError::Offline` (exit status 1). Anything that is not a
  cacheable `GET` also fails that way.
- **Does not** use `max-age` or `Expires` to skip a request: every online use
  revalidates. It does not serve a stale entry when the network is down
  (`stale-if-error` is not implemented); only `--offline` serves from the cache
  without asking.

## What is stored, and what is not

| | |
| --- | --- |
| Stored | `200` bodies and an allow-list of headers: `content-type`, `content-encoding`, `etag`, `last-modified`, `cache-control`, `content-location`, `vary`, `repr-digest` (each at most 8 KiB, visible ASCII). |
| Never stored | Any other header (`Set-Cookie`, `Authorization`, ...), request headers, credentials, the API key or its secret, bearer tokens, redirect targets (so a signed query in a `Location` is not kept), and the query string of the request URL in readable form (it is hashed into the key). |
| Not stored | `Cache-Control: no-store` (an existing entry is removed); `Vary: *`; non-`200` statuses; a body over `max_entry_bytes`. |
| Not stored by default | Responses to requests that carried credentials, and `Cache-Control: private` responses. `CacheConfig::with_authenticated(true)` (`--cache-authenticated`) allows both. |

**Key.** `SHA-256(method, normalized URL, SHA-256(origin, key id))`. The URL is
normalized by the URL parser (lowercase scheme and host, default port removed,
dot-segments resolved) with the fragment and any userinfo removed; the query is
kept as sent. The key id is the API key's *identifier*, hashed, never the
secret. Two keys, or two origins, never share an entry.

**Limits** (`CacheConfig`): 512 MiB total (oldest entries are removed first),
64 MiB per entry (larger responses are simply not cached), entries older than
7 days since their last successful revalidation are treated as absent. All are
ceilings; the defaults are not tuned.

## Entry format

One file per key, `v1/<key>.entry`:
`LIBTEA-CACHE 1\n` + body + metadata JSON + `\n` + a 10-digit metadata length +
`\n`. The metadata holds the key, the status, timestamps, whether the body is
decoded, the allow-listed headers, the body length, and the body's SHA-256. The
trailer layout lets a large body be written while it streams. Parsing is strict
(unknown fields, a different version, a wrong key, an out-of-range value, or a
length/hash mismatch refuse the entry). It is implemented in
`libtea_core::http_cache` with a proptest and the `http_cache_entry` fuzz target.

## Threat model: what the cache does and does not protect

The cache is **not a trust boundary**.

- *Anyone who can write the cache directory can serve you fake metadata.* The
  length and SHA-256 detect truncation and accidental corruption, not a
  deliberate edit: an attacker writes a consistent entry. The defence is the
  directory: it is created `0700` (entries `0600`) on Unix; a symbolic link, a
  non-directory, or a directory that group or others can access makes
  `ClientBuilder::build` fail (`ClientError::Io`); entry paths that turn out to
  be links or the wrong type are removed, never followed. On Windows there are
  no mode bits; rely on the profile directory's ACL.
- *Artifact bytes are re-checked on every use.* Whether they come from the
  network or the cache, the declared checksums are verified over the decoded
  bytes, so a forged artifact entry fails with `Integrity` (tested for buffered
  and streaming downloads, online with a `304` and offline). Note what that
  does and does not cover: the checksums themselves come from the artifact
  *metadata*, which may also be served from the cache. A cache writer who
  forges both is not detected. That is the limit of "never trusted" here.
- *Credentials stay out.* Nothing from a request is stored, and responses to
  credentialed requests are not stored unless enabled (then only under that
  key id's hash).
- *Damaged entries are absent.* A truncated, garbage, wrong-key, oversized,
  future-dated or hash-mismatched entry is removed and treated as a miss; the
  request goes to the network (or fails `Offline`) and a good entry replaces it.
- *Server text is escaped.* Header values that are not visible ASCII are never
  stored; the CLI prints cache information through the same control-character
  escaping as everything else.
- *Atomic, not durable.* Entries are written to a temporary file and renamed. They
  are not `fsync`ed, so a crash can lose an entry but not leave a half-written
  one in use; stray temporary files older than an hour are removed.

Several clients may share a directory; concurrent writers replace entries
atomically (tested).

## Streaming downloads

`Service::download_*_to_file` / `download_format_to_writer` keep a copy of the
decoded stream in a temporary cache file while writing the destination. The copy
becomes an entry only after the download passed its checksum policy; a failed or
cancelled download leaves nothing. A body over `max_entry_bytes` stops being
copied. Caching a large artifact therefore needs room for a second copy.

## `Retry-After` and `429`

Independent of the cache. A `GET` answered `429` or `503` is retried up to
`Limits::max_retries` times. If the server sent `Retry-After` (seconds or an
IMF-fixdate), the client waits that long when it is at most
`Limits::max_retry_after` (default 30 s); a longer request fails at once with
`ClientError::Throttled { status, retry_after_secs }` so the caller can decide.
Without the header the client backs off as before. After the retries run out the
error is `Throttled` too (it used to be `Transport("HTTP 503")` for a `503`).
The token `POST` is not retried.

## Commands

```sh
libtea product-release --base URL --uuid UUID --cache-dir ~/.cache/libtea
libtea sync TEI --lock tea.lock --cache-dir ~/.cache/libtea            # warm
libtea sync TEI --lock tea.lock --cache-dir ~/.cache/libtea --offline  # no network
libtea cache stat  --cache-dir ~/.cache/libtea [--json]
libtea cache clear --cache-dir ~/.cache/libtea [--json]
```

`cache clear` removes only files named like cache entries (a 64-hex key plus
`.entry`) and stray `*.tmp` files; other files and the directories stay.
`cache stat` counts entries and bytes by file, without decoding them.
Output shapes: `docs/cli-json.md`.
