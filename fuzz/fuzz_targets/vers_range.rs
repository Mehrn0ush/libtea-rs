#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::vers::VersRange;

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    let Ok(text) = std::str::from_utf8(rest) else {
        return;
    };
    let Ok(range) = VersRange::parse(text) else {
        return;
    };
    // Re-encoding the parsed constraints gives a string that parses to the
    // same constraints.
    let mut rebuilt = format!("vers:{}/", range.version_type());
    if range.is_any() {
        rebuilt.push('*');
    }
    for (index, constraint) in range.constraints().iter().enumerate() {
        if index > 0 {
            rebuilt.push('|');
        }
        rebuilt.push_str(constraint.comparator().as_str());
        for byte in constraint.version().bytes() {
            if byte.is_ascii_alphanumeric() || b".-_~+:".contains(&byte) {
                rebuilt.push(char::from(byte));
            } else {
                rebuilt.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    // Escaping can triple the length; this module has no canonical form that
    // promises to fit the input limit, so only a rebuilt string that fits is
    // checked.
    if rebuilt.len() <= libtea_core::vers::MAX_VERS_BYTES {
        let again = VersRange::parse(&rebuilt).expect("rebuilt range parses");
        assert_eq!(again.constraints(), range.constraints());
    }
    // Evaluation never panics, for any version the selector picks.
    let version = semver::Version::new(u64::from(selector % 4), u64::from(selector / 4 % 4), 0);
    let _ = range.contains_semver(&version);
    let pypi = format!("{}.{}", selector % 4, selector / 4 % 4);
    let pypi = libtea_core::pep440::Pep440Version::parse(&pypi).expect("a PEP 440 version");
    let _ = range.contains_pypi(&pypi);
});
