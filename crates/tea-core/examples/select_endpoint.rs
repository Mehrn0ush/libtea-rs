//! Select an API base from a `.well-known/tea` document.
//!
//! Run with `cargo run -p cdx-tea-core --example select_endpoint`.

use cdx_tea_core::{SupportedApi, WellKnownDocument};

fn main() {
    let document = WellKnownDocument::parse(
        br#"{
          "schemaVersion": 1,
          "endpoints": [
            {"url": "https://api.example.com", "versions": ["1.0.0"], "priority": 1},
            {"url": "https://api2.example.com/mytea", "versions": ["1.0.0"], "priority": 0.5}
          ]
        }"#,
    )
    .expect("well-known document");
    let selected = document
        .select(&SupportedApi::tea_1_0_0())
        .expect("supported version");
    let first = &selected[0];
    println!("{} /v{}", first.root_url, first.version);
}
