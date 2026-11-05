#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::Tei;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        if let Ok(tei) = Tei::parse(text) {
            let _ = tei.validate_registry();
            let _ = tei.publication_issues();
            assert_eq!(tei.as_str(), text);
        }
    }
});
