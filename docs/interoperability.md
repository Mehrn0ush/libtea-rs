# Interoperability

No live interoperability run was executed for this repository. The notes below record what the public implementation list says and what was not tested. A server that answers this SDK is not, by that fact, a conforming TEA server. This SDK is not a conformance certificate.

## Implementations named by the specification

The pinned `doc/tea-implementations.md` lists:

| Name | Role described there | What this repository verified |
| --- | --- | --- |
| py-libtea | Python library | Not executed |
| pypi-tea | Service in front of PyPI | Not executed |
| ReARM CLI | Command line client | Not executed |
| Vulnetix | Client, described as implementing TEA v0.4.0 | Not executed. The pinned OpenAPI version is 1.0.0, so that claim is a different baseline |
| Oolong | Java client and server | Not executed |
| ReARM server | Server | Not executed |
| sbomify | Server | Not executed |

The same document gives `https://us.sbom.cybeats.com/.well-known/tea` as a Cybeats demonstration document. This SDK was not run against it.

## Optional ignored live check

There is no ignored test that calls a public server. Adding one later should stay behind an environment variable, should print the server and the specification commit, and should not be part of the default `cargo test` run.

## Behavior that can affect other implementations

- Query values follow the OpenAPI example and encode `/` as `%2F`. Servers that decode the query once should accept this.
- The version path uses the advertised string, including build metadata, with `+` left unencoded.
- Unknown TEA error codes do not fail over.
- HTTP and non-public addresses are refused unless that origin is explicitly allowed.
- Checksums are calculated after `gzip` or `deflate` content coding is removed.
