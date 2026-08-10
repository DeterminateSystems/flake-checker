//! Recognition of Determinate Secure Packages flakes as legitimate Nixpkgs inputs.
//!
//! These flakes are available only from FlakeHub, published under the `DeterminateSystems` org
//! with names beginning with `secure-packages` (`secure-packages-rolling`,
//! `secure-packages-26.05`, `secure-packages-rolling-fips`, `secure-packages-26.05-fips`, and so
//! on).

const DETERMINATE_ORG: &str = "determinatesystems";
const SECURE_PACKAGES_PREFIX: &str = "secure-packages";
const FLAKEHUB_HOSTS: &[&str] = &["flakehub.com", "api.flakehub.com"];

/// Whether a tarball URL points to a Determinate Secure Packages flake on FlakeHub. Handles both
/// the user-supplied form (`https://flakehub.com/f/{org}/{name}/{version}`) and the locked form
/// (`https://api.flakehub.com/f/pinned/{org}/{name}/{version}/{uuid}/source.tar.gz`).
pub(crate) fn is_secure_packages_url(url: &str) -> bool {
    let Some((_scheme, rest)) = url.split_once("://") else {
        return false;
    };

    let mut segments = rest.split('/');

    let Some(host) = segments.next() else {
        return false;
    };

    if !FLAKEHUB_HOSTS.contains(&host.to_lowercase().as_str()) {
        return false;
    }

    if segments.next() != Some("f") {
        return false;
    }

    // Locked FlakeHub URLs insert a `pinned` segment before the org
    let owner = match segments.next() {
        Some("pinned") => segments.next(),
        owner => owner,
    };

    let (Some(owner), Some(name)) = (owner, segments.next()) else {
        return false;
    };

    is_secure_packages_flake(owner, name)
}

fn is_secure_packages_flake(owner: &str, name: &str) -> bool {
    owner.to_lowercase() == DETERMINATE_ORG && is_secure_packages_name(&name.to_lowercase())
}

fn is_secure_packages_name(name: &str) -> bool {
    name == SECURE_PACKAGES_PREFIX || name.starts_with(&format!("{SECURE_PACKAGES_PREFIX}-"))
}

#[cfg(test)]
mod test {
    use super::{is_secure_packages_flake, is_secure_packages_url};

    #[test]
    fn secure_packages_flakes() {
        let cases: Vec<(&str, &str, bool)> = vec![
            ("DeterminateSystems", "secure-packages-rolling", true),
            ("DeterminateSystems", "secure-packages-rolling-fips", true),
            ("DeterminateSystems", "secure-packages-25.11", true),
            ("DeterminateSystems", "secure-packages-26.05", true),
            ("DeterminateSystems", "secure-packages-26.05-fips", true),
            // Channels that don't exist yet still match the prefix
            ("DeterminateSystems", "secure-packages-26.11", true),
            ("DeterminateSystems", "secure-packages", true),
            ("determinatesystems", "secure-packages-rolling", true),
            // Right name, wrong org
            ("NotDeterminateSystems", "secure-packages-rolling", false),
            ("someone-else", "secure-packages-rolling", false),
            // Right org, wrong name
            ("DeterminateSystems", "nixpkgs", false),
            ("DeterminateSystems", "secure-packagesrolling", false),
            ("DeterminateSystems", "not-secure-packages-rolling", false),
        ];

        for (owner, name, expected) in cases {
            assert_eq!(
                is_secure_packages_flake(owner, name),
                expected,
                "unexpected result for {owner}/{name}"
            );
        }
    }

    #[test]
    fn secure_packages_urls() {
        let cases: Vec<(&str, bool)> = vec![
            (
                "https://flakehub.com/f/DeterminateSystems/secure-packages-rolling/*",
                true,
            ),
            (
                "https://flakehub.com/f/DeterminateSystems/secure-packages-26.05/0.1.tar.gz",
                true,
            ),
            (
                "https://api.flakehub.com/f/pinned/DeterminateSystems/secure-packages-rolling/0.1.1%2Brev-6fb441f657e98f31024f19ca68b691c5b0c3fdd6/01978415-c996-7747-a25c-77ca3a4c5ed9/source.tar.gz",
                true,
            ),
            // Right flake, wrong host
            (
                "https://example.com/f/DeterminateSystems/secure-packages-rolling/*",
                false,
            ),
            // Right host, wrong flake
            ("https://flakehub.com/f/NixOS/nixpkgs/0.1", false),
            (
                "https://flakehub.com/f/DeterminateSystems/easy-template/0",
                false,
            ),
            // Not a FlakeHub URL at all
            ("https://some-server.com/flake.tar.gz", false),
            ("flakehub.com/f/DeterminateSystems/secure-packages", false),
            ("https://flakehub.com/f/DeterminateSystems", false),
        ];

        for (url, expected) in cases {
            assert_eq!(
                is_secure_packages_url(url),
                expected,
                "unexpected result for {url}"
            );
        }
    }
}
