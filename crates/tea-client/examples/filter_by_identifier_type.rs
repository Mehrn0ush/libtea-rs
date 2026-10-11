//! Ask a TEA server for components that carry a compliance-document identifier.
//!
//! The short fragments in the repository README are illustrations. This file
//! is a program and is type-checked with the workspace.
//!
//! Production uses the default destination policy. That policy allows HTTPS
//! to public addresses and refuses cleartext, loopback, and metadata
//! addresses. No credential is set.
//!
//! ```text
//! cargo run -p tea-client --example filter_by_identifier_type -- https://api.example.com
//! ```
//!
//! A local HTTP server needs an exception for that one origin. Other origins
//! stay on the default policy.
//!
//! ```text
//! cargo run -p tea-client --example filter_by_identifier_type -- \
//!   --allow-origin http://127.0.0.1:8080 \
//!   http://127.0.0.1:8080
//! ```
//!
//! Compiling the example does not open a connection. Running it does.

use std::env;
use std::error::Error;
use std::fmt;
use std::process::ExitCode;

use tea_client::{Client, ClientBuilder};
use tea_core::enums::{ComponentSort, IdentifierType};
use tea_core::{DestinationPolicy, ListParams};
use url::Url;

#[derive(Debug)]
struct Usage(String);

impl fmt::Display for Usage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for Usage {}

fn usage(message: impl Into<String>) -> Usage {
    Usage(message.into())
}

fn compliance_document_query() -> ListParams<ComponentSort> {
    ListParams {
        page_size: Some(25),
        page_token: None,
        sort_field: None,
        sort_order: None,
        id_type: Some(IdentifierType::ComplianceDocument),
        id_value: None,
    }
}

fn parse_args(args: &[String]) -> Result<(String, Option<Url>), Usage> {
    let mut base = None;
    let mut allow_origin = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--allow-origin" {
            let value = rest
                .next()
                .ok_or_else(|| usage("--allow-origin requires a URL"))?;
            let origin = Url::parse(value).map_err(|error| usage(error.to_string()))?;
            allow_origin = Some(origin);
        } else if arg.starts_with('-') {
            return Err(usage(format!("unknown argument {arg}")));
        } else if base.is_some() {
            return Err(usage("pass one API base URL"));
        } else {
            base = Some(arg.clone());
        }
    }
    let base = base
        .ok_or_else(|| usage("usage: filter_by_identifier_type [--allow-origin URL] BASE_URL"))?;
    Ok((base, allow_origin))
}

fn client_for(allow_origin: Option<&Url>) -> Result<Client, Box<dyn Error>> {
    let mut builder = ClientBuilder::new();
    if let Some(origin) = allow_origin {
        builder = builder.destination_policy(DestinationPolicy::allow_origin(origin)?);
    }
    Ok(builder.build()?)
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match filter(&args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("filter_by_identifier_type: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn filter(args: &[String]) -> Result<(), Box<dyn Error>> {
    let (base, allow_origin) = parse_args(args)?;
    let client = client_for(allow_origin.as_ref())?;
    let service = client.service(&base, "1.0.0")?;
    let query = compliance_document_query();
    let page = service.query_components(&query).await?;
    println!("count {}", page.results.len());
    for component in &page.results {
        println!("{}\t{}", component.uuid, component.name);
    }
    match page.next_page_token.as_deref() {
        Some(token) if page.has_next => {
            println!("has_next true");
            println!("next_page_token {token}");
        }
        _ => println!("has_next false"),
    }
    Ok(())
}
