//! Closed request enums and forward-compatible response enums.
//!
//! Response enumerations keep unknown wire values. The OpenAPI schemas list a
//! closed set, but the discovery document requires clients to accept an
//! `error` value they do not know. The same representation is used for other
//! protocol enumerations so a later revision does not become unreadable.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// How an enumeration value relates to the pinned TEA 1.0.0 schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnumKnowledge {
    /// The value is listed in the pinned schema.
    Known,
    /// The value was preserved for forward compatibility.
    Unknown,
}

macro_rules! wire_enum {
    ($name:ident { $($var:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum $name {
            $($var,)+
            /// A value this client does not recognize.
            Other(String),
        }

        impl $name {
            /// Parse a wire string without rejecting unknown values.
            #[must_use]
            pub fn from_wire(value: &str) -> Self {
                match value {
                    $($wire => Self::$var,)+
                    other => Self::Other(other.to_owned()),
                }
            }

            /// Borrow the wire spelling.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$var => $wire,)+
                    Self::Other(value) => value,
                }
            }

            /// Whether this value appears in the pinned schema.
            #[must_use]
            pub fn knowledge(&self) -> EnumKnowledge {
                match self {
                    Self::Other(_) => EnumKnowledge::Unknown,
                    _ => EnumKnowledge::Known,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                Ok(Self::from_wire(&value))
            }
        }
    };
}

wire_enum!(IdentifierType {
    Cpe => "CPE",
    Tei => "TEI",
    Purl => "PURL",
    ComplianceDocument => "COMPLIANCE_DOCUMENT",
});

wire_enum!(ComplianceDocumentType {
    Soc2TypeI => "SOC_2_TYPE_I",
    Soc2TypeIi => "SOC_2_TYPE_II",
    Soc3 => "SOC_3",
    Iso27001 => "ISO_27001",
    Iso27017 => "ISO_27017",
    Iso27018 => "ISO_27018",
    Iso27701 => "ISO_27701",
    Iso42001 => "ISO_42001",
    PciDss => "PCI_DSS",
    Hipaa => "HIPAA",
    Fedramp => "FEDRAMP",
    Gdpr => "GDPR",
    CsaStar => "CSA_STAR",
    Nist80053 => "NIST_800_53",
    Nist800171 => "NIST_800_171",
    Cmmc => "CMMC",
    Hitrust => "HITRUST",
    Tisax => "TISAX",
    CyberEssentials => "CYBER_ESSENTIALS",
    CyberEssentialsPlus => "CYBER_ESSENTIALS_PLUS",
    EuDeclarationOfConformity => "EU_DECLARATION_OF_CONFORMITY",
});

wire_enum!(ArtifactType {
    Attestation => "ATTESTATION",
    Bom => "BOM",
    BuildMeta => "BUILD_META",
    Certification => "CERTIFICATION",
    Formulation => "FORMULATION",
    License => "LICENSE",
    ReleaseNotes => "RELEASE_NOTES",
    SecurityTxt => "SECURITY_TXT",
    ThreatModel => "THREAT_MODEL",
    Vulnerabilities => "VULNERABILITIES",
    OtherArtifact => "OTHER",
});

wire_enum!(CollectionUpdateReasonType {
    InitialRelease => "INITIAL_RELEASE",
    VexUpdated => "VEX_UPDATED",
    ArtifactUpdated => "ARTIFACT_UPDATED",
    ArtifactAdded => "ARTIFACT_ADDED",
    ArtifactRemoved => "ARTIFACT_REMOVED",
});

wire_enum!(CollectionOwner {
    ComponentRelease => "COMPONENT_RELEASE",
    ProductRelease => "PRODUCT_RELEASE",
});

wire_enum!(CleEventType {
    Released => "released",
    EndOfDevelopment => "endOfDevelopment",
    EndOfSupport => "endOfSupport",
    EndOfLife => "endOfLife",
    EndOfDistribution => "endOfDistribution",
    EndOfMarketing => "endOfMarketing",
    SupersededBy => "supersededBy",
    ComponentRenamed => "componentRenamed",
    Withdrawn => "withdrawn",
});

wire_enum!(TeaErrorCode {
    ObjectUnknown => "OBJECT_UNKNOWN",
    NotImplemented => "NOT_IMPLEMENTED",
    NoAcceptableFormat => "NO_ACCEPTABLE_FORMAT",
    SignatureNotFound => "SIGNATURE_NOT_FOUND",
    InvalidRequest => "INVALID_REQUEST",
    InvalidPageToken => "INVALID_PAGE_TOKEN",
});

wire_enum!(ChecksumAlgorithm {
    Sha1 => "SHA-1",
    Sha256 => "SHA-256",
    Sha384 => "SHA-384",
    Sha512 => "SHA-512",
    Sha3_256 => "SHA3-256",
    Sha3_384 => "SHA3-384",
    Sha3_512 => "SHA3-512",
    Blake2b256 => "BLAKE2b-256",
    Blake2b384 => "BLAKE2b-384",
    Blake2b512 => "BLAKE2b-512",
    Blake3 => "BLAKE3",
});

impl ChecksumAlgorithm {
    /// Digest size in bytes for algorithms listed in `checksum-type`.
    #[must_use]
    pub fn digest_len(&self) -> Option<usize> {
        match self {
            Self::Sha1 => Some(20),
            Self::Sha256 | Self::Sha3_256 | Self::Blake2b256 | Self::Blake3 => Some(32),
            Self::Sha384 | Self::Sha3_384 | Self::Blake2b384 => Some(48),
            Self::Sha512 | Self::Sha3_512 | Self::Blake2b512 => Some(64),
            Self::Other(_) => None,
        }
    }

    /// Whether this algorithm is too weak for a match to count as
    /// [`crate::IntegrityOutcome::Verified`] on its own.
    ///
    /// `SHA-1` is listed in the pinned `checksum-type` enum but is not
    /// collision resistant. `MD5` is not in that enum; it is parsed as
    /// [`Self::Other`], never computed, and also reported as weak.
    #[must_use]
    pub fn is_weak(&self) -> bool {
        match self {
            Self::Sha1 => true,
            Self::Other(name) => name.eq_ignore_ascii_case("MD5"),
            _ => false,
        }
    }
}

/// Sort direction for paginated queries. The default in the specification is `asc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SortOrder {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

impl SortOrder {
    /// Wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

/// `sortField` for product queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProductSort {
    /// Sort by product name.
    Name,
}

/// `sortField` for component queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComponentSort {
    /// Sort by component name.
    Name,
}

/// `sortField` for product-release and component-release queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReleaseSort {
    /// Sort by `createdDate`.
    CreatedDate,
    /// Sort by `releaseDate`.
    ReleaseDate,
    /// Sort by the release version string.
    Version,
}

/// `sortField` for collection queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CollectionSort {
    /// Sort by collection version.
    Version,
}

impl ProductSort {
    /// Wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        "name"
    }
}

impl ComponentSort {
    /// Wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        "name"
    }
}

impl ReleaseSort {
    /// Wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CreatedDate => "createdDate",
            Self::ReleaseDate => "releaseDate",
            Self::Version => "version",
        }
    }
}

impl CollectionSort {
    /// Wire value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        "version"
    }
}
