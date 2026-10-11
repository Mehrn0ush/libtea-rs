#![no_main]

use libfuzzer_sys::fuzz_target;
use cdx_tea_core::{tea_error_code, DiscoveryInfo, WellKnownDocument};

fuzz_target!(|data: &[u8]| {
    let _ = WellKnownDocument::parse(data);
    let _ = DiscoveryInfo::parse_list(data);
    let _ = tea_error_code(Some("application/json"), data);
});
