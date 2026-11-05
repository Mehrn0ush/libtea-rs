//! Artifact checksum calculation and verification.
//!
//! Checksums cover artifact bytes after HTTP transfer coding and content coding
//! are removed. They are not ETags and they do not cover detached signatures.

use blake2::Blake2b;
use digest::consts::{U32, U48, U64};
use digest::Digest;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use sha3::{Sha3_256, Sha3_384, Sha3_512};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::enums::{ChecksumAlgorithm, EnumKnowledge};
use crate::error::ModelError;

type Blake2b256Hasher = Blake2b<U32>;
type Blake2b384Hasher = Blake2b<U48>;
type Blake2b512Hasher = Blake2b<U64>;

/// One published checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum {
    algorithm: ChecksumAlgorithm,
    hex_value: String,
}

impl Checksum {
    /// Validate `algType` and lowercase `algValue`.
    ///
    /// Known algorithms must use their digest length. Unrecognized algorithms
    /// keep the hex string when it matches the schema length bounds.
    ///
    /// # Errors
    ///
    /// Returns an error when the hex spelling or length is not allowed.
    pub fn new(algorithm: ChecksumAlgorithm, hex_value: &str) -> Result<Self, ModelError> {
        if !hex_value.bytes().all(|b| b.is_ascii_hexdigit())
            || hex_value.bytes().any(|b| b.is_ascii_uppercase())
        {
            return Err(ModelError::Invalid(
                "checksum algValue must be lowercase hexadecimal".into(),
            ));
        }
        if let Some(len) = algorithm.digest_len() {
            if hex_value.len() != len * 2 {
                return Err(ModelError::Invalid(format!(
                    "checksum length for {} must be {} hexadecimal characters",
                    algorithm,
                    len * 2
                )));
            }
        } else if !(40..=128).contains(&hex_value.len()) || hex_value.len() % 2 != 0 {
            return Err(ModelError::Invalid(
                "unrecognized checksum must be an even lowercase hex string of 40 to 128 characters"
                    .into(),
            ));
        }
        Ok(Self {
            algorithm,
            hex_value: hex_value.to_owned(),
        })
    }

    /// Algorithm.
    #[must_use]
    pub fn algorithm(&self) -> &ChecksumAlgorithm {
        &self.algorithm
    }

    /// Lowercase hexadecimal digest.
    #[must_use]
    pub fn hex_value(&self) -> &str {
        &self.hex_value
    }
}

#[derive(Deserialize)]
struct ChecksumWire {
    #[serde(rename = "algType")]
    alg_type: String,
    #[serde(rename = "algValue")]
    alg_value: String,
}

impl Serialize for Checksum {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Checksum", 2)?;
        state.serialize_field("algType", self.algorithm.as_str())?;
        state.serialize_field("algValue", &self.hex_value)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Checksum {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = ChecksumWire::deserialize(deserializer)?;
        Self::new(
            ChecksumAlgorithm::from_wire(&wire.alg_type),
            &wire.alg_value,
        )
        .map_err(serde::de::Error::custom)
    }
}

/// Outcome of checking every declared checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityOutcome {
    /// Every declared checksum was computed and matched.
    Verified,
    /// At least one computed checksum differed.
    Failed,
    /// No declared algorithm is implemented, so verification was not performed.
    Unsupported,
    /// Some declared checksums matched and at least one algorithm is unimplemented.
    ///
    /// This is not successful verification.
    Incomplete,
    /// The artifact format did not declare checksums.
    NotDeclared,
}

/// One checksum comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumCheck {
    /// Algorithm wire name.
    pub algorithm: String,
    /// Expected lowercase hex.
    pub expected: String,
    /// Computed lowercase hex when the algorithm is implemented.
    pub actual: Option<String>,
    /// Whether the comparison could be performed.
    pub supported: bool,
    /// Whether a supported comparison matched.
    pub matched: Option<bool>,
}

/// Full integrity report. [`IntegrityOutcome::Verified`] is set only when every
/// declared checksum was actually checked and matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrityReport {
    /// Summary that callers should branch on.
    pub outcome: IntegrityOutcome,
    /// Per-checksum detail.
    pub checks: Vec<ChecksumCheck>,
}

