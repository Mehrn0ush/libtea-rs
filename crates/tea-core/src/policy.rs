//! Destination checks that do not perform DNS lookups.
//!
//! The HTTP client applies the same classification to addresses returned by
//! DNS. Textual hostname checks are not the only control.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::Url;

use crate::error::PolicyError;

/// Classification of an IP address for SSRF policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IpClass {
    /// A globally routable unicast address.
    Public,
    /// Loopback.
    Loopback,
    /// Private, unique-local, or shared address space.
    Private,
    /// Link-local, including cloud metadata addresses in `169.254.0.0/16`.
    LinkLocal,
    /// Documentation, benchmarking, or discard ranges.
    Documentation,
    /// Multicast or broadcast.
    Multicast,
    /// Unspecified or otherwise reserved.
    Reserved,
}

impl IpClass {
    /// Whether production discovery may connect to this address.
    #[must_use]
    pub fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

/// Classify an address, including IPv4 embedded in IPv6 transition ranges.
#[must_use]
pub fn classify_ip(ip: IpAddr) -> IpClass {
    match ip {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => classify_v6(v6),
    }
}

fn classify_v4(ip: Ipv4Addr) -> IpClass {
    let [a, b, _, _] = ip.octets();
    if ip.is_unspecified() || a >= 240 {
        return IpClass::Reserved;
    }
    if ip.is_loopback() {
        return IpClass::Loopback;
    }
    if ip.is_private() || (a == 100 && (64..128).contains(&b)) {
        return IpClass::Private;
    }
    if ip.is_link_local() {
        return IpClass::LinkLocal;
    }
    if ip.is_multicast() || ip.is_broadcast() {
        return IpClass::Multicast;
    }
    if a == 0 {
        return IpClass::Reserved;
    }
    // RFC 2544 benchmarking range.
    if a == 198 && (b == 18 || b == 19) {
        return IpClass::Documentation;
    }
    let third = ip.octets()[2];
    if a == 192 && b == 0 && (third == 0 || third == 2)
        || (a == 198 && b == 51 && third == 100)
        || (a == 203 && b == 0 && third == 113)
    {
        return IpClass::Documentation;
    }
    IpClass::Public
}

fn classify_v6(ip: Ipv6Addr) -> IpClass {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return classify_v4(v4);
    }
    if let Some(v4) = embedded_transition_v4(ip) {
        return classify_v4(v4);
    }
    if ip.is_unspecified() {
        return IpClass::Reserved;
    }
    if ip.is_loopback() {
        return IpClass::Loopback;
    }
    // Deprecated IPv4-compatible form `::a.b.c.d` (RFC 4291 section 2.5.5.1).
    // `::` and `::1` are handled above.
    if ip.segments()[..6].iter().all(|s| *s == 0) {
        let octets = ip.octets();
        return classify_v4(Ipv4Addr::new(
            octets[12], octets[13], octets[14], octets[15],
        ));
    }
    if is_unique_local(ip) || is_site_local(ip) || is_nat64_local_use(ip) {
        return IpClass::Private;
    }
    if is_link_local_v6(ip) {
        return IpClass::LinkLocal;
    }
    if ip.is_multicast() {
        return IpClass::Multicast;
    }
    if is_documentation_v6(ip) || is_teredo(ip) || is_discard_v6(ip) {
        return IpClass::Documentation;
    }
    IpClass::Public
}

fn embedded_transition_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let segments = ip.segments();
    // 6to4 2002::/16 embeds IPv4 in the next 32 bits.
    if segments[0] == 0x2002 {
        return Some(Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            segments[1] as u8,
            (segments[2] >> 8) as u8,
            segments[2] as u8,
        ));
    }
    // NAT64 well-known prefix 64:ff9b::/96.
    if segments[0] == 0x0064 && segments[1] == 0xff9b && segments[2..6].iter().all(|s| *s == 0) {
        let octets = ip.octets();
        return Some(Ipv4Addr::new(
            octets[12], octets[13], octets[14], octets[15],
        ));
    }
    None
}

fn is_unique_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

/// Deprecated site-local `fec0::/10` (RFC 3879). Not globally routable.
fn is_site_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfec0
}

/// NAT64 local-use prefix `64:ff9b:1::/48` (RFC 8215) maps to private IPv4 space.
fn is_nat64_local_use(ip: Ipv6Addr) -> bool {
    let [first, second, third, ..] = ip.segments();
    first == 0x0064 && second == 0xff9b && third == 0x0001
}

fn is_link_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn is_documentation_v6(ip: Ipv6Addr) -> bool {
    let [first, second, ..] = ip.segments();
    first == 0x2001 && (second == 0x0db8 || second == 0x0002)
}

fn is_teredo(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0
}

fn is_discard_v6(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x0100 && ip.segments()[1] == 0
}

/// Policy for URLs learned from TEIs, discovery documents, and redirects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationPolicy {
    /// Origins allowed to use HTTP and non-public addresses.
    ///
    /// An origin is `scheme://host:port` with the default port included.
    /// Production policy has an empty set.
    exceptions: BTreeSet<String>,
}

