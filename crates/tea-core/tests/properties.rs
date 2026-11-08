use libtea_core::{Tei, WellKnownDocument};
use proptest::prelude::*;

proptest! {
    #[test]
    fn draining_a_content_decoder_matches_one_shot_decoding(
        raw in proptest::collection::vec(any::<u8>(), 0..4096),
        selector in 0u8..3,
        step in 1usize..257,
        cut in proptest::option::of(1usize..32),
    ) {
        use std::io::Write;
        use libtea_core::checksum::{decode_content_coding, ContentDecoder};
        const CAP: usize = 1 << 16;
        let (coding, mut body) = match selector {
            0 => (None, raw.clone()),
            1 => {
                let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
                e.write_all(&raw).unwrap();
                (Some("gzip"), e.finish().unwrap())
            }
            _ => {
                let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
                e.write_all(&raw).unwrap();
                (Some("deflate"), e.finish().unwrap())
            }
        };
        if let Some(cut) = cut {
            let keep = body.len().saturating_sub(cut);
            body.truncate(keep);
        }
        let whole = decode_content_coding(&body, coding, CAP);
        let mut decoder = ContentDecoder::new(coding, CAP).unwrap();
        let mut collected = Vec::new();
        let mut ok = true;
        for chunk in body.chunks(step) {
            if decoder.push(chunk).is_err() {
                ok = false;
                break;
            }
            collected.extend(decoder.take_output());
        }
        if ok {
            match decoder.finish() {
                Ok(tail) => collected.extend(tail),
                Err(_) => ok = false,
            }
        }
        prop_assert_eq!(ok, whole.is_ok());
        if let Ok(expected) = whole {
            prop_assert_eq!(collected, expected);
        }
    }

    #[test]
    fn tei_parser_does_not_panic(input in ".{0,160}") {
        let _ = Tei::parse(&input);
    }

    #[test]
    fn tei_identifier_validation_does_not_panic(tei_type in "[a-z]{1,6}", id in "\\PC{0,24}") {
        let text = format!("tei://example.com/{tei_type}/{id}");
        if let Ok(tei) = Tei::parse(&text) {
            let _ = tei.validate_registry();
            let _ = tei.publication_issues();
        }
        let percent = format!("tei://example.com/{tei_type}/%{id}");
        let _ = Tei::parse(&percent);
    }

    #[test]
    fn parsed_tei_displays_the_original_bytes(label in "[a-z][a-z0-9]{0,8}", id in "[a-z0-9]{1,20}") {
        let text = format!("tei://example.com/{label}/{id}");
        let parsed = Tei::parse(&text).unwrap();
        prop_assert_eq!(parsed.as_str(), text.as_str());
        prop_assert_eq!(parsed.to_string(), text);
    }

    #[test]
    fn discovery_json_does_not_panic(input in ".{0,200}") {
        let _ = WellKnownDocument::parse(input.as_bytes());
    }
}
