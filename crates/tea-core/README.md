# cdx-tea-core

Independent Rust types and protocol rules for the [CycloneDX Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api). This crate does not open network connections.

It is not a release of the CycloneDX project. It is implemented against Transparency Exchange API commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`.

```toml
cdx-tea-core = "0.1.0"
```

Edition 2021. MSRV 1.85. License Apache-2.0.

The program below is an illustration. The example that builds in this crate is `examples/parse_tei.rs`.

```rust
use cdx_tea_core::Tei;

let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?;
```

HTTP is in [`cdx-tea-client`](https://crates.io/crates/cdx-tea-client). The workspace guide is at <https://github.com/Mehrn0ush/teaSDK>.
