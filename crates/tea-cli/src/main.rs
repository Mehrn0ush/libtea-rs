//! Command-line tool for the TEA consumer libraries (`libtea-cli`).
//!
//! Protocol behavior lives in `libtea-core` and `libtea-client`. This binary only
//! parses arguments and prints results.

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand};
use libtea_client::{ApiKey, ArtifactFetch, ClientBuilder, ClientError};
use libtea_core::{spec, IntegrityOutcome, ListParams, Tei};
use url::Url;

/// Make server-supplied text safe to print to a terminal (see
/// [`libtea_core::escape_control_chars`]). Applied to every line the CLI
/// prints, including text that the library already escaped.
fn clean(text: &str) -> String {
    libtea_core::escape_control_chars(text)
}

/// `println!` for text that may contain server data.
macro_rules! out {
    ($($arg:tt)*) => {
        println!("{}", clean(&format!($($arg)*)))
    };
}

/// `eprintln!` for text that may contain server data.
macro_rules! err {
    ($($arg:tt)*) => {
        eprintln!("{}", clean(&format!($($arg)*)))
    };
}

mod sync;

const ENV_API_KEY_ID: &str = "TEA_API_KEY_ID";
const ENV_API_KEY_SECRET: &str = "TEA_API_KEY_SECRET";

/// Process exit statuses. Documented in `docs/cli-exit-codes.md` and in
/// `--help`; the numbers are part of the CLI contract.
mod exit {
    /// Success.
    pub const OK: u8 = 0;
    /// Any failure not covered by a more specific status (network, I/O,
    /// server error, local file refusal).
    pub const FAILURE: u8 = 1;
    /// Bad command line or bad input value (clap uses 2 for argument errors).
    pub const USAGE: u8 = 2;
    /// Authentication or authorization failed (missing or rejected credential).
    pub const AUTH: u8 = 3;
    /// Every declared checksum matched but all use weak algorithms (SHA-1) and
    /// `--allow-weak-checksums` was not given.
    pub const WEAK_CHECKSUMS: u8 = 4;
    /// The identifier or object was not found or not resolved.
    pub const NOT_FOUND: u8 = 5;
    /// Integrity was not established: a checksum failed, could not be
    /// computed, or was required and absent.
    pub const INTEGRITY: u8 = 6;
    /// A comparison found changes (`sync --check`, `diff`).
    pub const CHANGES_DETECTED: u8 = 7;
}

/// A failure with a chosen exit status, raised by the CLI itself.
#[derive(Debug)]
struct Coded {
    status: u8,
    message: String,
}

impl std::fmt::Display for Coded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Coded {}

fn coded(status: u8, message: impl Into<String>) -> anyhow::Error {
    Coded {
        status,
        message: message.into(),
    }
    .into()
}

fn usage(message: impl Into<String>) -> anyhow::Error {
    coded(exit::USAGE, message)
}

const WEAK_MESSAGE: &str = "only weak checksum algorithms (SHA-1) were declared for this format; \
     that is not Verified. Pass --allow-weak-checksums to accept it";

/// Map an error to its exit status and a stable `kind` string for `--json`.
fn classify(error: &anyhow::Error) -> (u8, &'static str) {
    if let Some(coded) = error.downcast_ref::<Coded>() {
        return (coded.status, kind_for(coded.status));
    }
    if let Some(client) = error.downcast_ref::<ClientError>() {
        let status = match client {
            ClientError::AuthenticationRequired
            | ClientError::Forbidden
            | ClientError::Token { .. } => exit::AUTH,
            ClientError::Api {
                status: 401 | 403, ..
            } => exit::AUTH,
            ClientError::Api { status: 404, .. } | ClientError::Unresolved { .. } => {
                exit::NOT_FOUND
            }
            ClientError::Integrity {
                outcome: IntegrityOutcome::Weak,
                ..
            } => exit::WEAK_CHECKSUMS,
            ClientError::Integrity { .. } => exit::INTEGRITY,
            _ => exit::FAILURE,
        };
        return (status, kind_for(status));
    }
    // A bare model, TEI or policy error comes from a value the user passed.
    if error.downcast_ref::<libtea_core::TeiError>().is_some()
        || error.downcast_ref::<libtea_core::ModelError>().is_some()
        || error.downcast_ref::<libtea_core::PolicyError>().is_some()
    {
        return (exit::USAGE, kind_for(exit::USAGE));
    }
    (exit::FAILURE, kind_for(exit::FAILURE))
}

