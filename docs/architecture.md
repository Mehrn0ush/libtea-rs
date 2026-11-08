# Architecture

```text
libtea-cli
  -> libtea-conformance (offline checks and reports)
  -> libtea-client     (HTTP, auth, streaming, retries)
       -> libtea-core  (TEI, versions, models, checksums, policy)
```

`libtea-core` has no Tokio, reqwest, filesystem, or process-global state. Parsing and validation can be used by another HTTP stack.

`libtea-client` owns the HTTP session, destination policy, token cache, and resource limits. `Service` is one versioned API base. `Client::discover` walks well-known endpoints and calls `/discovery` on each selected base.

`libtea-conformance` runs checks that do not contact a network and writes a JSON report. A passing report means the local checks passed. It does not certify a server.

`libtea-cli` calls the public crates. It does not reimplement TEI parsing or HTTP. Its commands are `spec`, `parse-tei`, `conformance`, `discover`, `product-release`, and `download`. Identifier filters are `ListParams` on `libtea-client` list methods. The CLI does not expose them.

## Request path

1. Parse the TEI and keep the original string.
2. Build `https://{domain}/.well-known/tea` from the domain spelling. The destination policy accepts or refuses that URL before a connection.
3. Parse the well-known document. Select mutually supported versions by SemVer precedence, then priority.
4. `GET {root}/v{advertised}/discovery?tei=...`. A TEA error stops the walk. A non-TEA failure tries the next selected endpoint.
5. Typed JSON models are validated after decoding. Unknown forward-compatible values are preserved.
6. Artifact downloads stream through a size cap, remove one layer of `Content-Encoding` (`gzip`, `deflate`, or `identity`), and hash the decoded bytes. The bearer token is attached only when the URL is under the versioned API prefix.

## Extensibility

- TEI types outside the registry parse and remain valid. Registry checks apply only to known types.
- Artifact types, update reasons, and other OpenAPI enums keep unknown strings.
- API versions other than `1.0.0` are selected when the caller lists them in `SupportedApi`.
- Checksum algorithms outside the registry are `ChecksumAlgorithm::Other` and produce `IntegrityOutcome::Unsupported`.

Well-known documents and discovery entries accept unknown fields at every level (tested in `crates/tea-core/tests/discovery_tolerance.rs`), although the upstream schemas set `additionalProperties: false`; the client is deliberately tolerant of what servers send. The lock file and cache entries are our own formats and reject unknown fields.

## Timeouts

Three limits bound a request (`Limits`):

| Limit | Default | Applies to |
| --- | --- | --- |
| `request_timeout` | 30 s | The whole of a request that is not an artifact download: discovery, JSON metadata, the token exchange. Headers and body. |
| `download_timeout` | 30 min | The whole of an artifact or signature `GET`, body included. |
| `idle_timeout` | 30 s | Any single wait for the next bytes of a response, for every request. |

`connect_timeout` (10 s) bounds connecting. An artifact that keeps arriving is
therefore not cut off after 30 s, which a single total timeout would do; a
transfer that stalls is cut off after `idle_timeout`, and no download runs longer
than `download_timeout`. Raise `download_timeout` for very large artifacts on
slow links. `idle_timeout` and `download_timeout` must not be zero
(`ClientBuilder::build` fails). Retries (`max_retries`) start a new attempt with
a fresh clock.

## Dependency direction

`libtea-core` does not depend on `libtea-client`. `libtea-client` does not depend on `libtea-cli` or `libtea-conformance`. The fuzz crate is excluded from the workspace so `cargo test --workspace` does not require `cargo-fuzz`.
