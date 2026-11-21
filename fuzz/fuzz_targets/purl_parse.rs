#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::purl::{same_package_url, Purl};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(purl) = Purl::parse(text) else {
        return;
    };
    // The original is kept; the canonical form is ASCII, parses, and is a
    // fixed point.
    assert_eq!(purl.as_str(), text);
    let canonical = purl.canonical();
    assert!(canonical.as_str().is_ascii());
    let reparsed = Purl::parse(canonical.as_str()).expect("canonical form parses");
    assert_eq!(reparsed.canonical(), canonical);
    assert!(same_package_url(&purl, &reparsed));
    // The type rules do not depend on how the PURL was spelled.
    assert_eq!(purl.check_type(), reparsed.check_type());
});
