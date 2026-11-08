//! Command-line tool for the TEA consumer libraries (`libtea-cli`).
//!
//! Protocol behavior lives in `libtea-core` and `libtea-client`. This binary only
//! parses arguments and prints results.

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{anyhow, Context};
use clap::{Parser, Subcommand};
use libtea_client::{ApiKey, ArtifactFetch, Client, ClientBuilder, ClientError};
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

const ENV_API_KEY_ID: &str = "TEA_API_KEY_ID";
const ENV_API_KEY_SECRET: &str = "TEA_API_KEY_SECRET";

/// Exit status when every declared checksum matched but all of them use weak
/// algorithms (SHA-1) and `--allow-weak-checksums` was not given.
const EXIT_WEAK_CHECKSUMS: u8 = 4;

/// Marker error for [`EXIT_WEAK_CHECKSUMS`].
#[derive(Debug)]
struct WeakChecksums;

impl std::fmt::Display for WeakChecksums {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "only weak checksum algorithms (SHA-1) were declared for this format; \
             that is not Verified. Pass --allow-weak-checksums to accept it",
        )
    }
}

impl std::error::Error for WeakChecksums {}

#[derive(Debug, Parser)]
#[command(
    name = "libtea",
    about = "Transparency Exchange API consumer (libtea-rs)",
    after_help = "Credentials: prefer TEA_API_KEY_ID and TEA_API_KEY_SECRET. \
CLI --api-key-secret is supported for compatibility but is insecure (process list / shell history). \
Do not mix a CLI id with an environment secret (or the reverse). Precedence: complete CLI pair, else complete environment pair, else anonymous."
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
    Conformance,
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
        #[command(flatten)]
        credentials: CredentialArgs,
        #[arg(long)]
        allow_origin: Option<Url>,
    },
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
    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            err!("Error: {error:?}");
            if error.downcast_ref::<WeakChecksums>().is_some() {
                ExitCode::from(EXIT_WEAK_CHECKSUMS)
            } else {
                ExitCode::FAILURE
            }
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
        Command::Conformance => {
            let report = libtea_conformance::static_report();
            out!("{}", report.to_json()?);
            if !report.passed() {
                anyhow::bail!("conformance checks failed");
            }
        }
        Command::Discover {
            tei,
            credentials,
            allow_discovered_credentials,
            well_known_url,
            allow_origin,
            json,
        } => {
            let parsed = Tei::parse(&tei)?;
            let mut builder =
                ClientBuilder::new().allow_discovered_credentials(allow_discovered_credentials);
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
            json,
        } => {
            let client = configured_client(&credentials, allow_origin)?;
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
            json,
        } => {
            if latest && version.is_some() {
                anyhow::bail!("pass only one of --latest and --version");
            }
            let client = configured_client(&credentials, allow_origin)?;
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
            credentials,
            allow_origin,
        } => {
            let client = configured_builder(&credentials, allow_origin)?
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
                None => anyhow::bail!(
                    "artifact has {} formats; pass --media-type",
                    metadata.formats.len()
                ),
            };
            let fetched = match service.download_format(&metadata, &selected, None).await {
                Ok(fetched) => fetched,
                Err(ClientError::Integrity {
                    outcome: IntegrityOutcome::Weak,
                    ..
                }) => return Err(WeakChecksums.into()),
                Err(other) => return Err(other.into()),
            };
            let ArtifactFetch::Body(body) = fetched else {
                anyhow::bail!("artifact was not modified");
            };
            match body.integrity.outcome {
                IntegrityOutcome::Verified
                    if body.integrity.checks.iter().all(|check| check.weak) =>
                {
                    err!(
                        "integrity verified with weak algorithms only, accepted by --allow-weak-checksums (decoded-byte checksums only; not publisher authenticity)\tstrong_etag {}\tbytes {}",
                        body.strong_etag,
                        body.bytes.len()
                    );
                }
                IntegrityOutcome::Verified => {
                    err!(
                        "integrity verified (decoded-byte checksums only; not publisher authenticity)\tstrong_etag {}\tbytes {}",
                        body.strong_etag,
                        body.bytes.len()
                    );
                }
                IntegrityOutcome::NotDeclared if require_checksums => {
                    anyhow::bail!(
                        "checksums were not declared for this format and --require-checksums is set"
                    );
                }
                IntegrityOutcome::NotDeclared => {
                    err!(
                        "checksums were not declared for this format; decoded-byte checksums were not checked"
                    );
                }
                other => {
                    anyhow::bail!("decoded-byte checksum integrity was not Verified ({other:?})")
                }
            }
            if let Some(path) = output {
                write_output(&path, &body.bytes, force)?;
            } else {
                std::io::stdout().write_all(&body.bytes)?;
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
                (Some(_), None) => Err(anyhow!(
                    "{ENV_API_KEY_ID} is set but {ENV_API_KEY_SECRET} is missing or empty"
                )),
                (None, Some(_)) => Err(anyhow!(
                    "{ENV_API_KEY_SECRET} is set but {ENV_API_KEY_ID} is missing or empty"
                )),
            }
        }
        (Some(_), None) => Err(anyhow!(
            "--api-key-id requires --api-key-secret (or omit both and use {ENV_API_KEY_ID}/{ENV_API_KEY_SECRET})"
        )),
        (None, Some(_)) => Err(anyhow!(
            "--api-key-secret requires --api-key-id (prefer {ENV_API_KEY_ID}/{ENV_API_KEY_SECRET})"
        )),
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

/// Write `bytes` to a caller-chosen path without following symlinks.
///
/// Without `force` the file must not exist (`create_new`, so a path that
/// appears between the check and the open is refused too). With `force` an
/// existing regular file is replaced by renaming a temporary file from the
/// same directory over it; `rename` replaces a link itself, never its target.
fn write_output(path: &Path, bytes: &[u8], force: bool) -> anyhow::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            anyhow::bail!(
                "refusing to write {}: it is a symbolic link",
                path.display()
            )
        }
        Ok(_) if !force => anyhow::bail!(
            "{} already exists; pass --force to replace it",
            path.display()
        ),
        Ok(meta) if !meta.is_file() => {
            anyhow::bail!("{} exists and is not a regular file", path.display())
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).with_context(|| format!("inspecting {}", path.display()));
        }
    }
    if !force {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .with_context(|| format!("creating {}", path.display()))?;
        if let Err(err) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(err).with_context(|| format!("writing {}", path.display()));
        }
        return Ok(());
    }
    let name = path
        .file_name()
        .ok_or_else(|| anyhow!("{} has no file name", path.display()))?;
    let dir = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(".libtea-{}.tmp", std::process::id()));
    let temp = dir.join(temp_name);
    let result = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.with_context(|| format!("replacing {}", path.display()))
}

fn configured_client(
    credentials: &CredentialArgs,
    allow_origin: Option<Url>,
) -> anyhow::Result<Client> {
    Ok(configured_builder(credentials, allow_origin)?.build()?)
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
