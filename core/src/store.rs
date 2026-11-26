use std::sync::LazyLock;

use regex::Regex;

use crate::api::{self, Client};
use crate::bbcode;
use crate::catalogue::{Addon, Category, Details};

const FILE_LIST: &str = "filelist.json";
const CATEGORY_LIST: &str = "categorylist.json";

static INFO_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)esoui\.com/downloads/info(\d+)-").unwrap());
static FILEINFO_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)esoui\.com/\S*?fileinfo\.php\?\S*?\bid=(\d+)").unwrap());

pub async fn file_list(client: &Client) -> Result<Vec<Addon>, api::Error> {
    let raw = client.get_raw(FILE_LIST).await?;
    api::parse_file_list(client.version(), &raw).map_err(api::Error::parse)
}

pub async fn categories(client: &Client) -> Result<Vec<Category>, api::Error> {
    let raw = client.get_raw(CATEGORY_LIST).await?;
    api::parse_categories(client.version(), &raw).map_err(api::Error::parse)
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
}
