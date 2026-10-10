#![no_main]

use libfuzzer_sys::fuzz_target;
use tea_core::{classify_ip, DestinationPolicy};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    if let Ok(ip) = text.parse::<std::net::IpAddr>() {
        let _ = classify_ip(ip);
    }
    if let Ok(url) = tea_core::parse_api_base(&text) {
        let _ = DestinationPolicy::public_https().check_url(&url);
        if let Ok(policy) = DestinationPolicy::allow_origin(&url) {
            let _ = policy.check_url(&url);
            let _ = policy.allows_resolved_ip(
                &url,
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            );
        }
    }
});
