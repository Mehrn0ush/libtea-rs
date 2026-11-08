#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::LockFile;

fuzz_target!(|data: &[u8]| {
    // Parsing must never panic. A lock that parses must serialise to a form
    // that parses again and serialises to the same bytes.
    if let Ok(lock) = LockFile::parse(data) {
        let text = lock.to_json().expect("a parsed lock is valid");
        let again = LockFile::parse(text.as_bytes()).expect("canonical form parses");
        assert_eq!(again.to_json().unwrap(), text);
        assert!(libtea_core::diff_locks(&lock, &again).is_empty());
    }
});
