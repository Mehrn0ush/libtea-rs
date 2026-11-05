use libtea_core::{Tei, WellKnownDocument};
use proptest::prelude::*;

proptest! {
    #[test]
    fn tei_parser_does_not_panic(input in ".{0,160}") {
        let _ = Tei::parse(&input);
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
