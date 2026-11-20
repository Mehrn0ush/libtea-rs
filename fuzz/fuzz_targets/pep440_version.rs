#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::pep440::Pep440Version;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(version) = Pep440Version::parse(text) else {
        return;
    };
    assert_eq!(version.as_str(), text);
    // The normalized form parses to an equal version and is a fixed point.
    let normalized = version.to_string();
    let again = Pep440Version::parse(&normalized).expect("normalized form parses");
    assert_eq!(again, version);
    assert_eq!(again.to_string(), normalized);
    assert_eq!(again.cmp(&version), std::cmp::Ordering::Equal);
});
