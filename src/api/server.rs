//! Signing in to a server: what it takes, and which addresses are safe to send a
//! password to.

pub use crate::auth::{Credentials, DEFAULT_SERVICE};

/// Why a server address typed by hand shouldn't be used, if it shouldn't: plain
/// http would send passwords and keys unencrypted, except to this computer.
pub fn insecure_server(address: &str) -> Option<&'static str> {
    let u = url::Url::parse(address.trim()).ok()?;
    let local = match u.host() {
        Some(url::Host::Domain(d)) => d.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    (u.scheme() == "http" && !local)
        .then_some("Use an https:// address. Plain http would send your password unencrypted.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_servers_except_locally() {
        assert!(insecure_server("http://example.com").is_some());
        assert!(insecure_server("https://example.com").is_none());
        assert!(insecure_server("http://localhost:5000").is_none());
        assert!(insecure_server("http://127.0.0.1:5000").is_none());
        assert!(insecure_server("http://[::1]:5000").is_none());
    }
}