fn kind_for(status: u8) -> &'static str {
    match status {
        exit::USAGE => "usage",
        exit::AUTH => "auth",
        exit::WEAK_CHECKSUMS => "weak_checksums",
        exit::NOT_FOUND => "not_found",
        exit::INTEGRITY => "integrity",
        exit::CHANGES_DETECTED => "changes_detected",
        _ => "error",
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "libtea",
    about = "Transparency Exchange API consumer (libtea-rs)",
    after_help = "Credentials: prefer TEA_API_KEY_ID and TEA_API_KEY_SECRET. \
CLI --api-key-secret is supported for compatibility but is insecure (process list / shell history). \
Do not mix a CLI id with an environment secret (or the reverse). Precedence: complete CLI pair, else complete environment pair, else anonymous.

Exit status: 0 success; 1 failure; 2 usage or bad input; 3 authentication or authorization; \
4 only weak (SHA-1) checksums matched; 5 not found or not resolved; 6 integrity not established; \
7 changes detected (sync --check, diff). With --json a failure is one JSON object on stderr. \
See docs/cli-exit-codes.md and docs/cli-json.md."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the pinned specification identity.
    Spec {
        /// Emit JSON on stdout.
        #[arg(long)]
        json: bool,
    },
    /// Parse a TEI and print its components. The spelling is not rewritten.
    ParseTei {
        tei: String,
        #[arg(long)]
        json: bool,
    },
    /// Run the offline smoke checks and print a JSON report (not a certification).
    Conformance {
        /// Accepted for uniformity: the report is always JSON.
        #[arg(long)]
        json: bool,
    },
    /// Resolve a TEI through `.well-known/tea` and `/discovery`.
    Discover {
        tei: String,
        #[command(flatten)]
        credentials: CredentialArgs,
        /// Send the API key to token endpoints learned from discovery.
        #[arg(long)]
        allow_discovered_credentials: bool,
        /// Fetch the well-known document from this URL instead of the TEI domain.
        #[arg(long)]
        well_known_url: Option<Url>,
        /// Allow HTTP and non-public addresses for this origin only.
        #[arg(long)]
        allow_origin: Option<Url>,
        #[command(flatten)]
        cache: CacheArgs,
        #[arg(long)]
        json: bool,
    },
    /// Fetch one product release from a known API base.
    ProductRelease {
        #[arg(long)]
        base: String,
        #[arg(long, default_value = "1.0.0")]
        api_version: String,
        #[arg(long)]
        uuid: String,
        #[command(flatten)]
        credentials: CredentialArgs,
        #[arg(long)]
        allow_origin: Option<Url>,
        #[command(flatten)]
        cache: CacheArgs,
        #[arg(long)]
        json: bool,
    },
    /// List or fetch Product Release collections (caller-managed pagination).
    Collections {
        #[arg(long)]
        base: String,
        #[arg(long, default_value = "1.0.0")]
        api_version: String,
        /// Product release UUID.
        #[arg(long)]
        uuid: String,
        /// Fetch this collection version instead of listing.
        #[arg(long)]
        version: Option<u64>,
        /// Fetch the latest collection instead of listing.
        #[arg(long)]
        latest: bool,
        #[arg(long)]
        page_size: Option<u32>,
        #[arg(long)]
        page_token: Option<String>,
        #[command(flatten)]
        credentials: CredentialArgs,
        #[arg(long)]
        allow_origin: Option<Url>,
        #[command(flatten)]
        cache: CacheArgs,
        #[arg(long)]
        json: bool,
    },
    /// Download one artifact revision to a file or stdout.
    Download {
        #[arg(long)]
        base: String,
        #[arg(long, default_value = "1.0.0")]
        api_version: String,
        #[arg(long)]
        artifact: String,
        /// Revision. Omit to use the latest-revision endpoint.
        #[arg(long)]
        revision: Option<u64>,
        #[arg(long)]
        media_type: Option<String>,
        /// Write the decoded bytes to this new file. Omit to write them to stdout.
        ///
        /// An existing path is refused unless --force is given. Only the
        /// final path component is checked: if it is a symbolic link the write
        /// is refused. Parent directories are not checked and may be symbolic
        /// links (for example `/tmp` on macOS), and the write goes through
        /// them. The file name is never taken from server data.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Replace an existing regular file at --output (written to a temporary
        /// file in the same directory, then renamed over it).
        #[arg(long, requires = "output")]
        force: bool,
        /// Accept a match when every declared checksum uses a weak algorithm
        /// (SHA-1). Without it such a download exits with status 4.
        #[arg(long)]
        allow_weak_checksums: bool,
        /// Fail when the format declares no checksums. Exit status 0 then means
        /// every declared checksum matched the decoded bytes.
        #[arg(long)]
        require_checksums: bool,
        /// Print one JSON object describing the download on stdout. Requires
        /// --output, so that stdout carries only the JSON.
        #[arg(long, requires = "output")]
        json: bool,
        #[command(flatten)]
        credentials: CredentialArgs,
        #[arg(long)]
        allow_origin: Option<Url>,
        #[command(flatten)]
        cache: CacheArgs,
    },
    /// Resolve a TEI, fetch and verify its artifacts, and write a lock file
    /// (`--check`: compare with the lock instead and exit 7 on differences).
    Sync {
        tei: String,
        /// The lock file to write, or with --check to compare against.
        #[arg(long)]
        lock: PathBuf,
        /// Do not write: exit 7 if the server differs from the lock.
        #[arg(long)]
        check: bool,
        /// Also keep the verified artifact bytes here, one file per format.
        #[arg(long, conflicts_with = "check")]
        artifacts_dir: Option<PathBuf>,
        /// Product release UUID, when the TEI resolves to several.
        #[arg(long)]
        product_release: Option<String>,
        /// Lock this collection version instead of the latest.
        #[arg(long)]
        collection_version: Option<u64>,
        /// Record this time (RFC 3339 UTC, `Z`) as fetchedAt instead of now.
        #[arg(long)]
        fetched_at: Option<String>,
        /// Accept artifacts whose declared checksums are all weak (SHA-1).
        #[arg(long)]
        allow_weak_checksums: bool,
        /// Send the API key to token endpoints learned from discovery.
        #[arg(long)]
        allow_discovered_credentials: bool,
        /// Fetch the well-known document from this URL instead of the TEI domain.
        #[arg(long)]
        well_known_url: Option<Url>,
        /// Allow HTTP and non-public addresses for this origin only.
        #[arg(long)]
        allow_origin: Option<Url>,
        #[command(flatten)]
        credentials: CredentialArgs,
        #[command(flatten)]
        cache: CacheArgs,
        #[arg(long)]
        json: bool,
    },
    /// Compare two lock files offline; exit 7 if they differ (fetchedAt is ignored).
    Diff {
        old: PathBuf,
        new: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Inspect or empty the persistent response cache.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Debug, Subcommand)]
