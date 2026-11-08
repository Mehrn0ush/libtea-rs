#![no_main]

use libfuzzer_sys::fuzz_target;
use libtea_core::checksum::{decode_content_coding, ContentDecoder};

const CAP: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    let coding = match selector % 5 {
        0 => None,
        1 => Some("identity"),
        2 => Some("gzip"),
        3 => Some("deflate"),
        _ => Some("x-gzip"),
    };
    // One-shot and chunked decoding must agree and never exceed the cap.
    let whole = decode_content_coding(rest, coding, CAP);
    if let Ok(bytes) = &whole {
        assert!(bytes.len() <= CAP);
    }
    let mut decoder = match ContentDecoder::new(coding, CAP) {
        Ok(decoder) => decoder,
        Err(_) => return,
    };
    let step = usize::from(selector / 5).max(1);
    let mut chunked = Ok(());
    for chunk in rest.chunks(step) {
        chunked = decoder.push(chunk);
        if chunked.is_err() {
            break;
        }
    }
    let chunked = chunked.and_then(|()| decoder.finish());
    if let (Ok(a), Ok(b)) = (&whole, &chunked) {
        assert_eq!(a, b);
    }
    assert_eq!(whole.is_ok(), chunked.is_ok());

    // Draining after every chunk (the streaming download path) must agree with
    // the buffered result, and the cap must cover everything already taken.
    let mut drained = match ContentDecoder::new(coding, CAP) {
        Ok(decoder) => decoder,
        Err(_) => return,
    };
    let mut collected = Vec::new();
    let mut ok = true;
    for chunk in rest.chunks(step) {
        if drained.push(chunk).is_err() {
            ok = false;
            break;
        }
        collected.extend(drained.take_output());
        assert!(collected.len() <= CAP);
    }
    if ok {
        match drained.finish() {
            Ok(tail) => collected.extend(tail),
            Err(_) => ok = false,
        }
    }
    assert_eq!(ok, whole.is_ok());
    if let (true, Ok(expected)) = (ok, &whole) {
        assert_eq!(&collected, expected);
    }
});
