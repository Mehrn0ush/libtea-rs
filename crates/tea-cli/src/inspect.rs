//! `libtea inspect`: resolve a TEI and summarize its product, release,
//! collection and artifacts without downloading anything.
//!
//! Read-only: no artifact bytes are fetched and no checksum is checked. To
//! fetch and verify the artifacts of the latest collection, use `sync`.

use libtea_core::{Collection, Product, ProductReleaseWithCollection, Tei};
use url::Url;

use super::{clean, configured_builder, redact_url, with_cache, CacheArgs, CredentialArgs};

pub(crate) struct InspectOptions {
    pub tei: String,
    pub product_release: Option<String>,
    pub collection_version: Option<u64>,
    pub allow_discovered_credentials: bool,
    pub well_known_url: Option<Url>,
    pub allow_origin: Option<Url>,
    pub credentials: CredentialArgs,
    pub cache: CacheArgs,
    pub json: bool,
}

pub(crate) async fn inspect(options: InspectOptions) -> anyhow::Result<()> {
    let tei = Tei::parse(&options.tei)?;
    let mut builder = with_cache(
        configured_builder(&options.credentials, options.allow_origin.clone())?,
        &options.cache,
    )?
    .allow_discovered_credentials(options.allow_discovered_credentials);
    if let Some(url) = options.well_known_url.clone() {
        builder = builder.well_known_fetch_url(url);
    }
    let client = builder.build()?;
    let releases = client.discover(&tei).await?;
    let resolved = super::sync::select_release(&releases, options.product_release.as_deref())?;
    let service = client.service(&resolved.root_url, &resolved.api_version)?;
    let release = service
        .get_product_release(resolved.product_release)
        .await?;
    let product = service.get_product(release.release.product).await?;
    let collection = match options.collection_version {
        Some(version) => {
            service
                .get_product_release_collection(resolved.product_release, version)
                .await?
        }
        None => release.latest_collection.clone(),
    };
    let root_url = redact_url(&resolved.root_url);
    if options.json {
        let value = summary_json(
            &options.tei,
            &root_url,
            &resolved.api_version,
            &product,
            &release,
            &collection,
        );
        out!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        print_text(
            &root_url,
            &resolved.api_version,
            &product,
            &release,
            &collection,
        );
    }
    Ok(())
}

fn summary_json(
    tei: &str,
    root_url: &str,
    api_version: &str,
    product: &Product,
    release: &ProductReleaseWithCollection,
    collection: &Collection,
) -> serde_json::Value {
    let identifiers: Vec<_> = product
        .identifiers
        .iter()
        .map(|id| serde_json::json!({"type": id.id_type.as_str(), "value": id.id_value}))
        .collect();
    let components: Vec<_> = release
        .release
        .components
        .iter()
        .map(|component| {
            serde_json::json!({
                "uuid": component.uuid.to_string(),
                "release": component.release.map(|id| id.to_string()),
            })
        })
        .collect();
    let artifacts: Vec<_> = collection
        .artifacts
        .iter()
        .map(|artifact| {
            let formats: Vec<_> = artifact
                .formats
                .iter()
                .map(|format| {
                    let algorithms: Vec<_> = format
                        .checksums
                        .iter()
                        .flatten()
                        .map(|checksum| checksum.algorithm().as_str().to_owned())
                        .collect();
                    serde_json::json!({
                        "media_type": format.media_type,
                        "checksum_algorithms": algorithms,
                        "has_url": format.url.is_some(),
                        "has_signature": format.signature_url.is_some(),
                    })
                })
                .collect();
            serde_json::json!({
                "uuid": artifact.uuid.to_string(),
                "version": artifact.version,
                "name": artifact.name,
                "type": artifact.artifact_type.as_str(),
                "formats": formats,
            })
        })
        .collect();
    serde_json::json!({
        "schema_version": 1,
        "tei": tei,
        "server": {"root_url": root_url, "api_version": api_version},
        "product": {
            "uuid": product.uuid.to_string(),
            "name": product.name,
            "identifiers": identifiers,
        },
        "product_release": {
            "uuid": release.release.uuid.to_string(),
            "version": release.release.version,
            "created_date": release.release.created_date.as_str(),
            "release_date": release.release.release_date.as_ref().map(|t| t.as_str()),
            "components": components,
        },
        "collection": {
            "uuid": collection.uuid.to_string(),
            "version": collection.version,
            "created_date": collection.created_date.as_str(),
            "update_reason": collection.update_reason.reason_type.as_str(),
            "artifacts": artifacts,
        },
    })
}

fn print_text(
    root_url: &str,
    api_version: &str,
    product: &Product,
    release: &ProductReleaseWithCollection,
    collection: &Collection,
) {
    out!("server\t{root_url}\t{api_version}");
    out!("product\t{}\t{}", product.uuid, product.name);
    for id in &product.identifiers {
        out!("identifier\t{}\t{}", id.id_type.as_str(), id.id_value);
    }
    let release_ = &release.release;
    out!(
        "product_release\t{}\t{}\tcreated {}\treleased {}",
        release_.uuid,
        release_.version,
        release_.created_date.as_str(),
        release_.release_date.as_ref().map_or("-", |t| t.as_str())
    );
    for component in &release_.components {
        out!(
            "component\t{}\t{}",
            component.uuid,
            component
                .release
                .map_or_else(|| "-".to_owned(), |id| id.to_string())
        );
    }
    out!(
        "collection\t{}\tversion {}\tcreated {}\treason {}\tartifacts {}",
        collection.uuid,
        collection.version,
        collection.created_date.as_str(),
        collection.update_reason.reason_type.as_str(),
        collection.artifacts.len()
    );
    for artifact in &collection.artifacts {
        out!(
            "artifact\t{}\t{}\tversion {}\t{}",
            artifact.uuid,
            artifact.artifact_type.as_str(),
            artifact.version,
            artifact.name.as_deref().unwrap_or("-")
        );
        for format in &artifact.formats {
            let algorithms: Vec<_> = format
                .checksums
                .iter()
                .flatten()
                .map(|checksum| checksum.algorithm().as_str().to_owned())
                .collect();
            out!(
                "format\t{}\tchecksums {}\tsignature {}",
                format.media_type,
                if algorithms.is_empty() {
                    "-".to_owned()
                } else {
                    algorithms.join(",")
                },
                if format.signature_url.is_some() {
                    "published"
                } else {
                    "-"
                }
            );
        }
    }
}