enum CacheAction {
    /// Count entries and bytes without changing anything.
    Stat {
        /// The cache directory. Defaults to $LIBTEA_CACHE_DIR.
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Remove every cache entry. Other files in the directory are left alone.
    Clear {
        /// The cache directory. Defaults to $LIBTEA_CACHE_DIR.
        #[arg(long)]
        cache_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

impl Command {
    /// Whether the user asked for JSON output.
    fn json(&self) -> bool {
        match self {
            Command::Spec { json }
            | Command::ParseTei { json, .. }
            | Command::Conformance { json }
            | Command::Discover { json, .. }
            | Command::ProductRelease { json, .. }
            | Command::Collections { json, .. }
            | Command::Download { json, .. }
            | Command::Sync { json, .. }
            | Command::Diff { json, .. } => *json,
            Command::Cache { action } => match action {
                CacheAction::Stat { json, .. } | CacheAction::Clear { json, .. } => *json,
            },
        }
    }
}

/// Persistent cache options (`docs/cache.md`). Off unless a directory is given.
#[derive(Debug, Clone, clap::Args)]
struct CacheArgs {
    /// Keep a response cache in this directory (created private, mode 0700).
    /// Defaults to $LIBTEA_CACHE_DIR when that is set.
    #[arg(long)]
    cache_dir: Option<PathBuf>,
    /// Answer only from the cache and never connect. Needs a cache directory;
    /// a response that is not cached fails.
    #[arg(long)]
    offline: bool,
    /// Also cache responses to credentialed requests and `private` responses.
    #[arg(long)]
    cache_authenticated: bool,
}

const ENV_CACHE_DIR: &str = "LIBTEA_CACHE_DIR";

impl CacheArgs {
    fn dir(&self) -> Option<PathBuf> {
        self.cache_dir.clone().or_else(|| {
            std::env::var_os(ENV_CACHE_DIR)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        })
    }
}

/// Apply the cache options to a client builder.
fn with_cache(builder: ClientBuilder, args: &CacheArgs) -> anyhow::Result<ClientBuilder> {
    let Some(dir) = args.dir() else {
        if args.offline {
            return Err(usage(
                "--offline needs a cache directory (--cache-dir or LIBTEA_CACHE_DIR)",
            ));
        }
        if args.cache_authenticated {
            return Err(usage(
                "--cache-authenticated needs a cache directory (--cache-dir or LIBTEA_CACHE_DIR)",
            ));
        }
        return Ok(builder);
    };
    let config = libtea_client::CacheConfig::new(dir).with_authenticated(args.cache_authenticated);
    Ok(builder.cache(config).offline(args.offline))
}

#[derive(Debug, clap::Args)]
struct CredentialArgs {
    /// API key identifier. Prefer TEA_API_KEY_ID.
    #[arg(long)]
    api_key_id: Option<String>,
    /// API key secret (insecure on argv). Prefer TEA_API_KEY_SECRET.
    #[arg(long)]
    api_key_secret: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.command.json();
    match run(cli).await {
        Ok(()) => ExitCode::from(exit::OK),
        Err(error) => {
            let (status, kind) = classify(&error);
            if json {
                eprintln!(
                    "{}",
                    serde_json::json!({
                        "schema_version": 1,
                        "error": {
                            "kind": kind,
                            "exit_code": status,
                            "message": clean(&format!("{error:#}")),
                        },
                    })
                );
            } else {
                err!("Error: {error:?}");
            }
            ExitCode::from(status)
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Spec { json } => {
            if json {
                out!(
                    "{}",
                    serde_json::json!({
                        "repository": spec::REPOSITORY,
                        "branch": spec::BRANCH,
                        "commit": spec::COMMIT,
                        "commit_date": spec::COMMIT_DATE,
                        "retrieval_date": spec::RETRIEVAL_DATE,
                        "spec_version": spec::SPEC_VERSION,
                        "openapi": spec::OPENAPI_VERSION,
                    })
                );
            } else {
                out!(
                    "repository {}\nbranch {}\ncommit {}\ncommit_date {}\nretrieval_date {}\nspec_version {}\nopenapi {}",
                    spec::REPOSITORY,
                    spec::BRANCH,
                    spec::COMMIT,
                    spec::COMMIT_DATE,
                    spec::RETRIEVAL_DATE,
                    spec::SPEC_VERSION,
                    spec::OPENAPI_VERSION
                );
            }
        }
        Command::ParseTei { tei, json } => {
            let parsed = Tei::parse(&tei)?;
            parsed.validate_registry()?;
            if json {
                out!(
                    "{}",
                    serde_json::json!({
                        "domain": parsed.domain(),
                        "type": parsed.tei_type(),
                        "identifier": parsed.identifier(),
                        "well_known": parsed.well_known_url(),
                        "publication_issues": parsed.publication_issues(),
                    })
                );
            } else {
                out!("domain {}", parsed.domain());
                out!("type {}", parsed.tei_type());
                out!("identifier {}", parsed.identifier());
                out!("well_known {}", parsed.well_known_url());
                for issue in parsed.publication_issues() {
                    out!("publication_issue {issue}");
                }
            }
        }
        Command::Conformance { .. } => {
            let report = libtea_conformance::static_report();
            out!("{}", report.to_json()?);
            if !report.passed() {
                return Err(coded(exit::FAILURE, "conformance checks failed"));
            }
        }
        Command::Discover {
            tei,
            credentials,
            allow_discovered_credentials,
            well_known_url,
            allow_origin,
            cache,
            json,
        } => {
            let parsed = Tei::parse(&tei)?;
            let mut builder = with_cache(
                ClientBuilder::new().allow_discovered_credentials(allow_discovered_credentials),
                &cache,
            )?;
            if let Some((id, secret)) = resolve_api_key(&credentials)? {
                builder = builder.api_key(ApiKey::new(id, secret));
            }
            if let Some(origin) = &allow_origin {
                builder = builder
                    .destination_policy(libtea_core::DestinationPolicy::allow_origin(origin)?);
            }
            if let Some(url) = well_known_url {
                builder = builder.well_known_fetch_url(url);
            }
            let releases = builder.build()?.discover(&parsed).await?;
            if json {
                let rows: Vec<_> = releases
                    .iter()
                    .map(|release| {
                        serde_json::json!({
                            "product_release": release.product_release.to_string(),
                            "root_url": release.root_url,
                            "api_version": release.api_version,
                        })
                    })
                    .collect();
                out!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                for release in releases {
                    out!(
                        "{}\t{}\t{}",
                        release.product_release,
                        release.root_url,
                        release.api_version
                    );
                }
            }
        }
        Command::ProductRelease {
            base,
            api_version,
            uuid,
            credentials,
            allow_origin,
            cache,
            json,
        } => {
            let client =
                with_cache(configured_builder(&credentials, allow_origin)?, &cache)?.build()?;
            let service = client.service(&base, &api_version)?;
            let id = libtea_core::ProductReleaseId::parse(&uuid)?;
            let release = service.get_product_release(id).await?;
            if json {
                out!("{}", serde_json::to_string(&release)?);
            } else {
                out!("{}", serde_json::to_string_pretty(&release)?);
            }
        }
        Command::Collections {
            base,
            api_version,
            uuid,
            version,
            latest,
            page_size,
            page_token,
            credentials,
            allow_origin,
            cache,
            json,
        } => {
            if latest && version.is_some() {
                return Err(usage("pass only one of --latest and --version"));
            }
            let client =
                with_cache(configured_builder(&credentials, allow_origin)?, &cache)?.build()?;
            let service = client.service(&base, &api_version)?;
            let id = libtea_core::ProductReleaseId::parse(&uuid)?;
            if latest {
                let collection = service.get_latest_product_release_collection(id).await?;
                print_collection(&collection, json)?;
            } else if let Some(version) = version {
                let collection = service.get_product_release_collection(id, version).await?;
                print_collection(&collection, json)?;
            } else {
                let mut query = ListParams::default();
                query.page_size = page_size;
                query.page_token = page_token;
                let page = service.list_product_release_collections(id, &query).await?;
                if json {
                    out!("{}", serde_json::to_string_pretty(&page)?);
                } else {
                    out!(
                        "has_next {}\tnext_page_token {}",
                        page.has_next,
                        page.next_page_token.as_deref().unwrap_or("")
                    );
                    for collection in &page.results {
                        out!(
                            "version {}\tartifacts {}\tbelongs_to {}",
                            collection.version,
                            collection.artifacts.len(),
                            collection.belongs_to.as_str()
                        );
                    }
                }
            }
        }
        Command::Download {
            base,
            api_version,
            artifact,
            revision,
            media_type,
            output,
            force,
            allow_weak_checksums,
            require_checksums,
            json,
            credentials,
            allow_origin,
            cache,
        } => {
            let client = with_cache(configured_builder(&credentials, allow_origin)?, &cache)?
                .allow_weak_checksums(allow_weak_checksums)
                .build()?;
            let service = client.service(&base, &api_version)?;
            let id = libtea_core::ArtifactId::parse(&artifact)?;
            let metadata = if let Some(revision) = revision {
                service.get_artifact(id, revision).await?
            } else {
                service.get_latest_artifact(id).await?
            };
            let selected = match media_type {
                Some(media_type) => media_type,
                None if metadata.formats.len() == 1 => metadata.formats[0].media_type.clone(),
                None => {
                    return Err(usage(format!(
                        "artifact has {} formats; pass --media-type",
                        metadata.formats.len()
                    )))
                }
            };
            // No declared checksum means the outcome could only be NotDeclared,
            // so refuse before any bytes are fetched or written.
            if require_checksums
                && metadata
                    .format(&selected)
                    .is_some_and(|format| format.checksums.as_ref().is_none_or(Vec::is_empty))
            {
                return Err(coded(
                    exit::INTEGRITY,
                    "checksums were not declared for this format and --require-checksums is set",
                ));
            }
            if let Some(path) = output {
                check_output_target(&path, force)?;
                let options = libtea_client::FileDownloadOptions::new().with_overwrite(force);
                let result = service
                    .download_format_to_file(&metadata, &selected, &path, &options)
                    .await
                    .map_err(map_download_error)?;
                let download = &result.download;
                note_integrity(
                    &download.integrity,
                    download.strong_etag,
                    download.bytes_written,
                );
                if json {
                    out!(
                        "{}",
                        download_json(&artifact, revision, &selected, &path, download,)
                    );
                }
            } else {
                // Stdout cannot take bytes back, so this path holds the whole
                // body in memory and prints it only after the check.
                let fetched = service
                    .download_format(&metadata, &selected, None)
                    .await
                    .map_err(map_download_error)?;
                let ArtifactFetch::Body(body) = fetched else {
                    return Err(coded(exit::FAILURE, "artifact was not modified"));
                };
                note_integrity(&body.integrity, body.strong_etag, body.bytes.len() as u64);
                std::io::stdout().write_all(&body.bytes)?;
            }
        }
        Command::Sync {
            tei,
            lock,
            check,
            artifacts_dir,
            product_release,
            collection_version,
            fetched_at,
            allow_weak_checksums,
            allow_discovered_credentials,
            well_known_url,
            allow_origin,
            credentials,
            cache,
            json,
        } => {
            sync::sync(sync::SyncOptions {
                tei,
                lock,
                check,
                artifacts_dir,
                product_release,
                collection_version,
                fetched_at,
                allow_weak_checksums,
                allow_discovered_credentials,
                well_known_url,
                allow_origin,
                credentials,
                cache,
                json,
            })
            .await?;
        }
        Command::Diff { old, new, json } => sync::diff(&old, &new, json)?,
        Command::Cache { action } => cache_command(action)?,
    }
    Ok(())
}

/// A weak-only match gets its own exit status; everything else keeps its type.
fn map_download_error(error: ClientError) -> anyhow::Error {
    match error {
        ClientError::Integrity {
            outcome: IntegrityOutcome::Weak,
            ..
        } => coded(exit::WEAK_CHECKSUMS, WEAK_MESSAGE),
        other => other.into(),
    }
}

/// Say on stderr what the integrity check did and did not establish.
fn note_integrity(report: &libtea_core::IntegrityReport, strong_etag: bool, bytes: u64) {
    match report.outcome {
        IntegrityOutcome::Verified if report.checks.iter().all(|check| check.weak) => {
            err!(
                "integrity verified with weak algorithms only, accepted by --allow-weak-checksums (decoded-byte checksums only; not publisher authenticity)\tstrong_etag {strong_etag}\tbytes {bytes}"
            );
        }
        IntegrityOutcome::Verified => {
            err!(
                "integrity verified (decoded-byte checksums only; not publisher authenticity)\tstrong_etag {strong_etag}\tbytes {bytes}"
            );
        }
        _ => {
            err!(
                "checksums were not declared for this format; decoded-byte checksums were not checked"
            );
        }
    }
}

/// Name of an outcome for JSON. `Unknown` covers outcomes added later.
fn outcome_name(outcome: &IntegrityOutcome) -> &'static str {
    match outcome {
        IntegrityOutcome::Verified => "Verified",
        IntegrityOutcome::Failed => "Failed",
        IntegrityOutcome::Unsupported => "Unsupported",
        IntegrityOutcome::Incomplete => "Incomplete",
        IntegrityOutcome::NotDeclared => "NotDeclared",
        IntegrityOutcome::Weak => "Weak",
        _ => "Unknown",
    }
}

/// Strip userinfo, query and fragment: a final URL may carry a signed query.
fn redact_url(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut parsed) => {
            let _ = parsed.set_username("");
            let _ = parsed.set_password(None);
            parsed.set_query(None);
            parsed.set_fragment(None);
            parsed.to_string()
        }
        Err(_) => "(unparseable url)".to_owned(),
    }
}

/// The `download --json` object. Fields are documented in `docs/cli-json.md`.
fn download_json(
    artifact: &str,
    revision: Option<u64>,
    media_type: &str,
    output: &Path,
    download: &libtea_client::StreamedDownload,
) -> serde_json::Value {
    let checks: Vec<_> = download
        .integrity
        .checks
        .iter()
        .map(|check| {
            serde_json::json!({
                "algorithm": check.algorithm,
                "expected": check.expected,
                "actual": check.actual,
                "supported": check.supported,
                "matched": check.matched,
                "weak": check.weak,
            })
        })
        .collect();
    serde_json::json!({
        "schema_version": 1,
        "artifact": artifact,
        "revision": revision,
        "media_type": media_type,
        "output": output.display().to_string(),
        "bytes_written": download.bytes_written,
        "content_type": download.content_type,
        "content_encoding": download.content_encoding,
        "etag": download.etag,
        "strong_etag": download.strong_etag,
        "final_url": redact_url(&download.final_url),
        "integrity": {
            "outcome": outcome_name(&download.integrity.outcome),
            "scope": "decoded artifact bytes; not publisher authenticity",
            "checks": checks,
        },
    })
}

/// `libtea cache stat` and `libtea cache clear`.
fn cache_command(action: CacheAction) -> anyhow::Result<()> {
    let (dir, json, clear) = match action {
        CacheAction::Stat { cache_dir, json } => (cache_dir, json, false),
        CacheAction::Clear { cache_dir, json } => (cache_dir, json, true),
    };
    let dir = dir
        .or_else(|| {
            std::env::var_os(ENV_CACHE_DIR)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        })
        .ok_or_else(|| usage("pass --cache-dir or set LIBTEA_CACHE_DIR"))?;
    if clear {
        let removed = libtea_client::cache_clear(&dir)?;
        if json {
            out!(
                "{}",
                serde_json::json!({
                    "schema_version": 1,
                    "dir": dir.display().to_string(),
                    "removed": removed,
                })
            );
        } else {
            out!("removed {removed} files from {}", dir.display());
        }
    } else {
        let stats = libtea_client::cache_stat(&dir)?;
        if json {
            out!(
                "{}",
                serde_json::json!({
                    "schema_version": 1,
                    "dir": stats.dir.display().to_string(),
                    "exists": stats.exists,
                    "entries": stats.entries,
                    "bytes": stats.bytes,
                    "foreign": stats.foreign,
                    "oldest_age_secs": stats.oldest_age_secs,
                    "newest_age_secs": stats.newest_age_secs,
                })
            );
        } else {
            out!("dir {}", stats.dir.display());
            out!("exists {}", stats.exists);
            out!("entries {}", stats.entries);
            out!("bytes {}", stats.bytes);
            out!("foreign {}", stats.foreign);
            if let Some(age) = stats.oldest_age_secs {
                out!("oldest_age_secs {age}");
            }
            if let Some(age) = stats.newest_age_secs {
                out!("newest_age_secs {age}");
            }
        }
    }
    Ok(())
}

fn print_collection(collection: &libtea_core::Collection, json: bool) -> anyhow::Result<()> {
    if json {
        out!("{}", serde_json::to_string_pretty(collection)?);
    } else {
        out!(
            "version {}\tartifacts {}\tbelongs_to {}\tcreated {}",
            collection.version,
            collection.artifacts.len(),
            collection.belongs_to.as_str(),
            collection.created_date
        );
    }
    Ok(())
}

/// Resolve credentials without mixing CLI and environment sources.
///
/// Precedence: a complete CLI `--api-key-id` + `--api-key-secret` pair wins;
/// otherwise a complete `TEA_API_KEY_ID` + `TEA_API_KEY_SECRET` pair; otherwise
/// anonymous. A partial pair from either source is an error. Empty strings are
/// treated as absent.
fn resolve_api_key(args: &CredentialArgs) -> anyhow::Result<Option<(String, String)>> {
    let cli_id = nonempty(args.api_key_id.as_deref());
    let cli_secret = nonempty(args.api_key_secret.as_deref());
    match (cli_id, cli_secret) {
        (Some(id), Some(secret)) => Ok(Some((id.to_owned(), secret.to_owned()))),
        (None, None) => {
            let env_id = match std::env::var(ENV_API_KEY_ID) {
                Ok(value) => nonempty(Some(value.as_str())).map(str::to_owned),
                Err(_) => None,
            };
            let env_secret = match std::env::var(ENV_API_KEY_SECRET) {
                Ok(value) => nonempty(Some(value.as_str())).map(str::to_owned),
                Err(_) => None,
            };
            match (env_id, env_secret) {
                (Some(id), Some(secret)) => Ok(Some((id, secret))),
                (None, None) => Ok(None),
                (Some(_), None) => Err(usage(format!(
                    "{ENV_API_KEY_ID} is set but {ENV_API_KEY_SECRET} is missing or empty"
                ))),
                (None, Some(_)) => Err(usage(format!(
                    "{ENV_API_KEY_SECRET} is set but {ENV_API_KEY_ID} is missing or empty"
                ))),
            }
        }
        (Some(_), None) => Err(usage(format!(
            "--api-key-id requires --api-key-secret (or omit both and use {ENV_API_KEY_ID}/{ENV_API_KEY_SECRET})"
        ))),
        (None, Some(_)) => Err(usage(format!(
            "--api-key-secret requires --api-key-id (prefer {ENV_API_KEY_ID}/{ENV_API_KEY_SECRET})"
        ))),
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

/// Refuse an `--output` path early, with CLI wording. The library checks the
/// same things again when it commits the file.
///
/// Without `--force` the path must not exist; with it an existing regular file
/// is replaced by renaming a finished temporary file from the same directory
/// over it. A symbolic link at the path is always refused (`rename` would
/// replace the link itself, never its target).
fn check_output_target(path: &Path, force: bool) -> anyhow::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(coded(
            exit::FAILURE,
            format!(
                "refusing to write {}: it is a symbolic link",
                path.display()
            ),
        )),
        Ok(_) if !force => Err(coded(
            exit::FAILURE,
            format!(
                "{} already exists; pass --force to replace it",
                path.display()
            ),
        )),
        Ok(meta) if !meta.is_file() => Err(coded(
            exit::FAILURE,
            format!("{} exists and is not a regular file", path.display()),
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("inspecting {}", path.display())),
    }
}

fn configured_builder(
    credentials: &CredentialArgs,
    allow_origin: Option<Url>,
) -> anyhow::Result<ClientBuilder> {
    let mut builder = ClientBuilder::new();
    if let Some((id, secret)) = resolve_api_key(credentials)? {
        builder = builder.api_key(ApiKey::new(id, secret));
    }
    if let Some(origin) = allow_origin {
        builder =
            builder.destination_policy(libtea_core::DestinationPolicy::allow_origin(&origin)?);
    }
    Ok(builder)
}

#[cfg(test)]
mod resolve_tests {
    use super::*;
    use std::sync::Mutex;

    // Process environment is global; serialize these cases and always restore
    // the caller's TEA_API_KEY_* values so `cargo test` cannot wipe them.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        id: Option<String>,
        secret: Option<String>,
    }

    impl EnvGuard {
        fn take() -> Self {
            let id = std::env::var(ENV_API_KEY_ID).ok();
            let secret = std::env::var(ENV_API_KEY_SECRET).ok();
            std::env::remove_var(ENV_API_KEY_ID);
            std::env::remove_var(ENV_API_KEY_SECRET);
            Self { id, secret }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.id.take() {
                Some(value) => std::env::set_var(ENV_API_KEY_ID, value),
                None => std::env::remove_var(ENV_API_KEY_ID),
            }
            match self.secret.take() {
                Some(value) => std::env::set_var(ENV_API_KEY_SECRET, value),
                None => std::env::remove_var(ENV_API_KEY_SECRET),
            }
        }
    }

    #[test]
    fn cli_pair_wins_over_environment() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _env = EnvGuard::take();
        std::env::set_var(ENV_API_KEY_ID, "env-id");
        std::env::set_var(ENV_API_KEY_SECRET, "env-secret");
        let args = CredentialArgs {
            api_key_id: Some("cli-id".into()),
            api_key_secret: Some("cli-secret".into()),
        };
        let resolved = resolve_api_key(&args).unwrap().unwrap();
        assert_eq!(resolved.0, "cli-id");
        assert_eq!(resolved.1, "cli-secret");
    }

