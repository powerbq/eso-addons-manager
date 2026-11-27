use semver::Version;
use serde::Deserialize;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const RELEASES: &str = "https://github.com/powerbq/eso-addons-manager/releases";
// The full release list, newest first. `per_page=100` keeps every skipped version in one response so the
// dialog can show the whole changelog since the running build without paging.
const RELEASES_API: &str =
    "https://api.github.com/repos/powerbq/eso-addons-manager/releases?per_page=100";

#[derive(Debug, Clone)]
pub struct Available {
    pub version: String,
    pub page: String,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub draft: bool,
    // The git-cliff-rendered release notes (Markdown). GitHub sends `null` when a release has no
    // body, so it is optional and absent for drafts we never show.
    #[serde(default)]
    pub body: Option<String>,
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
fn is_newer(remote: &str) -> bool {
    match (Version::parse(remote.trim()), Version::parse(VERSION)) {
        (Ok(remote), Ok(current)) => remote > current,
        _ => false,
    }
}

// Every non-draft release strictly newer than the running build, newest first — so the dialog shows the
// notes for all versions the user skipped, not just the latest. Beta keeps pre-releases; stable drops
// them (an empty SemVer pre-release). Returns whole releases (tag kept with its leading `v`, plus body).
pub fn newer_than_current(releases: &[Release], beta: bool) -> Vec<&Release> {
    let mut newer: Vec<(Version, &Release)> = releases
        .iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.strip_prefix('v')?).ok()?;
            Some((version, release))
        })
        .filter(|(version, _)| beta || version.pre.is_empty())
        .filter(|(_, release)| {
            is_newer(
                release
                    .tag_name
                    .strip_prefix('v')
                    .unwrap_or(&release.tag_name),
            )
        })
        .collect();
    newer.sort_by(|a, b| b.0.cmp(&a.0));
    newer.into_iter().map(|(_, release)| release).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool) -> Release {
        Release {
            tag_name: tag.to_owned(),
            draft,
            body: None,
        }
    }

    // Tags built relative to the running build's version, so these tests hold whatever `CARGO_PKG_VERSION`
    // is at compile time.
    fn current() -> Version {
        Version::parse(VERSION).unwrap()
    }

    fn bumped(minor: u64) -> String {
        let current = current();
        format!("v{}.{}.0", current.major, current.minor + minor)
    }

    fn tags(releases: &[&Release]) -> Vec<String> {
        releases.iter().map(|r| r.tag_name.clone()).collect()
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
        let final_version = Version::parse(&bumped(1)[1..]).unwrap();
        let rc = Version::parse(&format!("{final_version}-rc.1")).unwrap();
        assert!(rc < final_version);
    }

    #[test]
    fn newer_than_current_collects_every_newer_release_newest_first() {
        let one = bumped(1);
        let two = bumped(2);
        let releases = [
            release(&one, false),
            release(&format!("v{VERSION}"), false), // the running build: not newer
            release(&two, false),
        ];
        assert_eq!(
            tags(&newer_than_current(&releases, false)),
            vec![two.clone(), one.clone()]
        );
    }

    #[test]
    fn newer_than_current_keeps_pre_releases_only_for_beta() {
        let rc = format!("{}-rc.1", bumped(1));
        let releases = [release(&rc, false)];
        assert_eq!(tags(&newer_than_current(&releases, true)), vec![rc]);
        assert!(newer_than_current(&releases, false).is_empty());
    }

    #[test]
    fn newer_than_current_skips_drafts_and_unparsable_and_older() {
        let newer = bumped(1);
        let releases = [
            release(&newer, true),     // draft: ignored even though newer
            release("nightly", false), // unparsable
            release("v0.0.0", false),  // older than any real build
        ];
        assert!(newer_than_current(&releases, true).is_empty());
    }
}