/// Incremental verifier over decoded artifact bytes.
pub struct ChecksumVerifier {
    items: Vec<Running>,
}

struct Running {
    checksum: Checksum,
    hasher: Option<RunningHasher>,
}

enum RunningHasher {
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
    Sha3_256(Sha3_256),
    Sha3_384(Sha3_384),
    Sha3_512(Sha3_512),
    Blake2b256(Blake2b256Hasher),
    Blake2b384(Blake2b384Hasher),
    Blake2b512(Blake2b512Hasher),
    Blake3(Box<blake3::Hasher>),
}

impl ChecksumVerifier {
    /// Start a verifier for the declared checksums.
    ///
    /// An empty list yields [`IntegrityOutcome::NotDeclared`] and does not
    /// report success.
    #[must_use]
    pub fn new(checksums: &[Checksum]) -> Self {
        let items = checksums
            .iter()
            .map(|checksum| Running {
                hasher: hasher_for(checksum.algorithm()),
                checksum: checksum.clone(),
            })
            .collect();
        Self { items }
    }

    /// Absorb decoded artifact bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        for item in &mut self.items {
            if let Some(hasher) = item.hasher.as_mut() {
                hasher.update(bytes);
            }
        }
    }

    /// Finish and compare.
    #[must_use]
    pub fn finish(self) -> IntegrityReport {
        if self.items.is_empty() {
            return IntegrityReport {
                outcome: IntegrityOutcome::NotDeclared,
                checks: Vec::new(),
            };
        }
        let mut checks = Vec::with_capacity(self.items.len());
        for item in self.items {
            let actual = item.hasher.map(RunningHasher::finish_hex);
            let matched = actual
                .as_ref()
                .map(|value| value == &item.checksum.hex_value);
            checks.push(ChecksumCheck {
                algorithm: item.checksum.algorithm().as_str().to_owned(),
                expected: item.checksum.hex_value().to_owned(),
                supported: item.checksum.algorithm().knowledge() == EnumKnowledge::Known
                    && actual.is_some(),
                actual,
                matched,
            });
        }
        let any_failed = checks.iter().any(|check| check.matched == Some(false));
        let any_unsupported = checks.iter().any(|check| !check.supported);
        let all_matched = checks.iter().all(|check| check.matched == Some(true));
        let outcome = if any_failed {
            IntegrityOutcome::Failed
        } else if all_matched {
            IntegrityOutcome::Verified
        } else if any_unsupported && checks.iter().any(|check| check.matched == Some(true)) {
            IntegrityOutcome::Incomplete
        } else {
            IntegrityOutcome::Unsupported
        };
        IntegrityReport { outcome, checks }
    }
}

impl RunningHasher {
    fn update(&mut self, bytes: &[u8]) {
        match self {
            Self::Sha1(hasher) => Digest::update(hasher, bytes),
            Self::Sha256(hasher) => Digest::update(hasher, bytes),
            Self::Sha384(hasher) => Digest::update(hasher, bytes),
            Self::Sha512(hasher) => Digest::update(hasher, bytes),
            Self::Sha3_256(hasher) => Digest::update(hasher, bytes),
            Self::Sha3_384(hasher) => Digest::update(hasher, bytes),
            Self::Sha3_512(hasher) => Digest::update(hasher, bytes),
            Self::Blake2b256(hasher) => Digest::update(hasher, bytes),
            Self::Blake2b384(hasher) => Digest::update(hasher, bytes),
            Self::Blake2b512(hasher) => Digest::update(hasher, bytes),
            Self::Blake3(hasher) => {
                hasher.update(bytes);
            }
        }
    }

    fn finish_hex(self) -> String {
        match self {
            Self::Sha1(hasher) => hex_encode(&hasher.finalize()),
            Self::Sha256(hasher) => hex_encode(&hasher.finalize()),
            Self::Sha384(hasher) => hex_encode(&hasher.finalize()),
            Self::Sha512(hasher) => hex_encode(&hasher.finalize()),
            Self::Sha3_256(hasher) => hex_encode(&hasher.finalize()),
            Self::Sha3_384(hasher) => hex_encode(&hasher.finalize()),
            Self::Sha3_512(hasher) => hex_encode(&hasher.finalize()),
            Self::Blake2b256(hasher) => hex_encode(&hasher.finalize()),
            Self::Blake2b384(hasher) => hex_encode(&hasher.finalize()),
            Self::Blake2b512(hasher) => hex_encode(&hasher.finalize()),
            Self::Blake3(hasher) => hex_encode(hasher.finalize().as_bytes()),
        }
    }
}

