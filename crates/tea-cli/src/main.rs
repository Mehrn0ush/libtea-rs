//! Command line for the TEA consumer SDK.
//!
//! Protocol behavior lives in `cdx-tea-core` and `cdx-tea-client`. This binary only
//! parses arguments and prints results.

#![forbid(unsafe_code)]

use std::path::PathBuf;

use anyhow::{anyhow, Context};
use cdx_tea_client::{ApiKey, ArtifactFetch, Client, ClientBuilder};
use cdx_tea_core::{spec, IntegrityOutcome, Tei};
use clap::{Parser, Subcommand};
use url::Url;

#[derive(Debug, Parser)]
#[command(name = "tea", about = "Transparency Exchange API consumer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the pinned specification identity.
    Spec,
    /// Parse a TEI and print its components. The spelling is not rewritten.
    ParseTei { tei: String },
    /// Run the offline conformance checks and print a JSON report.
    Conformance,
    /// Resolve a TEI through `.well-known/tea` and `/discovery`.
    Discover {
        tei: String,
        /// API key identifier. The secret is read from `--api-key-secret`.
        #[arg(long)]
        api_key_id: Option<String>,
        /// API key secret. It is not printed.
        #[arg(long)]
        api_key_secret: Option<String>,
        /// Send the API key to token endpoints learned from discovery.
        #[arg(long)]
        allow_discovered_credentials: bool,
        /// Fetch the well-known document from this URL instead of the TEI domain.
        #[arg(long)]
        well_known_url: Option<Url>,
        /// Allow HTTP and non-public addresses for this origin only.
        #[arg(long)]
        allow_origin: Option<Url>,
    },
    /// Fetch one product release from a known API base.
    ProductRelease {
        #[arg(long)]
        base: String,
        #[arg(long, default_value = "1.0.0")]
        api_version: String,
        #[arg(long)]
        uuid: String,
        #[arg(long)]
        api_key_id: Option<String>,
        #[arg(long)]
        api_key_secret: Option<String>,
        #[arg(long)]
        allow_origin: Option<Url>,
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
        /// Write the decoded bytes here. Omit to write them to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        api_key_id: Option<String>,
        #[arg(long)]
        api_key_secret: Option<String>,
        #[arg(long)]
        allow_origin: Option<Url>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Spec => {
            println!(
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
        Command::ParseTei { tei } => {
            let parsed = Tei::parse(&tei)?;
            parsed.validate_registry()?;
            println!("domain {}", parsed.domain());
            println!("type {}", parsed.tei_type());
            println!("identifier {}", parsed.identifier());
            println!("well_known {}", parsed.well_known_url());
            for issue in parsed.publication_issues() {
                println!("publication_issue {issue}");
            }
        }
        Command::Conformance => {
            let report = cdx_tea_conformance::static_report();
            println!("{}", report.to_json()?);
            if !report.passed() {
                anyhow::bail!("conformance checks failed");
            }
        }
        Command::Discover {
            tei,
            api_key_id,
            api_key_secret,
            allow_discovered_credentials,
            well_known_url,
            allow_origin,
        } => {
            let parsed = Tei::parse(&tei)?;
            let mut builder =
                ClientBuilder::new().allow_discovered_credentials(allow_discovered_credentials);
            if let Some((id, secret)) = api_key_from_parts(&api_key_id, &api_key_secret)? {
                builder = builder.api_key(ApiKey::new(id, secret));
            }
            if let Some(origin) = &allow_origin {
                builder = builder
                    .destination_policy(cdx_tea_core::DestinationPolicy::allow_origin(origin)?);
            }
            if let Some(url) = well_known_url {
                builder = builder.well_known_fetch_url(url);
            }
            let releases = builder.build()?.discover(&parsed).await?;
            for release in releases {
                println!(
                    "{}\t{}\t{}",
                    release.product_release, release.root_url, release.api_version
                );
            }
        }
        Command::ProductRelease {
            base,
            api_version,
            uuid,
            api_key_id,
            api_key_secret,
            allow_origin,
        } => {
            let client = configured_client(api_key_id, api_key_secret, allow_origin)?;
            let service = client.service(&base, &api_version)?;
            let id = cdx_tea_core::ProductReleaseId::parse(&uuid)?;
            let release = service.get_product_release(id).await?;
            println!("{}", serde_json::to_string_pretty(&release)?);
        }
        Command::Download {
            base,
            api_version,
            artifact,
            revision,
            media_type,
            output,
            api_key_id,
            api_key_secret,
            allow_origin,
        } => {
            let client = configured_client(api_key_id, api_key_secret, allow_origin)?;
            let service = client.service(&base, &api_version)?;
            let id = cdx_tea_core::ArtifactId::parse(&artifact)?;
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
            let fetched = service.download_format(&metadata, &selected, None).await?;
            let ArtifactFetch::Body(body) = fetched else {
                anyhow::bail!("artifact was not modified");
            };
            match body.integrity.outcome {
                IntegrityOutcome::Verified => {
                    eprintln!(
                        "integrity verified\tstrong_etag {}\tbytes {}",
                        body.strong_etag,
                        body.bytes.len()
                    );
                }
                IntegrityOutcome::NotDeclared => {
                    eprintln!(
                        "checksums were not declared for this format; bytes were not verified"
                    );
                }
                other => anyhow::bail!("artifact integrity was not verified ({other:?})"),
            }
            if let Some(path) = output {
                std::fs::write(&path, &body.bytes)
                    .with_context(|| format!("writing {}", path.display()))?;
            } else {
                use std::io::Write;
                std::io::stdout().write_all(&body.bytes)?;
            }
        }
    }
    Ok(())
}

fn api_key_from_parts(
    id: &Option<String>,
    secret: &Option<String>,
) -> anyhow::Result<Option<(String, String)>> {
    match (id, secret) {
        (None, None) => Ok(None),
        (Some(id), Some(secret)) => Ok(Some((id.clone(), secret.clone()))),
        _ => Err(anyhow!("api key id and secret must be provided together")),
    }
}

fn configured_client(
    api_key_id: Option<String>,
    api_key_secret: Option<String>,
    allow_origin: Option<Url>,
) -> anyhow::Result<Client> {
    let mut builder = ClientBuilder::new();
    if let Some((id, secret)) = api_key_from_parts(&api_key_id, &api_key_secret)? {
        builder = builder.api_key(ApiKey::new(id, secret));
    }
    if let Some(origin) = allow_origin {
        builder =
            builder.destination_policy(cdx_tea_core::DestinationPolicy::allow_origin(&origin)?);
    }
    Ok(builder.build()?)
}
