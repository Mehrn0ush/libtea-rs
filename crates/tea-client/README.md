# cdx-tea-client

Independent async Rust client for the [CycloneDX Transparency Exchange API](https://github.com/CycloneDX/transparency-exchange-api).

It is not a release of the CycloneDX project. It is implemented against Transparency Exchange API commit `962a0228af1bfcc1232ac3f79239317ff33fdb04`.

Publish `cdx-tea-core` 0.1.0 before this crate. This crate depends on that version and does not depend on the unpublished CLI or conformance crates.

```toml
cdx-tea-client = "0.1.0"
```

Edition 2021. MSRV 1.85. License Apache-2.0. TLS uses rustls. Certificate verification stays enabled. The default destination policy allows HTTPS to public addresses.

The program below is an illustration. The examples that build in this crate are `examples/filter_by_identifier_type.rs` and `examples/paginate_components.rs`.

```rust
use cdx_tea_client::Client;
use cdx_tea_core::enums::IdentifierType;
use cdx_tea_core::ListParams;

let client = Client::builder().build()?;
let service = client.service("https://api.example.com", "1.0.0")?;
let query = ListParams {
    id_type: Some(IdentifierType::ComplianceDocument),
    ..ListParams::default()
};
let page = service.query_components(&query).await?;
```

`ListParams::default()` leaves every field unset and does not require the sort type to implement `Default`. A local HTTP origin needs `DestinationPolicy::allow_origin` for that origin only. The workspace guide is at <https://github.com/Mehrn0ush/teaSDK>.
