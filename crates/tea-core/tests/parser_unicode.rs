//! Every public parser in `libtea-core` must return `Ok` or `Err` on arbitrary
//! Unicode input, never panic.
//!
//! Inputs mix free text (`any::<String>()`, all Unicode scalar values) with
//! structured prefixes so the parsers get past their first syntax check:
//! `tei://` forms, `%` escapes next to multi-byte characters, and JSON
//! documents whose string fields are arbitrary Unicode.

use libtea_core::checksum::{decode_content_coding, ContentDecoder};
use libtea_core::enums::{
    ArtifactType, ChecksumAlgorithm, CleEventType, CollectionOwner, ComplianceDocumentType,
    IdentifierType, TeaErrorCode,
};
use libtea_core::{
    encode_query_component, is_json_content_type, parse_api_base, parse_artifact, parse_collection,
    parse_component_release, parse_product_release, tea_error_code, versioned_api_prefix,
    ApiVersion, ArtifactId, Checksum, DiscoveryInfo, ProductReleaseId, SupportedApi, TeaErrorBody,
    TeaTimestamp, TeaUuid, Tei, TokenResponse, WellKnownDocument,
};
use proptest::prelude::*;

/// Free text that leans on the characters that break byte-index slicing.
fn tricky() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<String>(),
        proptest::collection::vec(
            prop_oneof![
                Just("%".to_owned()),
                Just("%2F".to_owned()),
                Just("%4".to_owned()),
                Just("\u{e9}".to_owned()),
                Just("\u{20ac}".to_owned()),
                Just("\u{1f600}".to_owned()),
                Just("\u{200d}".to_owned()),
                Just("/".to_owned()),
                Just(":".to_owned()),
                Just("T".to_owned()),
                Just("Z".to_owned()),
                Just("-".to_owned()),
                Just(".".to_owned()),
                "[0-9a-f]{1,4}",
                any::<char>().prop_map(String::from),
            ],
            0..24,
        )
        .prop_map(|parts| parts.concat()),
    ]
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn tei_forms(domain in tricky(), tei_type in tricky(), id in tricky()) {
        for text in [
            format!("tei://{domain}/{tei_type}/{id}"),
            format!("tei://example.com/{tei_type}/{id}"),
            format!("tei://example.com/uuid/{id}"),
            format!("tei://example.com/purl/{id}"),
            format!("tei://example.com/hash/{id}"),
            format!("tei://example.com/gtin/{id}"),
            format!("tei://example.com/x/%{id}"),
            id.clone(),
        ] {
            if let Ok(tei) = Tei::parse(&text) {
                let _ = tei.validate_registry();
                let _ = tei.publication_issues();
                let _ = tei.well_known_url();
                prop_assert_eq!(tei.as_str(), text.as_str());
            }
        }
    }

    #[test]
    fn identifiers_and_timestamps(text in tricky()) {
        let _ = TeaUuid::parse(&text);
        let _ = ProductReleaseId::parse(&text);
        let _ = ArtifactId::parse(&text);
        let _ = TeaTimestamp::parse(&text);
        let _ = TeaTimestamp::parse(&format!("2024-01-01T{text}Z"));
        let _ = TeaTimestamp::parse(&format!("2024-01-01T00:00:00.{text}Z"));
    }

    #[test]
    fn versions(text in tricky()) {
        let _ = ApiVersion::parse(&text);
        let _ = SupportedApi::from_versions([text.as_str()]);
        let _ = versioned_api_prefix("https://tea.example", &text);
        let _ = versioned_api_prefix(&text, "1.0.0");
    }

    #[test]
    fn urls_and_query_helpers(text in tricky()) {
        let _ = parse_api_base(&text);
        let _ = parse_api_base(&format!("https://{text}"));
        let _ = parse_api_base(&format!("https://tea.example/{text}"));
        let _ = encode_query_component(&text);
        let _ = is_json_content_type(Some(&text));
        let _ = tea_error_code(Some(&text), text.as_bytes());
    }

    #[test]
    fn wire_enums_round_trip(text in tricky()) {
        prop_assert_eq!(ChecksumAlgorithm::from_wire(&text).as_str().to_owned(), text.clone());
        prop_assert_eq!(IdentifierType::from_wire(&text).as_str().to_owned(), text.clone());
        prop_assert_eq!(ArtifactType::from_wire(&text).as_str().to_owned(), text.clone());
        prop_assert_eq!(CollectionOwner::from_wire(&text).as_str().to_owned(), text.clone());
        prop_assert_eq!(CleEventType::from_wire(&text).as_str().to_owned(), text.clone());
        prop_assert_eq!(TeaErrorCode::from_wire(&text).as_str().to_owned(), text.clone());
        prop_assert_eq!(ComplianceDocumentType::from_wire(&text).as_str().to_owned(), text.clone());
        let _ = ChecksumAlgorithm::from_wire(&text).is_weak();
    }

    #[test]
    fn checksums(alg in tricky(), value in tricky()) {
        let _ = Checksum::new(ChecksumAlgorithm::from_wire(&alg), &value);
        let _ = Checksum::new(ChecksumAlgorithm::Sha256, &value);
    }

    #[test]
    fn json_documents_with_unicode_fields(text in tricky()) {
        let s = json_string(&text);
        let _ = WellKnownDocument::parse(text.as_bytes());
        let _ = WellKnownDocument::parse(
            format!(r#"{{"schemaVersion":1,"endpoints":[{{"url":{s},"versions":[{s}]}}]}}"#).as_bytes(),
        );
        let _ = DiscoveryInfo::parse_list(
            format!(r#"[{{"productReleaseUuid":{s},"servers":[{{"rootUrl":{s},"versions":[{s}]}}]}}]"#).as_bytes(),
        );
        let _ = TeaErrorBody::parse(format!(r#"{{"error":{s},"message":{s}}}"#).as_bytes());
        let _ = serde_json::from_str::<TokenResponse>(
            &format!(r#"{{"access_token":{s},"token_type":{s},"expires_in":1}}"#),
        )
        .map(TokenResponse::bearer);
        let artifact = format!(
            r#"{{"uuid":{s},"version":1,"type":{s},"createdDate":{s},"formats":[{{"mediaType":{s},"checksums":[{{"algType":{s},"algValue":{s}}}],"url":{s},"signatureUrl":{s}}}]}}"#
        );
        let _ = parse_artifact(artifact.as_bytes());
        let _ = parse_artifact(text.as_bytes());
        let collection = format!(
            r#"{{"uuid":{s},"version":1,"createdDate":{s},"belongsTo":{s},"updateReason":{{"type":{s},"comment":{s}}},"artifacts":[]}}"#
        );
        let _ = parse_collection(collection.as_bytes());
        let release = format!(
            r#"{{"uuid":{s},"product":{s},"version":{s},"createdDate":{s},"components":[],"latestCollection":{collection}}}"#
        );
        let _ = parse_product_release(release.as_bytes());
        let _ = parse_component_release(release.as_bytes());
    }

    #[test]
    fn content_decoding(bytes in proptest::collection::vec(any::<u8>(), 0..256), coding in tricky()) {
        for name in [Some(coding.as_str()), Some("gzip"), Some("deflate"), Some("identity"), None] {
            if let Ok(decoded) = decode_content_coding(&bytes, name, 1024) {
                prop_assert!(decoded.len() <= 1024);
            }
            if let Ok(mut decoder) = ContentDecoder::new(name, 1024) {
                for chunk in bytes.chunks(7) {
                    if decoder.push(chunk).is_err() {
                        break;
                    }
                }
            }
        }
    }
}
