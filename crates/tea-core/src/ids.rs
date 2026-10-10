//! Identifier newtypes.
//!
//! A TEA UUID is unique only together with an API base and an object type.
//! These newtypes stop product, release, component, collection, and artifact
//! identifiers from being interchangeable in the type system.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::error::ModelError;

/// Lowercase RFC 9562 hyphenated UUID, matching the TEA `uuid` schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeaUuid(Uuid);

impl TeaUuid {
    /// Parse the canonical lowercase hyphenated form.
    ///
    /// # Errors
    ///
    /// Returns an error when the text is not a lowercase hyphenated UUID.
    pub fn parse(text: &str) -> Result<Self, ModelError> {
        if text.len() != 36 || text.chars().any(|c| c.is_ascii_uppercase()) {
            return Err(ModelError::Invalid(
                "UUID must be 36 lowercase hexadecimal characters with hyphens".into(),
            ));
        }
        let parsed = Uuid::parse_str(text)
            .map_err(|err| ModelError::Invalid(format!("UUID is not valid: {err}")))?;
        if parsed.as_hyphenated().to_string() != text {
            return Err(ModelError::Invalid(
                "UUID must use the hyphenated textual form".into(),
            ));
        }
        Ok(Self(parsed))
    }

    /// Borrow the underlying UUID.
    #[must_use]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for TeaUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

impl Serialize for TeaUuid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TeaUuid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

macro_rules! id_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(TeaUuid);

        impl $name {
            /// Parse a canonical TEA UUID.
            ///
            /// # Errors
            ///
            /// Returns an error when `text` is not a canonical TEA UUID.
            pub fn parse(text: &str) -> Result<Self, ModelError> {
                Ok(Self(TeaUuid::parse(text)?))
            }

            /// Wrap an already validated UUID.
            #[must_use]
            pub fn from_tea_uuid(uuid: TeaUuid) -> Self {
                Self(uuid)
            }

            /// Borrow the inner UUID.
            #[must_use]
            pub fn as_tea_uuid(&self) -> &TeaUuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_newtype!(ProductId, "UUID of a TEA Product at one API base.");
id_newtype!(
    ProductReleaseId,
    "UUID of a TEA Product Release at one API base."
);
id_newtype!(ComponentId, "UUID of a TEA Component at one API base.");
id_newtype!(
    ComponentReleaseId,
    "UUID of a TEA Component Release at one API base."
);
id_newtype!(
    CollectionId,
    "UUID of a TEA Collection. The specification requires this to equal the parent release UUID."
);
id_newtype!(ArtifactId, "UUID of a TEA Artifact at one API base.");
id_newtype!(DistributionId, "UUID of a component-release distribution.");

impl CollectionId {
    /// Collection UUID taken from its parent product release.
    #[must_use]
    pub fn from_product_release(id: ProductReleaseId) -> Self {
        Self(*id.as_tea_uuid())
    }

    /// Collection UUID taken from its parent component release.
    #[must_use]
    pub fn from_component_release(id: ComponentReleaseId) -> Self {
        Self(*id.as_tea_uuid())
    }
}

/// RFC 3339 UTC timestamp with a `Z` designator, as required by the `date-time` schema.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TeaTimestamp(String);

impl TeaTimestamp {
    /// Validate a TEA timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the text does not match the normative pattern.
    pub fn parse(text: &str) -> Result<Self, ModelError> {
        if !is_tea_timestamp(text) {
            return Err(ModelError::Invalid(format!(
                "timestamp must be RFC 3339 UTC with a Z designator, got `{text}`"
            )));
        }
        Ok(Self(text.to_owned()))
    }

    /// Borrow the original timestamp text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TeaTimestamp {
    type Error = ModelError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<TeaTimestamp> for String {
    fn from(value: TeaTimestamp) -> Self {
        value.0
    }
}

impl fmt::Display for TeaTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_tea_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[10] != b'T' || *bytes.last().unwrap_or(&0) != b'Z' {
        return false;
    }
    let (date, rest) = text.split_at(10);
    let time_and_frac = &rest[1..rest.len() - 1];
    let (time, frac) = match time_and_frac.split_once('.') {
        Some((time, frac)) => (time, Some(frac)),
        None => (time_and_frac, None),
    };
    if date.len() != 10 || time.len() != 8 {
        return false;
    }
    let date_ok = date.as_bytes()[4] == b'-'
        && date.as_bytes()[7] == b'-'
        && date
            .as_bytes()
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit());
    let time_ok = time.as_bytes()[2] == b':'
        && time.as_bytes()[5] == b':'
        && time
            .as_bytes()
            .iter()
            .enumerate()
            .all(|(i, b)| i == 2 || i == 5 || b.is_ascii_digit());
    let frac_ok = frac
        .map(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
        .unwrap_or(true);
    date_ok && time_ok && frac_ok
}
