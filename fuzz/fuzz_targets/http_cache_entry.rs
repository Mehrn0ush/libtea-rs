#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::http_cache::{cache_key, decode_entry, encode_entry};

fuzz_target!(|data: &[u8]| {
    // Decoding must never panic. An entry that decodes must re-encode to an
    // entry that decodes to the same metadata and body.
    let key = cache_key("GET", "https://tea.example.com/v1.0.0/x", "https://tea.example.com", None);
    if let Ok((meta, body)) = decode_entry(data, &key, 1 << 20) {
        let again = encode_entry(meta.clone(), &body).expect("a decoded entry re-encodes");
        let (meta2, body2) = decode_entry(&again, &key, 1 << 20).expect("canonical form decodes");
        assert_eq!(meta, meta2);
        assert_eq!(body, body2);
    }
});
