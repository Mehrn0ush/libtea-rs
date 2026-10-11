# Architecture

```text
tea-cli
  -> tea-conformance (offline checks and reports)
  -> tea-client     (HTTP, auth, streaming, retries)
       -> tea-core  (TEI, versions, models, checksums, policy)
```

`tea-core` has no Tokio, reqwest, filesystem, or process-global state. Parsing and validation can be used by another HTTP stack.

`tea-client` owns the HTTP session, destination policy, token cache, and resource limits. `Service` is one versioned API base. `Client::discover` walks well-known endpoints and calls `/discovery` on each selected base.

`tea-conformance` runs checks that do not contact a network and writes a JSON report. A passing report means the local checks passed. It does not certify a server.

`tea-cli` calls the public crates. It does not reimplement TEI parsing or HTTP. Its commands are `spec`, `parse-tei`, `conformance`, `discover`, `product-release`, and `download`. Identifier filters are `ListParams` on `tea-client` list methods. The CLI does not expose them.

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

Well-known documents, discovery entries, and server-info objects reject unknown fields because their schemas set `additionalProperties: false`.

## Dependency direction

`tea-core` does not depend on `tea-client`. `tea-client` does not depend on `tea-cli` or `tea-conformance`. The fuzz crate is excluded from the workspace so `cargo test --workspace` does not require `cargo-fuzz`.