impl Default for DestinationPolicy {
    fn default() -> Self {
        Self::public_https()
    }
}

impl DestinationPolicy {
    /// HTTPS to public addresses only.
    #[must_use]
    pub fn public_https() -> Self {
        Self {
            exceptions: BTreeSet::new(),
        }
    }

    /// Allow one explicit origin to use HTTP and non-public addresses.
    ///
    /// Other origins keep the production restrictions. This is the development
    /// seam for a local TEA server; it is not a global "insecure" switch.
    ///
    /// # Errors
    ///
    /// Returns an error when `origin` has no host.
    pub fn allow_origin(origin: &Url) -> Result<Self, PolicyError> {
        let mut policy = Self::public_https();
        policy.exceptions.insert(origin_key(origin)?);
        Ok(policy)
    }

    /// Add another explicit exception origin.
    ///
    /// # Errors
    ///
    /// Returns an error when `origin` has no host.
    pub fn and_allow(mut self, origin: &Url) -> Result<Self, PolicyError> {
        self.exceptions.insert(origin_key(origin)?);
        Ok(self)
    }

    /// Whether this origin was explicitly excepted.
    ///
    /// An exception is one `scheme://host:port` key. `https://example.test`
    /// and `https://example.test:443` are the same origin. A different scheme
    /// or port is not excepted, and another hostname is not excepted merely
    /// because it resolves to the same address.
    #[must_use]
    pub fn allows_origin(&self, url: &Url) -> bool {
        origin_key(url)
            .map(|key| self.exceptions.contains(&key))
            .unwrap_or(false)
    }

    /// Check scheme, userinfo, and literal addresses.
    ///
    /// DNS results are checked separately with [`Self::allows_resolved_ip`].
    ///
    /// # Errors
    ///
    /// Returns an error when the URL must not be contacted.
    pub fn check_url(&self, url: &Url) -> Result<(), PolicyError> {
        if url.username() != "" || url.password().is_some() {
            return Err(PolicyError::Userinfo);
        }
        let host = url.host_str().ok_or(PolicyError::Host)?;
        match url.scheme() {
            "https" => {}
            "http" if self.allows_origin(url) => {}
            other => return Err(PolicyError::Scheme(other.to_owned())),
        }
        if !self.allows_origin(url) && blocked_hostname(host) {
            return Err(PolicyError::Hostname(host.to_owned()));
        }
        if let Some(ip) = url.host().and_then(|host| match host {
            url::Host::Ipv4(ip) => Some(IpAddr::V4(ip)),
            url::Host::Ipv6(ip) => Some(IpAddr::V6(ip)),
            url::Host::Domain(_) => None,
        }) {
            self.allows_resolved_ip(url, ip)?;
        }
        Ok(())
    }

    /// Accept a DNS answer for `url`.
    ///
    /// # Errors
    ///
    /// Returns an error when the address is not public and the origin is not excepted.
    pub fn allows_resolved_ip(&self, url: &Url, ip: IpAddr) -> Result<(), PolicyError> {
        let class = classify_ip(ip);
        if class.is_public() || self.allows_origin(url) {
            Ok(())
        } else {
            Err(PolicyError::Destination(format!("{ip} ({class:?})")))
        }
    }
}

/// `scheme://host:port`, including the scheme default port.
///
/// # Errors
///
/// Returns an error when the URL has no host or no known port.
pub fn origin_key(url: &Url) -> Result<String, PolicyError> {
    let host = url.host_str().ok_or(PolicyError::Host)?;
    let port = url
        .port_or_known_default()
        .ok_or(PolicyError::Scheme(url.scheme().into()))?;
    if host.contains(':') {
        Ok(format!("{}://[{host}]:{port}", url.scheme()))
    } else {
        Ok(format!("{}://{host}:{port}", url.scheme()))
    }
}

fn blocked_hostname(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host == "metadata.google.internal"
        || host == "metadata.google.com"
}

