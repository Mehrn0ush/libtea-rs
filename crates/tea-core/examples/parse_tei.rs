//! Parse a TEI without rewriting it.
//!
//! Run with `cargo run -p libtea-core --example parse_tei`.

use libtea_core::Tei;

fn main() {
    let tei = Tei::parse("tei://example.com/uuid/d4d9f54a-abcf-11ee-ac79-1a52914d44b1")
        .expect("registry example");
    tei.validate_registry().expect("uuid registry");
    println!("{tei}");
    println!("domain {}", tei.domain());
    println!("well-known {}", tei.well_known_url());
}
