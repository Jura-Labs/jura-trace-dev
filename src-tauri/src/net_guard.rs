// SPDX-License-Identifier: AGPL-3.0-or-later

//! Keeps outbound fetches of caller-supplied URLs off this machine and off
//! the local network (SSRF).
//!
//! [`is_private_or_loopback_host`] reads a host as written: `localhost` and
//! literal addresses. [`check_host`] does that and then looks a hostname up,
//! so a public-looking name that points at `127.0.0.1` or `10.0.0.5` is
//! refused too. The caller pins its HTTP client to the addresses that
//! passed, so the address checked is the address connected to.
//!
//! Until v1.2.0 only the first check existed, it compared strings, and it
//! knew one IPv6 address (`::1`, which never matched, because a URL's host
//! is written `[::1]`).
//!
//! What this does not cover, on purpose:
//! - A name that does not resolve here is let through, because behind an
//!   HTTP proxy only the proxy can resolve public names, and refusing would
//!   break URL verification on those networks. The proxy then decides what
//!   is reachable.
//! - On a redirect the new host is checked but the client is not re-pinned,
//!   so a name that changes its answer between the check and the connection
//!   is not caught on a redirect hop. It is caught on the first request.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};

/// True for an address a caller-supplied URL must not reach: this machine,
/// a private or link-local network, or a range with no public meaning.
pub(crate) fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => is_blocked_v6(v6),
    }
}

fn is_blocked_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        // 0.0.0.0/8, "this network".
        || o[0] == 0
        // 100.64.0.0/10, carrier-grade NAT (RFC 6598).
        || (o[0] == 100 && (64..=127).contains(&o[1]))
}

fn is_blocked_v6(ip: Ipv6Addr) -> bool {
    // An IPv4 address written as IPv6 (`::ffff:127.0.0.1`, or the older
    // `::127.0.0.1`) is judged as the IPv4 address it is.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_v4(v4);
    }
    let s = ip.segments();
    if s[..6] == [0, 0, 0, 0, 0, 0] && (s[6] != 0 || s[7] > 1) {
        return is_blocked_v4(Ipv4Addr::new(
            (s[6] >> 8) as u8,
            s[6] as u8,
            (s[7] >> 8) as u8,
            s[7] as u8,
        ));
    }
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // fc00::/7, unique local.
        || (s[0] & 0xfe00) == 0xfc00
        // fe80::/10, link-local, and fec0::/10, the retired site-local range.
        || (s[0] & 0xffc0) == 0xfe80
        || (s[0] & 0xffc0) == 0xfec0
}

/// True when `host`, as written in a URL, names this machine or a private
/// or link-local address. Accepts the forms `url::Url::host_str` returns,
/// including a bracketed IPv6 literal.
///
/// A hostname that is not an address returns `false` here whatever it
/// resolves to; [`PublicOnlyResolver`] covers that.
pub(crate) fn is_private_or_loopback_host(host: &str) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    if h == "localhost" || h.ends_with(".localhost") {
        return true;
    }
    let literal = h
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(&h);
    literal.parse::<IpAddr>().is_ok_and(is_blocked_ip)
}

/// Drop the addresses a fetch must not reach. An error if none are left.
fn public_only(host: &str, addrs: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, std::io::Error> {
    let public: Vec<SocketAddr> = addrs
        .into_iter()
        .filter(|a| !is_blocked_ip(a.ip()))
        .collect();
    if public.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("{host} resolves only to local or private addresses"),
        ));
    }
    Ok(public)
}

/// What looking a host up found.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HostCheck {
    /// This machine, or a private or link-local address. Do not fetch.
    Blocked,
    /// Public. For a hostname, the addresses to pin the client to; empty
    /// for a literal address, which needs no pin.
    Public(Vec<SocketAddr>),
    /// A hostname that did not resolve on this machine. See the module
    /// note on proxies for why this is not refused.
    Unresolved,
}

/// Check a URL's host before fetching from it: as written, then by lookup.
pub(crate) fn check_host(host: &str) -> HostCheck {
    check_host_with(host, |h| Ok((h, 0).to_socket_addrs()?.collect()))
}

