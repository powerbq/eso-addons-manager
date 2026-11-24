use semver::Version;
use serde::Deserialize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const RELEASES: &str = "https://github.com/powerbq/eso-addons-manager/releases";
const RELEASES_API: &str = "https://api.github.com/repos/powerbq/eso-addons-manager/releases";

#[derive(Debug, Clone)]
pub struct Available {
    pub version: String,
    pub page: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub draft: bool,
}

pub fn stable_latest_api() -> String {
    format!("{RELEASES_API}/latest")
}

pub fn stable_page() -> String {
    format!("{RELEASES}/latest")
}

pub fn tag_page(tag: &str) -> String {
    format!("{RELEASES}/tag/{tag}")
}

pub fn releases_api() -> &'static str {
    RELEASES_API
}

// A published version string (no leading `v`) is an update only when it is strictly newer than the
// running build by SemVer precedence. `CARGO_PKG_VERSION` is the build's full version — under
// manifest-authoritative versioning it carries any `-rc.N`, so an rc build knows it is an rc and is
// correctly offered the final (`0.1.0 > 0.1.0-rc.2`).
pub fn is_newer(remote: &str) -> bool {
    match (Version::parse(remote.trim()), Version::parse(VERSION)) {
        (Ok(remote), Ok(current)) => remote > current,
        _ => false,
    }
}

// The newest release tag by SemVer among non-drafts (pre-releases included). Returning the max over all
// non-drafts — not just pre-releases — keeps the beta path non-empty: with no rc it falls to the stable
// latest. Tags keep their leading `v`.
pub fn newest(releases: &[Release]) -> Option<&str> {
    releases
        .iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.strip_prefix('v')?).ok()?;
            Some((version, release.tag_name.as_str()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, tag)| tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool) -> Release {
        Release {
            tag_name: tag.to_owned(),
            draft,
        }
    }

    #[test]
    fn newer_strictly_by_semver() {
        assert!(is_newer("999.0.0"));
        assert!(is_newer("  999.0.0\n"));
        assert!(!is_newer(VERSION));
        assert!(!is_newer("0.0.0"));
        assert!(!is_newer(""));
        assert!(!is_newer("not-a-version"));
    }

    #[test]
    fn a_release_outranks_its_pre_release() {
        let current = Version::parse(VERSION).unwrap();
        let final_version =
            Version::parse(&format!("{}.{}.{}", current.major, current.minor + 1, 0)).unwrap();
        let rc = Version::parse(&format!("{final_version}-rc.1")).unwrap();
        assert!(rc < final_version);
    }

    #[test]
    fn newest_picks_max_by_semver_and_skips_drafts() {
        let releases = [
            release("v0.1.0-rc.1", false),
            release("v0.1.0-rc.10", false),
            release("v0.1.0-rc.2", false),
            release("v9.9.9", true), // draft: ignored even though highest
        ];
        assert_eq!(newest(&releases), Some("v0.1.0-rc.10"));
    }

    #[test]
    fn newest_prefers_a_final_over_its_rc() {
        let releases = [release("v0.1.0-rc.3", false), release("v0.1.0", false)];
        assert_eq!(newest(&releases), Some("v0.1.0"));
    }

    #[test]
    fn newest_is_none_without_parsable_non_drafts() {
        assert_eq!(newest(&[]), None);
        assert_eq!(newest(&[release("nightly", false)]), None);
        assert_eq!(newest(&[release("v1.0.0", true)]), None);
    }
}
