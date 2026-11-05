//! Errors from TEA HTTP operations.
//!
//! Display text does not include access tokens, API key secrets, or
//! `Authorization` header values.

use cdx_tea_core::{IntegrityOutcome, IntegrityReport, ModelError, PolicyError};
use thiserror::Error;

/// Failure of a consumer HTTP operation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ClientError {
    /// The URL was rejected by destination policy.
    #[error(transparent)]
    Policy(#[from] PolicyError),
    /// A TEA document or identifier was not valid.
    #[error(transparent)]
    Model(#[from] ModelError),
    /// JSON that could not be decoded.
    #[error("JSON parse error: {0}")]
    Json(String),
    /// A connection or protocol failure.
    #[error("transport error: {0}")]
    Transport(String),
    /// A response exceeded a configured limit.
    #[error("response exceeded the configured size limit")]
    TooLarge,
    /// Redirects exceeded the configured limit.
    #[error("too many redirects")]
    TooManyRedirects,
    /// A redirect had no usable `Location`.
    #[error("redirect location is missing or invalid")]
    BadRedirect,
    /// The request timed out.
    #[error("request timed out")]
    Timeout,
    /// The server required authentication and no credential was available for that origin.
    #[error("authentication is required")]
    AuthenticationRequired,
    /// The server returned 403. This does not trigger token replacement or failover.
    #[error("request is not authorized")]
    Forbidden,
    /// An HTTP status the operation did not accept.
    #[error("TEA API status {status}{}", tea_suffix(tea_error))]
    Api {
        /// HTTP status code.
        status: u16,
        /// TEA `error` value when the body was a TEA error response.
        tea_error: Option<String>,
        /// Optional human-readable message. Not a programmatic contract.
        message: Option<String>,
    },
    /// `/discovery` returned a TEA 404. This is not evidence that no updates exist.
    #[error("identifier was not resolved ({error})")]
    Unresolved {
        /// TEA error code, preserved even when this client does not know it.
        error: String,
        /// Optional diagnostic message.
        message: Option<String>,
    },
    /// Well-known or API discovery attempts were exhausted.
    #[error("discovery failed during the {stage} stage after {attempts} attempts")]
    DiscoveryFailed {
        /// `well-known` or `api`.
        stage: &'static str,
        /// Attempts counted toward the bound.
        attempts: u32,
    },
    /// Declared checksums were not fully verified.
    #[error("artifact integrity was not verified ({outcome:?})")]
    Integrity {
        /// Summary outcome.
        outcome: IntegrityOutcome,
        /// Per-algorithm detail.
        report: IntegrityReport,
    },
    /// The token endpoint rejected the credential exchange.
    #[error("token request failed ({error})")]
    Token {
        /// OAuth error code or HTTP status.
        error: String,
    },
}

fn tea_suffix(error: &Option<String>) -> String {
    match error {
        Some(code) => format!(" ({code})"),
        None => String::new(),
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}

impl ClientError {
    pub(crate) fn transport(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return Self::Timeout;
        }
        // Prefer structured removal: reqwest's Display appends
        // ` for url (https://...?secret=...)`. Drop that URL object first.
        let message = err.without_url().to_string();
        if message.to_ascii_lowercase().contains("authorization")
            || message.to_ascii_lowercase().contains("bearer ")
        {
            return Self::Transport("request failed".into());
        }
        // Residual free-text URLs are rewritten only when the whole
        // whitespace-bounded span can be parsed; otherwise use a generic
        // message rather than risk leaving query-value fragments.
        Self::Transport(crate::transport::sanitize_transport_text(&message))
    }
}
