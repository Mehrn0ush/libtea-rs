//! Build a compliance-document query and a pagination continuation.
//!
//! With no arguments this program does not open a connection. It constructs a
//! client, a `ListParams` value, and the next page's parameters from a
//! synthetic response. Pass an API base URL to run the same query:
//!
//! ```text
//! cargo run -p cdx-tea-client --example paginate_components
//! cargo run -p cdx-tea-client --example paginate_components -- https://api.example.com
//! ```
//!
//! Compilation does not establish that a live TEA server accepts the query.

use std::env;
use std::process::ExitCode;

use cdx_tea_client::{Client, ClientError};
use cdx_tea_core::enums::{ComponentSort, IdentifierType};
use cdx_tea_core::{Component, ComponentId, ListParams, Page};

fn compliance_query() -> ListParams<ComponentSort> {
    ListParams {
        id_type: Some(IdentifierType::ComplianceDocument),
        ..ListParams::default()
    }
}

/// Keep the caller's filters and page size, and replace only the page token.
fn continue_with(
    query: &ListParams<ComponentSort>,
    page: &Page<Component>,
) -> Option<ListParams<ComponentSort>> {
    if !page.has_next {
        return None;
    }
    let token = page.next_page_token.clone()?;
    let mut next = query.clone();
    next.page_token = Some(token);
    Some(next)
}

fn offline() -> Result<(), ClientError> {
    let _client = Client::builder().build()?;
    let query = compliance_query();
    query.validate()?;
    let page = Page {
        has_next: true,
        next_page_token: Some("opaque-token".to_owned()),
        results: vec![Component {
            uuid: ComponentId::parse("d4d9f54a-abcf-11ee-ac79-1a52914d44b1")?,
            name: "Example component".to_owned(),
            identifiers: Vec::new(),
        }],
    };
    let next = continue_with(&query, &page).expect("a token is present");
    assert_eq!(next.id_type, query.id_type);
    assert_eq!(next.id_value, query.id_value);
    assert_eq!(next.page_size, query.page_size);
    assert_eq!(next.sort_field, query.sort_field);
    assert_eq!(next.page_token.as_deref(), Some("opaque-token"));
    println!(
        "{} result(s); next request keeps idType and sends the page token",
        page.results.len()
    );
    Ok(())
}

async fn fetch(base: &str) -> Result<(), ClientError> {
    let client = Client::builder().build()?;
    let service = client.service(base, "1.0.0")?;
    let mut query = compliance_query();
    loop {
        let page = service.query_components(&query).await?;
        for component in &page.results {
            println!("{}\t{}", component.uuid, component.name);
        }
        match continue_with(&query, &page) {
            Some(next) => query = next,
            None => break,
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let result = match env::args().nth(1) {
        Some(base) => fetch(&base).await,
        None => offline(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("paginate_components: {error}");
            ExitCode::FAILURE
        }
    }
}
