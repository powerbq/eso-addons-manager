const RELEASE_TAG: &str = match option_env!("RELEASE_TAG") {
    Some(tag) => tag,
    None => "current",
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const RELEASES: &str = "https://github.com/powerbq/eso-addons-manager/releases";

pub fn version_url() -> String {
    format!("{RELEASES}/download/{RELEASE_TAG}/version.txt")
}

pub fn download_page() -> String {
    format!("{RELEASES}/tag/{RELEASE_TAG}")
}

pub fn available(latest: &str) -> Option<String> {
    let latest = latest.trim();
    (!latest.is_empty() && latest != VERSION).then(|| latest.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_a_differing_non_empty_version() {
        assert_eq!(available("9.9.9"), Some("9.9.9".to_owned()));
        assert_eq!(available("  9.9.9\n"), Some("9.9.9".to_owned()));
    }

    #[test]
    fn ignores_the_same_or_empty_version() {
        assert_eq!(available(VERSION), None);
        assert_eq!(available(""), None);
        assert_eq!(available("   \n"), None);
    }
}
