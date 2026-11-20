# libtea-cli

`libtea`, a command-line client for the [CycloneDX Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api) (TEA), built on `libtea-client` and `libtea-core`.

**Independent implementation.** Not an official CycloneDX project. No conformance
certification. No cryptographic signature verification. A checksum match is
integrity of the downloaded bytes, **not** publisher authenticity.

Implemented against Transparency Exchange API commit
`962a0228af1bfcc1232ac3f79239317ff33fdb04`. The TEA specification is still beta.

## Install

```sh
cargo install libtea-cli --locked
libtea --version
```

This builds from the source published on crates.io. Unlike the release
archives on [GitHub Releases](https://github.com/Mehrn0ush/libtea-rs/releases),
it involves no checksum file and no build-provenance attestation. The archives
and how to verify them are described in
[`docs/installing.md`](https://github.com/Mehrn0ush/libtea-rs/blob/main/docs/installing.md).

## Commands

`spec`, `parse-tei`, `conformance`, `discover` (a TEI, or a PURL on a known
base), `product-release`, `collections`, `cle`, `search`, `inspect`,
`download`, `sync`, `diff` and `cache`. `libtea --help` and
`libtea <command> --help` list the options.

```sh
libtea discover 'tei://demo.rearmhq.com/uuid/b9f115fb-6688-4a3f-a262-3acc04e1fc9e' --json
libtea inspect 'tei://demo.rearmhq.com/uuid/b9f115fb-6688-4a3f-a262-3acc04e1fc9e'
```

What the CLI promises to keep stable, as semver while 0.x, is its command
names, flags and exit codes
([`docs/cli-exit-codes.md`](https://github.com/Mehrn0ush/libtea-rs/blob/main/docs/cli-exit-codes.md)),
its `--json` output
([`docs/cli-json.md`](https://github.com/Mehrn0ush/libtea-rs/blob/main/docs/cli-json.md))
and the lock-file format
([`docs/lock-file.md`](https://github.com/Mehrn0ush/libtea-rs/blob/main/docs/lock-file.md)).
The crate has no Rust API.

Credentials come from `TEA_API_KEY_ID` and `TEA_API_KEY_SECRET`.

MSRV 1.89. License Apache-2.0. The CLI is versioned separately from the
libraries.