fn check_host_with(
    host: &str,
    lookup: impl Fn(&str) -> std::io::Result<Vec<SocketAddr>>,
) -> HostCheck {
    if is_private_or_loopback_host(host) {
        return HostCheck::Blocked;
    }
    let bare = host
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(host);
    if bare.parse::<IpAddr>().is_ok() {
        return HostCheck::Public(Vec::new());
    }
    match lookup(host) {
        Err(_) => HostCheck::Unresolved,
        Ok(addrs) if addrs.is_empty() => HostCheck::Unresolved,
        Ok(addrs) => match public_only(host, addrs) {
            Ok(public) => HostCheck::Public(public),
            Err(e) => {
                log::warn!("URL fetch refused: {e}");
                HostCheck::Blocked
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_for_this_machine() {
        for host in ["localhost", "LOCALHOST", "localhost.", "app.localhost"] {
            assert!(is_private_or_loopback_host(host), "{host}");
        }
        assert!(!is_private_or_loopback_host("localhost.example.org"));
        assert!(!is_private_or_loopback_host("notlocalhost"));
    }

    #[test]
    fn ipv4_literals() {
        for host in [
            "127.0.0.1",
            "127.255.255.254",
            "0.0.0.0",
            "0.1.2.3",
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.1",
            "192.168.255.255",
            "169.254.0.1",
            "169.254.169.254",
            "100.64.0.1",
            "100.127.255.255",
            "224.0.0.1",
            "255.255.255.255",
        ] {
            assert!(is_private_or_loopback_host(host), "{host}");
        }
        for host in [
            "172.15.255.255",
            "172.32.0.1",
            "100.63.255.255",
            "100.128.0.1",
            "8.8.8.8",
            "93.184.216.34",
        ] {
            assert!(!is_private_or_loopback_host(host), "{host}");
        }
    }

    /// The string comparison this replaced treated any host starting `10.`
    /// as private, and so refused public names such as `10.example.org`.
    #[test]
    fn a_name_that_starts_like_an_address_is_a_name() {
        assert!(!is_private_or_loopback_host("10.example.org"));
        assert!(!is_private_or_loopback_host("192.168.example.org"));
    }

    #[test]
    fn ipv6_literals_bracketed_as_a_url_writes_them() {
        for host in [
            "[::1]",
            "::1",
            "[::]",
            "[::ffff:127.0.0.1]",
            "[::ffff:10.0.0.1]",
            "[::127.0.0.1]",
            "[fc00::1]",
            "[fd12:3456:789a::1]",
            "[fe80::1]",
            "[febf::1]",
            "[fec0::1]",
            "[ff02::1]",
        ] {
            assert!(is_private_or_loopback_host(host), "{host}");
        }
        for host in ["[2606:4700:4700::1111]", "[::ffff:8.8.8.8]", "[fe00::1]"] {
            assert!(!is_private_or_loopback_host(host), "{host}");
        }
    }

    /// What `url` hands the check for the ways of writing loopback that are
    /// not dotted decimal. If `url` stopped normalising these, this fails.
    #[test]
    fn url_normalises_the_unusual_spellings_first() {
        for raw in [
            "http://2130706433/",
            "http://0x7f.0.0.1/",
            "http://0177.0.0.1/",
            "http://127.1/",
            "http://[::1]:8300/api/v1/stats",
            "http://[0:0:0:0:0:ffff:7f00:1]/",
        ] {
            let parsed = url::Url::parse(raw).expect(raw);
            let host = parsed.host_str().expect(raw);
            assert!(is_private_or_loopback_host(host), "{raw} gave host {host}");
        }
    }

    fn addr(s: &str) -> SocketAddr {
        SocketAddr::new(s.parse().unwrap(), 0)
    }

    #[test]
    fn resolution_keeps_public_addresses_and_drops_the_rest() {
        let kept = public_only(
            "mixed.example",
            vec![addr("10.0.0.5"), addr("93.184.216.34"), addr("::1")],
        )
        .unwrap();
        assert_eq!(kept, vec![addr("93.184.216.34")]);
    }

    #[test]
    fn a_name_resolving_only_to_private_addresses_is_refused() {
        let err =
            public_only("intranet.example", vec![addr("127.0.0.1"), addr("fd00::1")]).unwrap_err();
        assert!(err.to_string().contains("intranet.example"), "{err}");
        assert!(public_only("empty.example", vec![]).is_err());
    }

    #[test]
    fn a_public_name_pointing_at_a_private_address_is_blocked() {
        let to = |ips: &'static [&'static str]| {
            move |_: &str| Ok(ips.iter().map(|ip| addr(ip)).collect::<Vec<_>>())
        };
        assert_eq!(
            check_host_with("rebind.example.org", to(&["127.0.0.1"])),
            HostCheck::Blocked
        );
        assert_eq!(
            check_host_with("metadata.example.org", to(&["169.254.169.254", "fd00::1"])),
            HostCheck::Blocked
        );
        assert_eq!(
            check_host_with("cdn.example.org", to(&["93.184.216.34", "10.0.0.5"])),
            HostCheck::Public(vec![addr("93.184.216.34")])
        );
    }

    #[test]
    fn literals_and_local_names_are_decided_without_a_lookup() {
        let never = |h: &str| -> std::io::Result<Vec<SocketAddr>> { panic!("looked up {h}") };
        assert_eq!(check_host_with("localhost", never), HostCheck::Blocked);
        assert_eq!(check_host_with("[::1]", never), HostCheck::Blocked);
        assert_eq!(check_host_with("192.168.1.10", never), HostCheck::Blocked);
        assert_eq!(check_host_with("8.8.8.8", never), HostCheck::Public(vec![]));
        assert_eq!(
            check_host_with("[2606:4700:4700::1111]", never),
            HostCheck::Public(vec![])
        );
    }

    #[test]
    fn a_name_that_does_not_resolve_here_is_left_to_the_client() {
        let fails = |_: &str| Err(std::io::Error::other("no such host"));
        assert_eq!(
            check_host_with("only-the-proxy-knows.example", fails),
            HostCheck::Unresolved
        );
        let empty = |_: &str| Ok(Vec::new());
        assert_eq!(
            check_host_with("empty.example", empty),
            HostCheck::Unresolved
        );
    }

    /// The real lookup, on the one name every machine resolves without a
    /// network. It is blocked by name before any lookup; this checks the
    /// wiring of `check_host` itself.
    #[test]
    fn check_host_uses_the_system_lookup() {
        assert_eq!(check_host("localhost"), HostCheck::Blocked);
        assert_eq!(check_host("127.0.0.1"), HostCheck::Blocked);
    }
}
