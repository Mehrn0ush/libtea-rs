use libtea_core::http_cache::{
    cache_key, decode_entry, encode_entry, parse_retry_after, CacheControl, CacheEntryMeta,
};
use proptest::prelude::*;

fn meta(key: &str) -> CacheEntryMeta {
    CacheEntryMeta::new(
        key,
        "https://tea.example.com/v1.0.0/x",
        200,
        10,
        false,
        &[("etag".into(), "\"e\"".into())],
    )
}

/// IMF-fixdate for a Unix time (Hinnant's civil-from-days).
fn imf(epoch: u64) -> String {
    let days = (epoch / 86_400) as i64;
    let rem = epoch % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days.rem_euclid(7)) as usize];
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month - 1) as usize];
    format!(
        "{weekday}, {day:02} {month_name} {year:04} {:02}:{:02}:{:02} GMT",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}

proptest! {
    #[test]
    fn decoding_arbitrary_bytes_never_panics_and_accepted_entries_round_trip(
        bytes in proptest::collection::vec(any::<u8>(), 0..512),
    ) {
        let key = cache_key("GET", "https://a/b", "https://a", None);
        if let Ok((meta, body)) = decode_entry(&bytes, &key, 1 << 20) {
            let again = encode_entry(meta.clone(), &body).unwrap();
            let (meta2, body2) = decode_entry(&again, &key, 1 << 20).unwrap();
            prop_assert_eq!(meta, meta2);
            prop_assert_eq!(body, body2);
        }
    }

    #[test]
    fn every_valid_entry_round_trips_and_any_truncation_is_refused(
        body in proptest::collection::vec(any::<u8>(), 0..2048),
        path in "[a-z0-9/]{0,30}",
        cut in 1usize..64,
    ) {
        let key = cache_key("GET", &format!("https://a/{path}"), "https://a", Some("id"));
        let bytes = encode_entry(meta(&key), &body).unwrap();
        let (m, b) = decode_entry(&bytes, &key, 1 << 20).unwrap();
        prop_assert_eq!(b, body);
        prop_assert_eq!(m.key, key.clone());
        let keep = bytes.len().saturating_sub(cut);
        prop_assert!(decode_entry(&bytes[..keep], &key, 1 << 20).is_err());
    }

    #[test]
    fn a_corrupted_body_is_always_refused(
        body in proptest::collection::vec(any::<u8>(), 1..512),
        at in any::<prop::sample::Index>(),
        flip in 1u8..=255,
    ) {
        let key = cache_key("GET", "https://a/b", "https://a", None);
        let mut bytes = encode_entry(meta(&key), &body).unwrap();
        let start = "LIBTEA-CACHE 1\n".len();
        let i = start + at.index(body.len());
        bytes[i] ^= flip;
        prop_assert!(decode_entry(&bytes, &key, 1 << 20).is_err());
    }

    #[test]
    fn cache_control_never_panics_and_no_store_is_never_missed(
        before in "[a-z, =0-9\"-]{0,30}",
        after in "[a-z, =0-9\"-]{0,30}",
    ) {
        let _ = CacheControl::parse(&[&before, &after]);
        let line = format!("{before},no-store,{after}");
        prop_assert!(CacheControl::parse(&[&line]).no_store);
        let split = [before.as_str(), "NO-STORE", after.as_str()];
        prop_assert!(CacheControl::parse(&split).no_store);
    }

    #[test]
    fn retry_after_seconds_and_dates_agree_with_the_clock(
        seconds in 0u64..1_000_000_000_000_000_000,
        epoch in 0u64..4_000_000_000,
        now in 0u64..4_000_000_000,
        junk in ".{0,40}",
    ) {
        prop_assert_eq!(parse_retry_after(&seconds.to_string(), now), Some(seconds));
        prop_assert_eq!(parse_retry_after(&imf(epoch), now), Some(epoch.saturating_sub(now)));
        let _ = parse_retry_after(&junk, now);
    }
}

/// A date-shaped value (29 bytes, `", "` at 3 and `" GMT"` at the end) with a
/// multi-byte character where an ASCII separator belongs is rejected, not a
/// char-boundary panic. `parse_retry_after` is public and takes any `&str`.
#[test]
fn retry_after_with_non_ascii_in_a_date_shaped_value_is_rejected_without_panicking() {
    let now = 1_700_000_000;
    // Each value is exactly 29 bytes; the two-byte character straddles a slice
    // boundary (byte 8, 12, 17 or 20, and so on).
    for value in [
        "Sun, 06\u{e9}No 1994 08:49:37 GMT",
        "Sun, 06 Nov\u{e9}994 08:49:37 GMT",
        "Sun, 06 Nov 1994\u{e9}8:49:37 GMT",
        "Sun, 06 Nov 1994 08\u{e9}9:37 GMT",
        "Sun, 06 Nov 1994 08:49\u{e9}7 GMT",
        "Sun, 0\u{e9}Nov 1994 08:49:37 GMT",
    ] {
        assert_eq!(value.len(), 29, "{value:?}");
        assert_eq!(parse_retry_after(value, now), None, "{value:?}");
    }
}
