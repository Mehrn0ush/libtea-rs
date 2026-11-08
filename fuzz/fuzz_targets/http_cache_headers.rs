#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::http_cache::{parse_retry_after, CacheControl};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut lines = text.split('\n');
    let first = lines.next().unwrap_or_default();
    let rest: Vec<&str> = lines.collect();
    let mut all = vec![first];
    all.extend(rest);
    let control = CacheControl::parse(&all);
    // The strictest directive wins: if any line says no-store, so does the result.
    if all.iter().any(|l| l.split(',').any(|p| p.trim().eq_ignore_ascii_case("no-store"))) {
        assert!(control.no_store);
    }
    let _ = parse_retry_after(first, 1_700_000_000);
});
