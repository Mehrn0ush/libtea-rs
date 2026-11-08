#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::version::{ApiVersion, OrderedPriority, VersionedEndpoint};
use libtea_core::{select_endpoints, SupportedApi};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut versions = Vec::new();
    for piece in text.split(|c: char| c.is_whitespace() || c == ',').take(8) {
        // Strict and lenient parsing must both be panic-free; lenient is a
        // superset, so keep whatever it accepts.
        let _ = ApiVersion::parse(piece);
        if let Ok(version) = ApiVersion::parse_lenient(piece) {
            versions.push(version);
        }
    }
    if versions.is_empty() {
        return;
    }
    let endpoint = VersionedEndpoint::new(
        "https://api.example.com",
        versions,
        OrderedPriority::default_priority(),
    );
    let endpoints = [endpoint];
    let _ = select_endpoints(&endpoints, &SupportedApi::tea_1_0_0());
    let _ = select_endpoints(&endpoints, &SupportedApi::tea_1_0_0().with_lenient(true));
});
