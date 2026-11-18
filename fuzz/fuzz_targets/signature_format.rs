#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::signature_format::{
    detect_signature_format, SignatureFormat, MAX_SIGNATURE_INSPECT_BYTES,
};

fuzz_target!(|data: &[u8]| {
    let format = detect_signature_format(data);
    // Detection depends on the bytes only, so it is stable.
    assert_eq!(format, detect_signature_format(data));
    match format {
        SignatureFormat::Empty => assert!(data.is_empty()),
        SignatureFormat::TooLarge => assert!(data.len() > MAX_SIGNATURE_INSPECT_BYTES),
        _ => assert!(!data.is_empty() && data.len() <= MAX_SIGNATURE_INSPECT_BYTES),
    }
    // Trailing bytes after a complete binary packet or DER value do not
    // change the answer for those formats.
    if matches!(format, SignatureFormat::OpenPgpBinary | SignatureFormat::Cms)
        && data.len() < MAX_SIGNATURE_INSPECT_BYTES
    {
        let mut longer = data.to_vec();
        longer.push(0);
        assert_eq!(detect_signature_format(&longer), format);
    }
});
