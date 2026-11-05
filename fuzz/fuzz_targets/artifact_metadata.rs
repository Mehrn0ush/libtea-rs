#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::parse_artifact;

fuzz_target!(|data: &[u8]| {
    let _ = parse_artifact(data);
});
