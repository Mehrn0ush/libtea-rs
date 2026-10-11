#![no_main]

use libfuzzer_sys::fuzz_target;
use cdx_tea_core::version::{ApiVersion, OrderedPriority, VersionedEndpoint};
use cdx_tea_core::{select_endpoints, SupportedApi};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut versions = Vec::new();
    for piece in text.split(|c: char| c.is_whitespace() || c == ',').take(8) {
        if let Ok(version) = ApiVersion::parse(piece) {
            versions.push(version);
        }
    }
    if versions.is_empty() {
        return;
    }
    let endpoint = VersionedEndpoint {
        root_url: "https://api.example.com".into(),
        versions,
        priority: OrderedPriority::default_priority(),
    };
    let _ = select_endpoints(&[endpoint], &SupportedApi::tea_1_0_0());
});