/// Validate an advertised API base URL.
///
/// # Errors
///
/// Returns an error when the URL is not an absolute HTTP(S) base without
/// userinfo, query, fragment, or a trailing slash.
pub fn parse_api_base(text: &str) -> Result<Url, PolicyError> {
    if text.chars().any(|c| c.is_whitespace()) {
        return Err(PolicyError::QueryOrFragment);
    }
    let url = Url::parse(text).map_err(|_| PolicyError::Host)?;
    if url.username() != "" || url.password().is_some() {
        return Err(PolicyError::Userinfo);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(PolicyError::QueryOrFragment);
    }
    if !matches!(url.scheme(), "https" | "http") {
        return Err(PolicyError::Scheme(url.scheme().to_owned()));
    }
    if url.host_str().is_none() {
        return Err(PolicyError::Host);
    }
    let path = url.path();
    if path.len() > 1 && path.ends_with('/') {
        return Err(PolicyError::TrailingSlash);
    }
    if text.ends_with('/') {
        return Err(PolicyError::TrailingSlash);
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_metadata_and_loopback_by_default() {
        let policy = DestinationPolicy::public_https();
        let metadata = Url::parse("https://169.254.169.254/latest").unwrap();
        assert!(policy.check_url(&metadata).is_err());
        let loopback = Url::parse("https://127.0.0.1/").unwrap();
        assert!(policy.check_url(&loopback).is_err());
        let http = Url::parse("http://example.com/.well-known/tea").unwrap();
        assert!(policy.check_url(&http).is_err());
        let ok = Url::parse("https://api.example.com/tea").unwrap();
        assert!(policy.check_url(&ok).is_ok());
    }

    #[test]
    fn exception_is_scoped_to_one_origin() {
        let allowed = Url::parse("http://127.0.0.1:8080/tea").unwrap();
        let policy = DestinationPolicy::allow_origin(&allowed).unwrap();
        assert!(policy.check_url(&allowed).is_ok());
        let other_port = Url::parse("http://127.0.0.1:9/").unwrap();
        assert!(policy.check_url(&other_port).is_err());
        let metadata = Url::parse("http://169.254.169.254/").unwrap();
        assert!(policy.check_url(&metadata).is_err());
    }

    #[test]
    fn classifies_embedded_private_addresses() {
        let mapped: IpAddr = "0:0:0:0:0:ffff:127.0.0.1".parse().unwrap();
        assert_eq!(classify_ip(mapped), IpClass::Loopback);
        let nat64: IpAddr = "64:ff9b::a00:1".parse().unwrap();
        assert_eq!(classify_ip(nat64), IpClass::Private);
        assert_eq!(classify_ip("0.1.2.3".parse().unwrap()), IpClass::Reserved);
        assert_eq!(
            classify_ip("198.18.0.1".parse().unwrap()),
            IpClass::Documentation
        );
    }

    #[test]
    fn deprecated_ipv6_forms_are_not_public() {
        // IPv4-compatible (::a.b.c.d, RFC 4291 deprecated) carries the IPv4 class.
        assert_eq!(
            classify_ip("::127.0.0.1".parse().unwrap()),
            IpClass::Loopback
        );
        assert_eq!(classify_ip("::10.0.0.1".parse().unwrap()), IpClass::Private);
        assert_eq!(
            classify_ip("::169.254.169.254".parse().unwrap()),
            IpClass::LinkLocal
        );
        // Site-local fec0::/10 (deprecated) and NAT64 local-use 64:ff9b:1::/48.
        assert_eq!(classify_ip("fec0::1".parse().unwrap()), IpClass::Private);
        assert_eq!(
            classify_ip("64:ff9b:1::1".parse().unwrap()),
            IpClass::Private
        );
        // Unchanged cases.
        assert_eq!(classify_ip("::1".parse().unwrap()), IpClass::Loopback);
        assert_eq!(classify_ip("::".parse().unwrap()), IpClass::Reserved);
        assert_eq!(classify_ip("fe80::1".parse().unwrap()), IpClass::LinkLocal);
        assert_eq!(
            classify_ip("2606:4700::1".parse().unwrap()),
            IpClass::Public
        );
        let policy = DestinationPolicy::public_https();
        for url in [
            "https://[::127.0.0.1]/",
            "https://[fec0::1]/",
            "https://[::ffff:127.0.0.1]/",
        ] {
            assert!(
                policy.check_url(&Url::parse(url).unwrap()).is_err(),
                "{url}"
            );
        }
    }

    #[test]
    fn exception_is_scheme_host_and_port() {
        let allowed = Url::parse("http://allowed.test:8080/tea").unwrap();
        let policy = DestinationPolicy::allow_origin(&allowed).unwrap();
        let private: IpAddr = "10.1.2.3".parse().unwrap();
        let v6: IpAddr = "fd00::1".parse().unwrap();
        assert!(policy.allows_resolved_ip(&allowed, private).is_ok());
        assert!(policy
            .allows_resolved_ip(
                &Url::parse("http://allowed.test:8080/other").unwrap(),
                private
            )
            .is_ok());
        assert!(policy
            .allows_resolved_ip(&Url::parse("https://allowed.test:8080/").unwrap(), private)
            .is_err());
        assert!(policy
            .allows_resolved_ip(&Url::parse("http://allowed.test:8443/").unwrap(), private)
            .is_err());
        assert!(policy
            .allows_resolved_ip(&Url::parse("http://other.test:8080/").unwrap(), private)
            .is_err());
        assert!(policy.allows_resolved_ip(&allowed, v6).is_ok());
        assert!(policy
            .allows_resolved_ip(&Url::parse("http://other.test:8080/").unwrap(), v6)
            .is_err());

        let https = Url::parse("https://example.test/tea").unwrap();
        let explicit = Url::parse("https://example.test:443/tea").unwrap();
        let policy = DestinationPolicy::allow_origin(&https).unwrap();
        assert!(policy.allows_origin(&explicit));
        assert_eq!(origin_key(&https).unwrap(), origin_key(&explicit).unwrap());
        let v6_origin = Url::parse("http://[::1]:8080/").unwrap();
        let v6_policy = DestinationPolicy::allow_origin(&v6_origin).unwrap();
        assert!(v6_policy.check_url(&v6_origin).is_ok());
        assert!(v6_policy
            .check_url(&Url::parse("http://[::1]:8081/").unwrap())
            .is_err());
    }
}
