use proptest::prelude::*;

proptest! {
    #[test]
    fn lock_parse_never_panics_and_accepted_locks_re_serialise_identically(input in ".{0,300}") {
        if let Ok(lock) = libtea_core::LockFile::parse(input.as_bytes()) {
            let text = lock.to_json().unwrap();
            let again = libtea_core::LockFile::parse(text.as_bytes()).unwrap();
            prop_assert_eq!(again.to_json().unwrap(), text);
        }
    }

    #[test]
    fn lock_bytes_do_not_depend_on_entry_order(
        order in proptest::collection::vec(0usize..6, 6),
        revisions in proptest::collection::vec(1u64..4, 6),
    ) {
        use libtea_core::{LockFile, LockedArtifact, LockedChecksum, LockedIntegrity};
        let sum = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let entries: Vec<LockedArtifact> = (0..6usize)
            .map(|i| {
                LockedArtifact::new(
                    format!("{:08x}-1111-4111-8111-111111111111", i),
                    revisions[i],
                    "BOM",
                    "text/plain",
                    i as u64,
                    vec![LockedChecksum::new("SHA-256", sum)],
                    LockedIntegrity::Verified,
                    "2026-11-08T00:00:00Z",
                )
            })
            .collect();
        let build = |idx: &[usize]| {
            let mut lock = LockFile::new(
                "tei://example.com/uuid/2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
                "https://tea.example.com/",
                "1.0.0",
                "962a0228af1bfcc1232ac3f79239317ff33fdb04",
                "2ff3ae7c-8eab-4e0b-9b8b-1a0c4c4c1c2d",
                1,
            );
            lock.artifacts = idx.iter().map(|i| entries[*i].clone()).collect();
            lock
        };
        let natural: Vec<usize> = (0..6).collect();
        // `order` may repeat indexes; a lock with duplicates is rejected, not reordered.
        let mut shuffled: Vec<usize> = natural.clone();
        shuffled.sort_by_key(|i| order[*i] * 8 + *i);
        prop_assert_eq!(
            build(&natural).to_json().unwrap(),
            build(&shuffled).to_json().unwrap()
        );
        let mut doubled = natural.clone();
        doubled.push(0);
        prop_assert!(build(&doubled).to_json().is_err());
    }
}
