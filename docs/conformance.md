# Conformance

`tea-conformance` runs checks that use the pinned specification examples and rules. The checks do not contact a network.

```sh
cargo test -p tea-conformance
cargo run -p tea-cli -- conformance
```

The report includes the specification commit and version and one result per case. A passing report means those local checks passed. It is not a statement that this SDK is a conforming TEA server, and it is not a statement that any particular server is conformant.

## What the static suite covers

- Registry TEI examples parse, validate, and keep their spelling
- Case variants of a TEI are not equal
- The normative well-known example selects the higher-priority equal version
- An empty discovery array is rejected
- An unknown `error` string is still recognized as a TEA error
- A wrong SHA-256 is not reported as verified

## What the HTTP tests cover

The tests in `tea-client` use a local mock server. They cover production destination refusal, JSON size limits, a product read, TEA `404` recognition, failover from a non-TEA `404`, one API-key exchange after a bearer challenge, redirect stripping of `Authorization`, and checksum verification of decoded bytes.

## What is not claimed

No public TEA server was used as a conformance oracle. Passing these tests does not imply interoperability with ReARM, Oolong, pypi-tea, sbomify, or any other implementation. See [interoperability.md](interoperability.md).
