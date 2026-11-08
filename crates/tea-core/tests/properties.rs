use libtea_core::{Tei, WellKnownDocument};
use proptest::prelude::*;

proptest! {
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