    #[test]
    fn environment_pair_when_cli_absent() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _env = EnvGuard::take();
        std::env::set_var(ENV_API_KEY_ID, "env-id");
        std::env::set_var(ENV_API_KEY_SECRET, "env-secret");
        let args = CredentialArgs {
            api_key_id: None,
            api_key_secret: None,
        };
        let resolved = resolve_api_key(&args).unwrap().unwrap();
        assert_eq!(resolved.0, "env-id");
        assert_eq!(resolved.1, "env-secret");
    }

    #[test]
    fn partial_cli_is_rejected() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _env = EnvGuard::take();
        let args = CredentialArgs {
            api_key_id: Some("only-id".into()),
            api_key_secret: None,
        };
        assert!(resolve_api_key(&args).is_err());
    }

    #[test]
    fn partial_environment_is_rejected() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _env = EnvGuard::take();
        std::env::set_var(ENV_API_KEY_ID, "env-id");
        let args = CredentialArgs {
            api_key_id: None,
            api_key_secret: None,
        };
        assert!(resolve_api_key(&args).is_err());
    }

    #[test]
    fn empty_strings_are_absent() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _env = EnvGuard::take();
        let args = CredentialArgs {
            api_key_id: Some("".into()),
            api_key_secret: Some("".into()),
        };
        assert!(resolve_api_key(&args).unwrap().is_none());
    }

    #[test]
    fn cli_credentials_preserve_leading_and_trailing_spaces() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _env = EnvGuard::take();
        let args = CredentialArgs {
            api_key_id: Some(" id ".into()),
            api_key_secret: Some(" secret ".into()),
        };
        let resolved = resolve_api_key(&args).unwrap().unwrap();
        assert_eq!(resolved.0, " id ");
        assert_eq!(resolved.1, " secret ");
    }

    #[test]
    fn environment_credentials_preserve_leading_and_trailing_spaces() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let _env = EnvGuard::take();
        std::env::set_var(ENV_API_KEY_ID, " env-id ");
        std::env::set_var(ENV_API_KEY_SECRET, " env-secret ");
        let args = CredentialArgs {
            api_key_id: None,
            api_key_secret: None,
        };
        let resolved = resolve_api_key(&args).unwrap().unwrap();
        assert_eq!(resolved.0, " env-id ");
        assert_eq!(resolved.1, " env-secret ");
    }
}

#[cfg(test)]
mod clean_tests {
    use super::clean;

    #[test]
    fn controls_are_escaped_and_ordinary_text_is_kept() {
        assert_eq!(
            clean("plain\ttab\nnewline é 日本"),
            "plain\ttab\nnewline é 日本"
        );
        assert_eq!(clean("a\u{1b}[31mred"), "a\\u001b[31mred");
        assert_eq!(
            clean("osc\u{1b}]0;title\u{7}x"),
            "osc\\u001b]0;title\\u0007x"
        );
        assert_eq!(clean("cr\rover"), "cr\\u000dover");
        assert_eq!(clean("del\u{7f}"), "del\\u007f");
        assert_eq!(clean("c1\u{9b}31m \u{85}"), "c1\\u009b31m \\u0085");
        assert_eq!(clean("\u{202e}rtl\u{2066}"), "\\u202ertl\\u2066");
    }

    #[test]
    fn escaped_text_never_contains_a_control_character() {
        let all: String = (0u32..0x10000).filter_map(char::from_u32).collect();
        let cleaned = clean(&all);
        assert!(cleaned
            .chars()
            .all(|c| !c.is_control() || c == '\t' || c == '\n'));
    }
}
