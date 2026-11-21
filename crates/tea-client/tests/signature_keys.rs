//! `KeyVerifier` against signatures made by OpenSSL
//! (`tests/fixtures/signature/README.md`).
#![cfg(feature = "signature-keys")]

use std::path::{Path, PathBuf};

use libtea_client::signature::{
    KeyAlgorithm, KeyVerifier, PublicKey, SignatureVerifier, SignatureVerifyOutcome, SignedContent,
    VerifyRequest,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/signature")
        .join(name)
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(fixture(name)).unwrap()
}

fn key(name: &str) -> PublicKey {
    PublicKey::parse(&read(name)).unwrap()
}

#[test]
fn openssl_keys_parse_with_their_openssl_fingerprints() {
    let ed = key("ed25519.pub.pem");
    assert_eq!(ed.algorithm(), KeyAlgorithm::Ed25519);
    assert_eq!(
        ed.fingerprint(),
        "sha256:9797c6e083b023da1acb7b83fd4635033203c436f2562c00012191dc3670b20f"
    );
    let p256 = key("p256.pub.pem");
    assert_eq!(p256.algorithm(), KeyAlgorithm::EcdsaP256Sha256);
    assert_eq!(
        p256.fingerprint(),
        "sha256:94abfadd4116e169226fb49f55625d71d44a724b2fcc52688e51f0d3e44c54f0"
    );
}

#[test]
fn openssl_signatures_verify_from_memory_and_from_a_file() {
    let artifact = read("artifact.txt");
    let path = fixture("artifact.txt");
    for (key_file, signature_file) in [
        ("ed25519.pub.pem", "artifact.ed25519.sig"),
        ("p256.pub.pem", "artifact.p256.sig"),
    ] {
        let key = key(key_file);
        let signature = read(signature_file);
        let verifier = KeyVerifier::new(vec![key.clone()]);
        for content in [SignedContent::Bytes(&artifact), SignedContent::File(&path)] {
            let result = verifier.verify(&VerifyRequest::new(content, &signature));
            assert_eq!(
                result.outcome,
                SignatureVerifyOutcome::Verified,
                "{key_file}"
            );
            assert_eq!(
                result.policy,
                format!("caller key {} {}", key.algorithm(), key.fingerprint())
            );
        }
        // One changed byte of content.
        let mut changed = artifact.clone();
        changed[0] ^= 1;
        let result = verifier.verify(&VerifyRequest::new(
            SignedContent::Bytes(&changed),
            &signature,
        ));
        assert_eq!(
            result.outcome,
            SignatureVerifyOutcome::Invalid,
            "{key_file}"
        );
    }
    // The other algorithm's key does not verify it.
    let verifier = KeyVerifier::new(vec![key("p256.pub.pem")]);
    let signature = read("artifact.ed25519.sig");
    let result = verifier.verify(&VerifyRequest::new(
        SignedContent::Bytes(&artifact),
        &signature,
    ));
    assert_eq!(result.outcome, SignatureVerifyOutcome::Invalid);
}