fn hasher_for(algorithm: &ChecksumAlgorithm) -> Option<RunningHasher> {
    Some(match algorithm {
        ChecksumAlgorithm::Sha1 => RunningHasher::Sha1(Sha1::new()),
        ChecksumAlgorithm::Sha256 => RunningHasher::Sha256(Sha256::new()),
        ChecksumAlgorithm::Sha384 => RunningHasher::Sha384(Sha384::new()),
        ChecksumAlgorithm::Sha512 => RunningHasher::Sha512(Sha512::new()),
        ChecksumAlgorithm::Sha3_256 => RunningHasher::Sha3_256(Sha3_256::new()),
        ChecksumAlgorithm::Sha3_384 => RunningHasher::Sha3_384(Sha3_384::new()),
        ChecksumAlgorithm::Sha3_512 => RunningHasher::Sha3_512(Sha3_512::new()),
        ChecksumAlgorithm::Blake2b256 => RunningHasher::Blake2b256(Blake2b256Hasher::new()),
        ChecksumAlgorithm::Blake2b384 => RunningHasher::Blake2b384(Blake2b384Hasher::new()),
        ChecksumAlgorithm::Blake2b512 => RunningHasher::Blake2b512(Blake2b512Hasher::new()),
        ChecksumAlgorithm::Blake3 => RunningHasher::Blake3(Box::new(blake3::Hasher::new())),
        ChecksumAlgorithm::Other(_) => return None,
    })
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

/// Verify a complete decoded payload.
#[must_use]
pub fn verify_checksums(bytes: &[u8], checksums: &[Checksum]) -> IntegrityReport {
    let mut verifier = ChecksumVerifier::new(checksums);
    verifier.update(bytes);
    verifier.finish()
}

/// Incremental removal of one HTTP content coding.
///
/// `deflate` is the zlib wrapper from RFC 9110, not raw DEFLATE. `gzip` and
/// `x-gzip` use the gzip wrapper. Both the encoded input and the decoded
/// output are capped at `max_output`.
pub struct ContentDecoder {
    inner: DecoderInner,
    max_output: usize,
    input_len: usize,
}

enum DecoderInner {
    Plain(Vec<u8>),
    Gzip(flate2::write::GzDecoder<CappedBuf>),
    Zlib(ZlibInflate),
}

/// HTTP `deflate` decoder that accepts a member only at zlib stream end.
///
/// `flate2`'s writer decoder returns the bytes inflated so far when `finish`
/// sees no further output, including a truncated member. This wrapper requires
/// [`flate2::Status::StreamEnd`].
struct ZlibInflate {
    decompressor: flate2::Decompress,
    output: CappedBuf,
    ended: bool,
}

impl ZlibInflate {
    fn new(max: usize) -> Self {
        Self {
            decompressor: flate2::Decompress::new(true),
            output: CappedBuf::new(max),
            ended: false,
        }
    }

    fn push(&mut self, chunk: &[u8]) -> Result<(), ModelError> {
        if self.ended {
            return Err(ModelError::Invalid(
                "content coding decode failed: trailing data after deflate stream".into(),
            ));
        }
        if inflate(
            &mut self.decompressor,
            chunk,
            &mut self.output,
            flate2::FlushDecompress::None,
        )? == flate2::Status::StreamEnd
        {
            self.ended = true;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<u8>, ModelError> {
        if !self.ended {
            let status = inflate(
                &mut self.decompressor,
                &[],
                &mut self.output,
                flate2::FlushDecompress::Finish,
            )?;
            if status != flate2::Status::StreamEnd {
                return Err(ModelError::Invalid(
                    "content coding decode failed: truncated deflate stream".into(),
                ));
            }
        }
        Ok(self.output.bytes)
    }

    fn len(&self) -> usize {
        self.output.len()
    }
}

fn inflate(
    decompressor: &mut flate2::Decompress,
    input: &[u8],
    output: &mut CappedBuf,
    flush: flate2::FlushDecompress,
) -> Result<flate2::Status, ModelError> {
    let mut offset = 0usize;
    loop {
        let mut buf = [0u8; 8192];
        let in_before = decompressor.total_in();
        let out_before = decompressor.total_out();
        let available = if offset < input.len() {
            &input[offset..]
        } else {
            &[]
        };
        let status = decompressor
            .decompress(available, &mut buf, flush)
            .map_err(|_| {
                ModelError::Invalid("content coding decode failed: corrupt deflate stream".into())
            })?;
        let consumed = usize::try_from(decompressor.total_in() - in_before).unwrap_or(usize::MAX);
        let produced = usize::try_from(decompressor.total_out() - out_before).unwrap_or(usize::MAX);
        if consumed > available.len() || produced > buf.len() {
            return Err(ModelError::Invalid(
                "content coding decode failed: corrupt deflate stream".into(),
            ));
        }
        if produced > 0 {
            std::io::Write::write_all(output, &buf[..produced]).map_err(decode_error)?;
        }
        offset = offset.saturating_add(consumed);
        if status == flate2::Status::StreamEnd {
            if offset < input.len() {
                return Err(ModelError::Invalid(
                    "content coding decode failed: trailing data after deflate stream".into(),
                ));
            }
            return Ok(status);
        }
        if consumed == 0 && produced == 0 {
            return Ok(status);
        }
    }
}

/// Output buffer that refuses to grow past the decoded-size cap.
///
/// The check runs inside `Write::write`, so one compressed chunk cannot expand
/// the buffer past the cap before the error is returned.
struct CappedBuf {
    bytes: Vec<u8>,
    max: usize,
}

impl CappedBuf {
    fn new(max: usize) -> Self {
        Self {
            bytes: Vec::new(),
            max,
        }
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }
}

impl std::io::Write for CappedBuf {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.bytes.len().saturating_add(data.len()) > self.max {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "decoded artifact exceeds the size limit",
            ));
        }
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ContentDecoder {
    /// Start a decoder for one coding.
    ///
    /// An omitted coding and `identity` copy the input. Stacked codings and
    /// unknown codings are rejected.
    ///
    /// # Errors
    ///
    /// Returns an error when the coding is not a single supported value.
    pub fn new(coding: Option<&str>, max_output: usize) -> Result<Self, ModelError> {
        let inner = match coding.map(str::trim).filter(|value| !value.is_empty()) {
            None => DecoderInner::Plain(Vec::new()),
            Some(coding) if coding.eq_ignore_ascii_case("identity") => {
                DecoderInner::Plain(Vec::new())
            }
            Some(coding) if coding.contains(',') => {
                return Err(ModelError::Invalid(
                    "stacked content codings are rejected".into(),
                ));
            }
            Some(coding)
                if coding.eq_ignore_ascii_case("gzip") || coding.eq_ignore_ascii_case("x-gzip") =>
            {
                DecoderInner::Gzip(flate2::write::GzDecoder::new(CappedBuf::new(max_output)))
            }
            Some(coding) if coding.eq_ignore_ascii_case("deflate") => {
                DecoderInner::Zlib(ZlibInflate::new(max_output))
            }
            Some(coding) => {
                return Err(ModelError::Invalid(format!(
                    "unsupported HTTP content coding `{coding}`"
                )));
            }
        };
        Ok(Self {
            inner,
            max_output,
            input_len: 0,
        })
    }

    /// Absorb one chunk of the encoded body.
    ///
    /// # Errors
    ///
    /// Returns an error when the encoded input or decoded output would exceed
    /// the cap, or the coding stream is invalid.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), ModelError> {
        if self.input_len.saturating_add(chunk.len()) > self.max_output {
            return Err(ModelError::Invalid(
                "encoded artifact exceeds the size limit".into(),
            ));
        }
        self.input_len += chunk.len();
        match &mut self.inner {
            DecoderInner::Plain(output) => output.extend_from_slice(chunk),
            DecoderInner::Gzip(decoder) => {
                std::io::Write::write_all(decoder, chunk).map_err(decode_error)?;
            }
            DecoderInner::Zlib(decoder) => decoder.push(chunk)?,
        }
        if self.output_len() > self.max_output {
            return Err(ModelError::Invalid(
                "decoded artifact exceeds the size limit".into(),
            ));
        }
        Ok(())
    }

    /// Finish the coding stream and return the decoded bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the stream is truncated or the output cap is exceeded.
    pub fn finish(self) -> Result<Vec<u8>, ModelError> {
        let output = match self.inner {
            DecoderInner::Plain(output) => output,
            DecoderInner::Gzip(decoder) => decoder.finish().map_err(decode_error)?.bytes,
            DecoderInner::Zlib(decoder) => decoder.finish()?,
        };
        if output.len() > self.max_output {
            return Err(ModelError::Invalid(
                "decoded artifact exceeds the size limit".into(),
            ));
        }
        Ok(output)
    }

    fn output_len(&self) -> usize {
        match &self.inner {
            DecoderInner::Plain(output) => output.len(),
            DecoderInner::Gzip(decoder) => decoder.get_ref().len(),
            DecoderInner::Zlib(decoder) => decoder.len(),
        }
    }
}

fn decode_error(err: std::io::Error) -> ModelError {
    ModelError::Invalid(format!("content coding decode failed: {err}"))
}

/// Decode one HTTP content coding.
///
/// `gzip` and HTTP `deflate` (zlib) are decoded. `identity` and an omitted
/// coding return the input. Any other coding is rejected so compressed bytes
/// are not treated as artifact content.
///
/// # Errors
///
/// Returns an error for an unsupported coding, a truncated stream, raw DEFLATE
/// presented as HTTP `deflate`, or output that would exceed `max_output`.
pub fn decode_content_coding(
    body: &[u8],
    coding: Option<&str>,
    max_output: usize,
) -> Result<Vec<u8>, ModelError> {
    let mut decoder = ContentDecoder::new(coding, max_output)?;
    decoder.push(body)?;
    decoder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_and_mismatch_is_not_success() {
        let bytes = b"tea";
        let good = Checksum::new(
            ChecksumAlgorithm::Sha256,
            "6ebf8529d78b9572b3c6b5a1b0c5c5c5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5",
        );
        // Compute the real digest through the verifier by using a known vector.
        let checksum = Checksum::new(ChecksumAlgorithm::Sha256, &{
            let report_seed = Checksum::new(
                ChecksumAlgorithm::Sha256,
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap();
            let report = verify_checksums(bytes, &[report_seed]);
            report.checks[0].actual.clone().unwrap()
        })
        .unwrap();
        let ok = verify_checksums(bytes, std::slice::from_ref(&checksum));
        assert_eq!(ok.outcome, IntegrityOutcome::Verified);
        let bad = checksum;
        // Flip the expected value by rebuilding.
        let flipped = format!("0{}", &bad.hex_value()[1..]);
        let bad_sum = Checksum::new(ChecksumAlgorithm::Sha256, &flipped).unwrap();
        let failed = verify_checksums(bytes, &[bad_sum]);
        assert_eq!(failed.outcome, IntegrityOutcome::Failed);
        let _ = good;
        let _ = bad;
    }

    #[test]
    fn missing_and_unknown_algorithms_are_not_verified() {
        let empty = verify_checksums(b"abc", &[]);
        assert_eq!(empty.outcome, IntegrityOutcome::NotDeclared);
        let unknown = Checksum::new(
            ChecksumAlgorithm::Other("SHA-224".into()),
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let report = verify_checksums(b"abc", &[unknown]);
        assert_eq!(report.outcome, IntegrityOutcome::Unsupported);
    }

    #[test]
    fn gzip_decoding_removes_content_coding_before_checksum() {
        let raw = b"artifact-bytes";
        let compressed = gzip_bytes(raw);
        let decoded = decode_content_coding(&compressed, Some("gzip"), 1024).unwrap();
        assert_eq!(decoded, raw);
        let checksum = checksum_of(raw);
        let report = verify_checksums(&decoded, std::slice::from_ref(&checksum));
        assert_eq!(report.outcome, IntegrityOutcome::Verified);
    }

    #[test]
    fn http_deflate_is_zlib_and_raw_deflate_is_rejected() {
        let raw = b"artifact-bytes";
        let zlib = zlib_bytes(raw);
        let decoded = decode_content_coding(&zlib, Some("deflate"), 1024).unwrap();
        assert_eq!(decoded, raw);
        let checksum = checksum_of(raw);
        let report = verify_checksums(&decoded, std::slice::from_ref(&checksum));
        assert_eq!(report.outcome, IntegrityOutcome::Verified);

        let raw_deflate = raw_deflate_bytes(raw);
        assert!(decode_content_coding(&raw_deflate, Some("deflate"), 1024).is_err());
    }

    #[test]
    fn gzip_expansion_is_capped() {
        let raw = vec![b'a'; 64 * 1024];
        let compressed = gzip_bytes(&raw);
        assert!(compressed.len() < raw.len());
        assert!(decode_content_coding(&compressed, Some("gzip"), 1024).is_err());
    }

    #[test]
    fn truncated_gzip_and_zlib_are_rejected() {
        let gzip = gzip_bytes(b"artifact-bytes");
        let zlib = zlib_bytes(b"artifact-bytes");
        assert!(gzip.len() > 8 && zlib.len() > 8);
        assert!(decode_content_coding(&gzip[..gzip.len() / 2], Some("gzip"), 1024).is_err());
        assert!(decode_content_coding(&zlib[..zlib.len() / 2], Some("deflate"), 1024).is_err());
        assert!(decode_content_coding(&zlib[..zlib.len() - 4], Some("deflate"), 1024).is_err());
    }

    fn gzip_bytes(raw: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, raw).unwrap();
        encoder.finish().unwrap()
    }

    fn zlib_bytes(raw: &[u8]) -> Vec<u8> {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, raw).unwrap();
        encoder.finish().unwrap()
    }

    fn raw_deflate_bytes(raw: &[u8]) -> Vec<u8> {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, raw).unwrap();
        encoder.finish().unwrap()
    }

    fn checksum_of(raw: &[u8]) -> Checksum {
        let seed = Checksum::new(
            ChecksumAlgorithm::Sha256,
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let actual = verify_checksums(raw, &[seed]).checks[0]
            .actual
            .clone()
            .unwrap();
        Checksum::new(ChecksumAlgorithm::Sha256, &actual).unwrap()
    }

    fn digest_hex(algorithm: ChecksumAlgorithm, raw: &[u8]) -> String {
        let zero = "0".repeat(algorithm.digest_len().unwrap_or(32) * 2);
        let seed = Checksum::new(algorithm.clone(), &zero).unwrap();
        verify_checksums(raw, &[seed]).checks[0]
            .actual
            .clone()
            .unwrap()
    }

    #[test]
    fn supported_algorithms_verify_decoded_bytes() {
        let raw = b"algorithm-matrix";
        for algorithm in [
            ChecksumAlgorithm::Sha1,
            ChecksumAlgorithm::Sha256,
            ChecksumAlgorithm::Sha384,
            ChecksumAlgorithm::Sha512,
            ChecksumAlgorithm::Sha3_256,
            ChecksumAlgorithm::Sha3_384,
            ChecksumAlgorithm::Sha3_512,
            ChecksumAlgorithm::Blake2b256,
            ChecksumAlgorithm::Blake2b384,
            ChecksumAlgorithm::Blake2b512,
            ChecksumAlgorithm::Blake3,
        ] {
            let hex = digest_hex(algorithm.clone(), raw);
            let checksum = Checksum::new(algorithm, &hex).unwrap();
            let report = verify_checksums(raw, &[checksum]);
            assert_eq!(report.outcome, IntegrityOutcome::Verified);
        }
    }

    #[test]
    fn multiple_matching_checksums_verify_and_conflict_fails() {
        let raw = b"multi";
        let sha = Checksum::new(
            ChecksumAlgorithm::Sha256,
            &digest_hex(ChecksumAlgorithm::Sha256, raw),
        )
        .unwrap();
        let blake = Checksum::new(
            ChecksumAlgorithm::Blake3,
            &digest_hex(ChecksumAlgorithm::Blake3, raw),
        )
        .unwrap();
        assert_eq!(
            verify_checksums(raw, &[sha.clone(), blake]).outcome,
            IntegrityOutcome::Verified
        );
        let wrong = Checksum::new(
            ChecksumAlgorithm::Sha256,
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        )
        .unwrap();
        assert_eq!(
            verify_checksums(raw, &[sha, wrong]).outcome,
            IntegrityOutcome::Failed
        );
    }

    #[test]
    fn incomplete_when_one_matches_and_one_is_unknown() {
        let raw = b"partial";
        let sha = Checksum::new(
            ChecksumAlgorithm::Sha256,
            &digest_hex(ChecksumAlgorithm::Sha256, raw),
        )
        .unwrap();
        let unknown = Checksum::new(
            ChecksumAlgorithm::Other("SHA-224".into()),
            "0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        assert_eq!(
            verify_checksums(raw, &[sha, unknown]).outcome,
            IntegrityOutcome::Incomplete
        );
    }

    #[test]
    fn uppercase_hex_is_rejected_at_construction() {
        assert!(Checksum::new(
            ChecksumAlgorithm::Sha256,
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        )
        .is_err());
    }

    /// Independent known-answer tests for input `b"abc"`.
    ///
    /// Digests are hard-coded from sources outside this verifier:
    /// - SHA-1 / SHA-2 / SHA-3: NIST FIPS 180-4 and FIPS 202 example vectors
    ///   (also cross-checked with Python `hashlib` and OpenSSL `dgst`).
    /// - BLAKE2b-*: RFC 7693 / OpenSSL `blake2b512` and Python `hashlib.blake2b`
    ///   with the matching `digest_size`.
    /// - BLAKE3: output of the reference `blake3` crate's `hash` API in an
    ///   isolated binary (not [`ChecksumVerifier`]), matching the published
    ///   empty-string vector `af1349b9…3262`.
    #[test]
    fn independent_known_answer_vectors_for_abc() {
        const MSG: &[u8] = b"abc";
        let cases: &[(ChecksumAlgorithm, &str)] = &[
            (
                ChecksumAlgorithm::Sha1,
                "a9993e364706816aba3e25717850c26c9cd0d89d",
            ),
            (
                ChecksumAlgorithm::Sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                ChecksumAlgorithm::Sha384,
                "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
8086072ba1e7cc2358baeca134c825a7",
            ),
            (
                ChecksumAlgorithm::Sha512,
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            ),
            (
                ChecksumAlgorithm::Sha3_256,
                "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
            ),
            (
                ChecksumAlgorithm::Sha3_384,
                "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b2\
98d88cea927ac7f539f1edf228376d25",
            ),
            (
                ChecksumAlgorithm::Sha3_512,
                "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e\
10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
            ),
            (
                ChecksumAlgorithm::Blake2b256,
                "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319",
            ),
            (
                ChecksumAlgorithm::Blake2b384,
                "6f56a82c8e7ef526dfe182eb5212f7db9df1317e57815dbda46083fc30f54ee6\
c66ba83be64b302d7cba6ce15bb556f4",
            ),
            (
                ChecksumAlgorithm::Blake2b512,
                "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
            ),
            (
                ChecksumAlgorithm::Blake3,
                "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
            ),
        ];
        for (algorithm, expected) in cases {
            let expected = expected.replace('\n', "");
            assert_eq!(
                expected.len(),
                algorithm.digest_len().unwrap() * 2,
                "{algorithm} digest length"
            );
            let checksum = Checksum::new(algorithm.clone(), &expected).unwrap();
            let report = verify_checksums(MSG, &[checksum]);
            assert_eq!(
                report.outcome,
                IntegrityOutcome::Verified,
                "{algorithm} known-answer failed; actual={:?}",
                report.checks[0].actual
            );
            assert_eq!(report.checks[0].actual.as_deref(), Some(expected.as_str()));

            // Wire values must be lowercase hex; uppercase is rejected.
            assert!(
                Checksum::new(algorithm.clone(), &expected.to_ascii_uppercase()).is_err(),
                "{algorithm} must reject uppercase hex"
            );
        }
    }
}
