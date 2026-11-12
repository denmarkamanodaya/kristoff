//! Which outside links the app may open in the default browser: only `https://` URLs whose host is
//! on the configured allowlist (official legal sources). The allowlist itself is configuration,
//! passed in by the caller.

/// The lower-cased host of an `https://` URL, or `None` when the URL is not plain https, carries
/// user info or a port, or has an empty host.
fn https_host(url: &str) -> Option<String> {
    let scheme_end = url
        .get(..8)
        .filter(|s| s.eq_ignore_ascii_case("https://"))?
        .len();
    let rest = &url[scheme_end..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..end];
    let plain = !host.is_empty() && !host.contains(['@', ':', '\\', ' ']);
    plain.then(|| host.to_ascii_lowercase())
}

/// Whether `url` is an https URL whose host is exactly one of `allowed` (no subdomain matching).
#[must_use]
pub fn is_allowed_external(url: &str, allowed: &[&str]) -> bool {
    let no_controls = !url.chars().any(char::is_control);
    no_controls && https_host(url).is_some_and(|host| allowed.contains(&host.as_str()))
}

#[cfg(test)]
mod tests {
    use super::is_allowed_external;

    const HOSTS: [&str; 2] = ["lawphil.net", "elibrary.judiciary.gov.ph"];

    #[test]
    fn allows_https_links_to_listed_hosts() {
        assert!(is_allowed_external(
            "https://lawphil.net/statutes/repacts/ra2004/ra_9262_2004.html",
            &HOSTS
        ));
        assert!(is_allowed_external("HTTPS://LawPhil.net", &HOSTS));
        assert!(is_allowed_external(
            "https://elibrary.judiciary.gov.ph/?q=1#x",
            &HOSTS
        ));
    }

    #[test]
    fn refuses_other_schemes_hosts_and_tricks() {
        let bad = [
            "http://lawphil.net/",
            "http://xlawphil.net/",
            "ftp://lawphil.net/",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://evil.net/",
            "https://lawphil.net.evil.net/",
            "https://sub.lawphil.net/",
            "https://lawphil.net@evil.net/",
            "https://evil.net\\@lawphil.net/",
            "https://lawphil.net:8443/",
            "https://lawphil.net/\nx",
            "https://",
            "lawphil.net",
        ];
        for url in bad {
            assert!(!is_allowed_external(url, &HOSTS), "{url}");
        }
    }
}
