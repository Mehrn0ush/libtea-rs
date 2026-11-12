# libtea-rs

[![crates.io: libtea-core](https://img.shields.io/crates/v/libtea-core?label=libtea-core)](https://crates.io/crates/libtea-core)
[![crates.io: libtea-client](https://img.shields.io/crates/v/libtea-client?label=libtea-client)](https://crates.io/crates/libtea-client)
[![docs.rs: libtea-core](https://img.shields.io/docsrs/libtea-core?label=docs.rs%20core)](https://docs.rs/libtea-core)
[![docs.rs: libtea-client](https://img.shields.io/docsrs/libtea-client?label=docs.rs%20client)](https://docs.rs/libtea-client)
[![CI](https://github.com/Mehrn0ush/libtea-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Mehrn0ush/libtea-rs/actions/workflows/ci.yml?query=branch%3Amain)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

Independent Rust client libraries and a command-line tool for the [Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api) (TEA).

**Status:** 0.1.6. Releases 0.1.1 to 0.1.4 changed only documentation, CI and the GitHub Action; 0.1.5 added `libtea_client::core`, a re-export of `libtea-core`; 0.1.6 fixes a panic in `libtea_core::http_cache::parse_retry_after` and updates internal dependencies. Supported TEA versions, Rust versions and platforms, and how each is tested: [`docs/compatibility.md`](docs/compatibility.md). `libtea-core` and `libtea-client` are on crates.io and the `libtea` binaries are on GitHub Releases. Experimental until 0.9: expect breaking changes before 1.0. Written against TEA commit `962a0228af1bfcc1232ac3f79239317ff33fdb04` (OpenAPI 1.0.0; the TEA specification is still beta). Minimum supported Rust version: 1.89. License: Apache-2.0.

## What it is, and what it is not

libtea-rs is an independent client for TEA servers. It resolves a Transparency Exchange Identifier (TEI), fetches product releases, collections and artifacts, and checks the checksums a server declares. It can write a lock file so a CI job can tell when what a server publishes has changed.

It is not a TEA server, does not sign or publish anything, does not scan for vulnerabilities, and does not verify signatures. It is not an official CycloneDX or Ecma project, has no affiliation with them or with any TEA server vendor, and is not a conformance certificate. It has been tried against two deployments (see [docs/interoperability.md](docs/interoperability.md)); that is not universal compatibility.

| Crate | Role |
| --- | --- |
| `libtea-core` | TEI, version negotiation, models, checksums, destination rules, lock file format. No network code. |
| `libtea-client` | Async client (`tokio`, `reqwest`, `rustls`). |
| `libtea-cli` | The `libtea` command. Not published to crates.io. |
| `libtea-conformance` | Offline smoke checks used by `libtea conformance`. Not published. |

## Install

**Release binaries.** Each release has archives for seven targets (Linux gnu and musl on x86_64 and aarch64, macOS on x86_64 and aarch64, Windows x86_64), a `SHA256SUMS` file, a CycloneDX SBOM and a build-provenance attestation. Download from the [releases page](https://github.com/Mehrn0ush/libtea-rs/releases); [docs/installing.md](docs/installing.md) gives the checksum and attestation commands, which you should run before installing. Then `libtea --version` shows what you have.

**From source** (needs Rust 1.89 or later):

```sh
cargo install --git https://github.com/Mehrn0ush/libtea-rs --tag v0.1.6 --locked libtea-cli
```

This builds the `libtea` binary at the tagged release. Without `--tag` it builds the default branch. No checksum or attestation check is involved in this route. `libtea-cli` is not published to crates.io, so `cargo install libtea-cli` does not work.

**Libraries** from crates.io. The library example below uses `libtea-client` and `tokio`; the `libtea-core` types come through `libtea_client::core` (from 0.1.5; with 0.1.4 or older, also depend on `libtea-core`):

```toml
[dependencies]
libtea-client = "0.1.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

or `cargo add libtea-client && cargo add tokio --features macros,rt-multi-thread`.

## Quick start

These runs used a local test server on `127.0.0.1`. `--allow-origin` is needed only for that: by default the CLI refuses anything but HTTPS to public addresses.

Offline commands (`libtea --version` prints the installed version):

```sh
libtea spec
libtea parse-tei tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
```

```text
$ libtea spec
repository https://github.com/CycloneDX/transparency-exchange-api
branch main
commit 962a0228af1bfcc1232ac3f79239317ff33fdb04
commit_date 2026-10-08T06:28:26Z
retrieval_date 2026-10-10
spec_version 1.0.0
openapi 3.1.1
$ libtea parse-tei tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
domain example.com
type uuid
identifier 2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
well_known https://example.com/.well-known/tea
```

Resolve a TEI, then download one artifact and check it:

```sh
libtea discover tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
libtea download --base https://api.example.com --artifact 11111111-1111-4111-8111-111111111111 --revision 1 --media-type application/vnd.cyclonedx+json --output sbom.json
```

```text
integrity verified (decoded-byte checksums only; not publisher authenticity)	strong_etag false	bytes 57
```

`download` writes to a temporary file and moves it into place only after every declared checksum matched the decoded bytes. An existing output file is refused unless you pass `--force`. If all declared checksums are SHA-1 the command exits 4 and writes nothing, unless you pass `--allow-weak-checksums`. Add `--require-checksums` to fail when a format declares none.

### Lock files in CI

`sync` fetches the latest collection, checks every artifact format and writes a lock file ([docs/lock-file.md](docs/lock-file.md)). `sync --check` compares the server with the lock and writes nothing. `diff` compares two lock files offline.

```sh
libtea sync tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d --lock tea.lock
libtea sync tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d --lock tea.lock --check
libtea diff old.lock new.lock
```

```text
wrote tea.lock (1 artifact formats, collection 1, product release 2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d)
checksum integrity of decoded bytes only; a lock is not publisher authenticity
...
lock is up to date (1 artifact formats)
...
~ collectionVersion: 1 -> 2
~ 11111111-1111-4111-8111-111111111111 application/vnd.cyclonedx+json: checksums SHA-256:8bcd56b6... -> SHA-256:1285571e...
Error: the locks differ
```

A lock records the checksums seen at fetch time. It is not signed.

### JSON output and exit codes

Add `--json` for machine-readable output ([docs/cli-json.md](docs/cli-json.md)). With `--json`, a failure is one JSON object on stderr:

```text
{"error":{"exit_code":5,"kind":"not_found","message":"TEA API status 404"},"schema_version":1}
```

| Code | Meaning |
| --- | --- |
| 0 | success |
| 1 | failure (network, TLS, timeout, size limit, server error, refused path) |
| 2 | usage or bad input |
| 3 | authentication or authorization |
| 4 | only weak (SHA-1) checksums matched |
| 5 | not found or not resolved |
| 6 | integrity not established |
| 7 | changes detected (`sync --check`, `diff`) |

Details: [docs/cli-exit-codes.md](docs/cli-exit-codes.md).

### Cache and offline mode

Off by default and experimental. `--cache-dir DIR` (or `LIBTEA_CACHE_DIR`) keeps responses in a private directory and revalidates them on every online use. `--offline` answers only from the cache and never connects. A cached artifact is checked against its checksums on every use. See [docs/cache.md](docs/cache.md) for what the cache stores and its limits.

```text
$ libtea sync TEI --lock c.lock --check --offline --cache-dir c
the lock matches the cached responses (1 artifact formats); offline, the server was not contacted
```

### Credentials

Set `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET` in the environment. `--api-key-secret` works but exposes the secret in process lists and shell history. The key is sent only to the token endpoint of the API origin you chose; to send it to token endpoints learned from discovery you must pass `--allow-discovered-credentials`.

## Library example

A library call that resolves a TEI, downloads a format and checks the result. It is compiled as a doctest and marked `no_run` because it needs a server.

```rust,no_run
use libtea_client::{ArtifactFetch, Client};
use libtea_client::core::{IntegrityOutcome, Tei};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?;
    let client = Client::builder().build()?;
    let releases = client.discover(&tei).await?;
    let selected = &releases[0];
    let service = client.service(&selected.root_url, &selected.api_version)?;
    let collection = service
        .get_latest_product_release_collection(selected.product_release)
        .await?;
    let artifact = &collection.artifacts[0];
    let fetched = service
        .download_format(artifact, &artifact.formats[0].media_type, None)
        .await?;
    if let ArtifactFetch::Body(body) = fetched {
        assert_eq!(body.integrity.outcome, IntegrityOutcome::Verified);
    }
    Ok(())
}
```

`Verified` means every checksum declared for that format matched the **decoded** bytes. SHA-1-only is `Weak`, which the client treats as an error unless `allow_weak_checksums(true)` is set. Setting up an API key:

```rust,no_run
use libtea_client::{ApiKey, Client};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _client = Client::builder()
        .api_key(ApiKey::new("key-id", "key-secret"))
        .allow_discovered_credentials(true) // only when discovery hosts may receive the key
        .build()?;
    Ok(())
}
```

More examples that build: `crates/tea-client/examples/` and `crates/tea-core/examples/`. The pagination helper, the cache configuration and the lock file types are public items; see the rustdoc of `libtea-client` and `libtea-core`.

## GitHub Action

The repository has a composite action that downloads a release of `libtea`, verifies it, and runs one command. Its scripts are tested against stubs, and it has been run from a workflow on Linux (x86_64 and arm64, gnu and musl), macOS, Windows and an Alpine container. Use 0.1.2 or later; earlier tags fail the checksum check on Windows.

```yaml
- uses: Mehrn0ush/libtea-rs@<commit-sha-of-a-release-tag>
  with:
    version: "0.1.6"
    command: sync
    check: "true"
    tei: tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d
    lock-file: tea.lock
    api-key-id: ${{ secrets.TEA_API_KEY_ID }}
    api-key-secret: ${{ secrets.TEA_API_KEY_SECRET }}
```

Pass the key through the two `api-key-*` inputs. Do not put `TEA_API_KEY_*` in `env:` at the job or workflow level: steps inside a composite action inherit that environment, and the action refuses to run if it finds them. Pin the action by commit SHA. See [docs/github-action.md](docs/github-action.md).

## Security model

- **Destinations.** Only HTTPS to public addresses by default. Cleartext, loopback, private and cloud-metadata addresses are refused. `--allow-origin` (CLI) and `DestinationPolicy::allow_origin` (library) allow one named origin.
- **TLS.** rustls. Certificate verification cannot be turned off.
- **Redirects and credentials.** Redirects are followed manually up to a limit. The bearer token is attached only under the versioned API path of an authorized origin. Artifact and signature URLs never receive it.
- **Size limits.** Defaults: 1 MiB for discovery documents, 8 MiB for JSON, 256 MiB for an artifact, 5 redirects. Configurable through `Limits`.
- **Proxies.** `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY` are honored. Behind a proxy the proxy resolves hostnames, so the address checks do not apply; use `ClientBuilder::ignore_env_proxies(true)` where they matter.
- **What "verified" means.** Every checksum the server declared for an artifact matched the decoded bytes. This shows integrity relative to what the server declared. It is not publisher authenticity: a server that serves a wrong artifact with matching checksums passes.
- **Not verified.** Signatures (detached signature bytes can be downloaded; nothing checks them), who published an artifact, and the contents of an artifact.

Details and reasoning: [docs/security.md](docs/security.md). Report vulnerabilities as described in [SECURITY.md](SECURITY.md).

## Scope and non-goals

Not in 0.1.x: a publisher API; official CycloneDX or ECMA certification (the offline suite is a smoke test); signature verification (see `docs/design/signature-verification.md`); universal server compatibility; HTTPS SVCB lookup; OAuth beyond API-key `client_credentials`; publishing `libtea-cli` or `libtea-conformance` to crates.io.

## Documentation

- [docs/installing.md](docs/installing.md): verify and install the release binaries
- [docs/github-action.md](docs/github-action.md), [docs/lock-file.md](docs/lock-file.md), [docs/cache.md](docs/cache.md)
- [docs/cli-json.md](docs/cli-json.md), [docs/cli-exit-codes.md](docs/cli-exit-codes.md)
- [docs/auth.md](docs/auth.md), [docs/security.md](docs/security.md), [docs/architecture.md](docs/architecture.md)
- [docs/interoperability.md](docs/interoperability.md): what has been tested against which server
- [docs/release-checklist.md](docs/release-checklist.md), [docs/release-pins.md](docs/release-pins.md): for maintainers
- [SKILLS.md](SKILLS.md): a short guide for agents and scripts that call libtea
- [CHANGELOG.md](CHANGELOG.md)

## Building and testing

```sh
cargo build --workspace --locked
cargo test --workspace --all-targets --locked
```

## Issues

Report bugs and interoperability results as GitHub issues on [Mehrn0ush/libtea-rs](https://github.com/Mehrn0ush/libtea-rs). Do not post credentials or unfixed vulnerabilities.

## License

Apache-2.0. See [LICENSE](LICENSE).
