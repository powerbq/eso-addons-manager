use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use futures::future;
use regex::Regex;

use crate::api::{self, ApiVersion, Client};
use crate::bbcode;
use crate::catalogue::{Addon, Category, Details};

const FILE_LIST: &str = "filelist.json";
const CATEGORY_LIST: &str = "categorylist.json";

static INFO_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)esoui\.com/downloads/info(\d+)-").unwrap());
static FILEINFO_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)esoui\.com/\S*?fileinfo\.php\?\S*?\bid=(\d+)").unwrap());

async fn file_list(client: &Client, version: ApiVersion) -> Result<Vec<Addon>, api::Error> {
    let raw = client.get_raw(version, FILE_LIST).await?;
    api::parse_file_list(version, &raw).map_err(api::Error::parse)
}

async fn categories(client: &Client, version: ApiVersion) -> Result<Vec<Category>, api::Error> {
    let raw = client.get_raw(version, CATEGORY_LIST).await?;
    api::parse_categories(version, &raw).map_err(api::Error::parse)
}

pub async fn catalogue(
    client: &Client,
    versions: &[ApiVersion],
) -> Result<(Vec<Addon>, Vec<Category>, HashMap<String, ApiVersion>), api::Error> {
    let fetched = future::join_all(versions.iter().map(|&version| async move {
        let (addons, categories) =
            future::join(file_list(client, version), categories(client, version)).await;
        (version, addons, categories)
    }))
    .await;

    let mut sources = Vec::new();
    for (version, addons, categories) in fetched {
        let addons = addons?;
        let categories = categories.unwrap_or_else(|error| {
            tracing::warn!("cannot load categories for {}: {error}", version.as_key());
            Vec::new()
        });
        sources.push((version, addons, categories));
    }

    Ok(merge(sources))
}

fn merge(
    sources: Vec<(ApiVersion, Vec<Addon>, Vec<Category>)>,
) -> (Vec<Addon>, Vec<Category>, HashMap<String, ApiVersion>) {
    let mut addons = Vec::new();
    let mut origin = HashMap::new();
    let mut category_ids = HashSet::new();
    let mut categories = Vec::new();

    for (version, source_addons, source_categories) in sources {
        for addon in source_addons {
            if !origin.contains_key(&addon.uid) {
                origin.insert(addon.uid.clone(), version);
                addons.push(addon);
            }
        }
        for category in source_categories {
            if category_ids.insert(category.id.clone()) {
                categories.push(category);
            }
        }
    }

    (addons, categories, origin)
}

pub fn links(details: &Details) -> Vec<u32> {
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

fn esoui_uid(url: &str) -> Option<u32> {
    INFO_LINK
        .captures(url)
        .or_else(|| FILEINFO_LINK.captures(url))
        .and_then(|captures| captures.get(1))
        .and_then(|value| value.as_str().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(description: &str) -> Details {
        Details {
            version: String::new(),
            md5: String::new(),
            download_url: String::new(),
            description: description.to_owned(),
            changelog: String::new(),
            screenshots: Vec::new(),
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

    fn addon(uid: &str, name: &str) -> Addon {
        api::FileListEntry {
            uid: uid.into(),
            name: name.into(),
            author: String::new(),
            version: String::new(),
            category_id: String::new(),
            info_url: String::new(),
            date: 0,
            downloads: 0,
            monthly_downloads: 0,
            favourites: 0,
            directories: Vec::new(),
            thumbnails: Vec::new(),
            screenshots: Vec::new(),
        }
        .into()
    }

    fn category(id: &str) -> Category {
        api::CategoryEntry {
            id: id.into(),
            title: String::new(),
            icon_url: String::new(),
            parent_ids: Vec::new(),
        }
        .into()
    }

    #[test]
    fn merge_unions_by_uid_earlier_source_wins() {
        let (addons, categories, origin) = merge(vec![
            (
                ApiVersion::V3,
                vec![addon("1", "v3-one"), addon("2", "shared-v3")],
                vec![category("a")],
            ),
            (
                ApiVersion::V4,
                vec![addon("2", "shared-v4"), addon("3", "v4-only")],
                vec![category("a"), category("b")],
            ),
        ]);

        let names: Vec<&str> = addons.iter().map(|addon| addon.name.as_str()).collect();
        assert_eq!(names, ["v3-one", "shared-v3", "v4-only"]);

        assert_eq!(origin.get("1"), Some(&ApiVersion::V3));
        assert_eq!(origin.get("2"), Some(&ApiVersion::V3));
        assert_eq!(origin.get("3"), Some(&ApiVersion::V4));

        let ids: Vec<&str> = categories
            .iter()
            .map(|category| category.id.as_str())
            .collect();
        assert_eq!(ids, ["a", "b"]);
    }
}
