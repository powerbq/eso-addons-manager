use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::de::DeserializeOwned;

use crate::api::{self, CategoryEntry, Client, FileDetails, FileListEntry};
use crate::bbcode;

pub const DIRECTORY: &str = "api";
pub const FILE_LIST: &str = "filelist.json";
pub const CATEGORY_LIST: &str = "categorylist.json";
pub const DETAILS: &str = "filedetails";

static INFO_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)esoui\.com/downloads/info(\d+)-").unwrap());
static FILEINFO_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)esoui\.com/\S*?fileinfo\.php\?\S*?\bid=(\d+)").unwrap());

pub async fn file_list(client: &Client, api: &Path) -> Result<Vec<FileListEntry>, api::Error> {
    fetch_cached(client, FILE_LIST, &api.join(FILE_LIST)).await
}

pub async fn categories(client: &Client, api: &Path) -> Result<Vec<CategoryEntry>, api::Error> {
    fetch_cached(client, CATEGORY_LIST, &api.join(CATEGORY_LIST)).await
}

pub async fn file_details(
    client: &Client,
    api: &Path,
    uid: &str,
) -> Result<FileDetails, api::Error> {
    let remote = format!("{DETAILS}/{uid}.json");
    let details: Vec<FileDetails> = fetch_cached(client, &remote, &details_path(api, uid)).await?;

    details
        .into_iter()
        .next()
        .ok_or_else(|| api::Error::NotFound(uid.to_owned()))
}

pub fn cached_details(api: &Path, uid: &str) -> Option<FileDetails> {
    let raw = fs::read_to_string(details_path(api, uid)).ok()?;
    let details: Vec<FileDetails> = serde_json::from_str(&raw).ok()?;

    details.into_iter().next()
}

pub fn cached_file_list(api: &Path) -> Option<Vec<FileListEntry>> {
    let raw = fs::read_to_string(api.join(FILE_LIST)).ok()?;

    serde_json::from_str(&raw).ok()
}

pub fn links(details: &FileDetails) -> Vec<u32> {
    let mut uids = Vec::new();
    for url in bbcode::urls(&details.description) {
        if let Some(uid) = esoui_uid(&url)
            && !uids.contains(&uid)
        {
            uids.push(uid);
        }
    }

    uids
}

pub fn is_cache_entry(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(FILE_LIST | CATEGORY_LIST | DETAILS)
    )
}

async fn fetch_cached<T: DeserializeOwned>(
    client: &Client,
    remote: &str,
    path: &Path,
) -> Result<T, api::Error> {
    match client.get_raw(remote).await {
        Ok(raw) => {
            let parsed = serde_json::from_str(&raw).map_err(api::Error::parse)?;
            write(path, raw.as_bytes());
            Ok(parsed)
        }
        Err(error) => match fs::read_to_string(path) {
            Ok(raw) => {
                tracing::warn!("using the cached {remote}: {error}");
                serde_json::from_str(&raw).map_err(api::Error::parse)
            }
            Err(_) => Err(error),
        },
    }
}

fn details_path(api: &Path, uid: &str) -> PathBuf {
    api.join(DETAILS).join(format!("{uid}.json"))
}

fn esoui_uid(url: &str) -> Option<u32> {
    INFO_LINK
        .captures(url)
        .or_else(|| FILEINFO_LINK.captures(url))
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse().ok())
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        tracing::warn!("cannot create {}: {error}", parent.display());
        return;
    }

    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);

    if let Err(error) = fs::write(&temporary, bytes).and_then(|()| fs::rename(&temporary, path)) {
        tracing::warn!("cannot cache {}: {error}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(description: &str) -> FileDetails {
        FileDetails {
            version: String::new(),
            md5: String::new(),
            download_url: String::new(),
            description: description.to_owned(),
            changelog: String::new(),
        }
    }

    #[test]
    fn extracts_esoui_uid_from_links() {
        assert_eq!(
            esoui_uid("https://www.esoui.com/downloads/info7-LibAddonMenu-2.0.html"),
            Some(7)
        );
        assert_eq!(
            esoui_uid("https://www.esoui.com/downloads/fileinfo.php?id=584"),
            Some(584)
        );
        assert_eq!(esoui_uid("https://example.com/downloads/info7-thing"), None);
        assert_eq!(esoui_uid("not a url"), None);
    }

    #[test]
    fn collects_dependency_links_from_description() {
        let description = "[LIST]\n\
            [*][URL=\"https://www.esoui.com/downloads/info2125-LibAsync.html\"]LibAsync[/URL]\n\
            [*][url=https://www.esoui.com/downloads/info584-LibHarvensAddonsSettings.html]LibHarvensAddonSettings[/url]\n\
            [/LIST]";

        assert_eq!(links(&details(description)), vec![2125, 584]);
    }

    #[test]
    fn ignores_non_link_text_and_deduplicates() {
        let description = "see info7-not-a-link in text \
            [url=https://www.esoui.com/downloads/info7-LibAddonMenu-2.0.html]a[/url] \
            [url=https://www.esoui.com/downloads/info7-LibAddonMenu-2.0.html]b[/url]";

        assert_eq!(links(&details(description)), vec![7]);
    }
}
