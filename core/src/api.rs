use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Deserializer};

use crate::catalogue::{Addon, Category, Details};
use crate::html;

pub(crate) mod v4;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ApiVersion {
    #[default]
    V3,
    V4,
}

impl ApiVersion {
    pub fn as_key(self) -> &'static str {
        match self {
            Self::V3 => "v3",
            Self::V4 => "v4",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_ascii_lowercase().as_str() {
            "v3" => Some(Self::V3),
            "v4" => Some(Self::V4),
            _ => None,
        }
    }
}

pub fn sources(include_outdated: bool) -> Vec<ApiVersion> {
    let mut sources = vec![ApiVersion::V3];
    if include_outdated {
        sources.push(ApiVersion::V4);
    }
    sources
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("network request failed: {0}")]
    Request(Arc<reqwest::Error>),
    #[error("could not parse the response: {0}")]
    Parse(Arc<serde_json::Error>),
    #[error("addon {0} not found")]
    NotFound(String),
}

impl Error {
    pub fn parse(error: serde_json::Error) -> Self {
        Self::Parse(Arc::new(error))
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(Arc::new(error))
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .user_agent(concat!("eso-addons-manager/", env!("CARGO_PKG_VERSION")))
            .build()?;

        Ok(Self { http })
    }

    fn prefix(version: ApiVersion) -> &'static str {
        match version {
            ApiVersion::V3 => "https://api.mmoui.com/v3/game/ESO",
            ApiVersion::V4 => "https://api.mmoui.com/v4/game/ESO",
        }
    }

    pub async fn file_details(&self, version: ApiVersion, uid: &str) -> Result<Details, Error> {
        let raw = self
            .get_raw(version, &format!("filedetails/{uid}.json"))
            .await?;

        parse_details(version, &raw)
            .map_err(Error::parse)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(uid.to_owned()))
    }

    pub async fn download(&self, url: &str) -> Result<Vec<u8>, Error> {
        tracing::debug!("download {url}");
        let response = self.http.get(url).send().await?.error_for_status()?;
        let bytes = response.bytes().await?.to_vec();
        tracing::debug!("downloaded {} bytes from {url}", bytes.len());

        Ok(bytes)
    }

    pub async fn text(&self, url: &str) -> Result<String, Error> {
        tracing::debug!("GET {url}");
        let response = self.http.get(url).send().await?.error_for_status()?;

        Ok(response.text().await?)
    }

    pub(crate) async fn get_raw(&self, version: ApiVersion, path: &str) -> Result<String, Error> {
        let url = format!("{}/{path}", Self::prefix(version));
        tracing::debug!("GET {url}");
        let response = self.http.get(url).send().await?.error_for_status()?;

        Ok(response.text().await?)
    }
}

pub fn parse_file_list(version: ApiVersion, raw: &str) -> Result<Vec<Addon>, serde_json::Error> {
    Ok(match version {
        ApiVersion::V3 => serde_json::from_str::<Vec<FileListEntry>>(raw)?
            .into_iter()
            .map(|entry| entry.into())
            .collect(),
        ApiVersion::V4 => serde_json::from_str::<Vec<v4::FileListEntry>>(raw)?
            .into_iter()
            .map(|entry| entry.into())
            .collect(),
    })
}

pub fn parse_categories(
    version: ApiVersion,
    raw: &str,
) -> Result<Vec<Category>, serde_json::Error> {
    Ok(match version {
        ApiVersion::V3 => serde_json::from_str::<Vec<CategoryEntry>>(raw)?
            .into_iter()
            .map(|entry| entry.into())
            .collect(),
        ApiVersion::V4 => serde_json::from_str::<Vec<v4::CategoryEntry>>(raw)?
            .into_iter()
            .map(|entry| entry.into())
            .collect(),
    })
}

pub fn parse_details(version: ApiVersion, raw: &str) -> Result<Vec<Details>, serde_json::Error> {
    Ok(match version {
        ApiVersion::V3 => serde_json::from_str::<Vec<FileDetails>>(raw)?
            .into_iter()
            .map(|details| details.into())
            .collect(),
        ApiVersion::V4 => serde_json::from_str::<Vec<v4::FileDetails>>(raw)?
            .into_iter()
            .map(|details| details.into())
            .collect(),
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileListEntry {
    #[serde(rename = "UID")]
    pub uid: String,
    #[serde(rename = "UIName", default, deserialize_with = "html_text")]
    pub name: String,
    #[serde(rename = "UIAuthorName", default, deserialize_with = "html_text")]
    pub author: String,
    #[serde(rename = "UIVersion", default, deserialize_with = "html_text")]
    pub version: String,
    #[serde(rename = "UICATID", default, deserialize_with = "nullable")]
    pub category_id: String,
    #[serde(rename = "UIFileInfoURL", default, deserialize_with = "html_text")]
    pub info_url: String,
    #[serde(rename = "UIDate", default, deserialize_with = "lenient_number")]
    pub date: u64,
    #[serde(
        rename = "UIDownloadTotal",
        default,
        deserialize_with = "lenient_number"
    )]
    pub downloads: u64,
    #[serde(
        rename = "UIDownloadMonthly",
        default,
        deserialize_with = "lenient_number"
    )]
    pub monthly_downloads: u64,
    #[serde(
        rename = "UIFavoriteTotal",
        default,
        deserialize_with = "lenient_number"
    )]
    pub favourites: u64,
    #[serde(rename = "UIDir", default, deserialize_with = "nullable")]
    pub directories: Vec<String>,
    #[serde(rename = "UIIMG_Thumbs", default, deserialize_with = "nullable")]
    pub thumbnails: Vec<String>,
    #[serde(rename = "UIIMGs", default, deserialize_with = "nullable")]
    pub screenshots: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryEntry {
    #[serde(rename = "UICATID")]
    pub id: String,
    #[serde(rename = "UICATTitle", default, deserialize_with = "html_text")]
    pub title: String,
    #[serde(rename = "UICATICON", default, deserialize_with = "html_text")]
    pub icon_url: String,
    #[serde(rename = "UICATParentIDs", default, deserialize_with = "nullable")]
    pub parent_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileDetails {
    #[serde(rename = "UIVersion", default, deserialize_with = "html_text")]
    pub version: String,
    #[serde(rename = "UIMD5", default, deserialize_with = "nullable")]
    pub md5: String,
    #[serde(rename = "UIDownload", default, deserialize_with = "html_text")]
    pub download_url: String,
    #[serde(rename = "UIDescription", default, deserialize_with = "html_text")]
    pub description: String,
    #[serde(rename = "UIChangeLog", default, deserialize_with = "html_text")]
    pub changelog: String,
}

pub(crate) fn html_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let text: String = nullable(deserializer)?;

    Ok(html::decode_entities(&text))
}

pub(crate) fn nullable<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

pub(crate) fn lenient_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Number(u64),
        Float(f64),
        Text(String),
    }

    Ok(match Option::<Raw>::deserialize(deserializer)? {
        Some(Raw::Number(value)) => value,
        Some(Raw::Float(value)) if value >= 0.0 => value as u64,
        Some(Raw::Text(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_list_entry_with_string_numbers() {
        let json = r#"{
            "UID": "7", "UICATID": "53", "UIVersion": "2.0 r43", "UIDate": 1778599485000,
            "UIName": "LibAddonMenu-2.0", "UIAuthorName": "sirinsidiator",
            "UIFileInfoURL": "https://www.esoui.com/downloads/info7-LibAddonMenu-2.0.html",
            "UIDownloadTotal": "6346016", "UIDownloadMonthly": "24549", "UIFavoriteTotal": "3102",
            "UIDir": ["LibAddonMenu-2.0"],
            "UIIMG_Thumbs": ["https://cdn-eso.mmoui.com/preview/tiny/pvw3588.jpg"],
            "UIIMGs": ["https://cdn-eso.mmoui.com/preview/pvw3588.jpg"], "UISiblings": null
        }"#;

        let entry: FileListEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.uid, "7");
        assert_eq!(entry.date, 1_778_599_485_000);
        assert_eq!(entry.downloads, 6_346_016);
        assert_eq!(entry.monthly_downloads, 24_549);
        assert_eq!(entry.favourites, 3_102);
        assert_eq!(entry.directories, ["LibAddonMenu-2.0"]);
        assert_eq!(
            entry.thumbnails,
            ["https://cdn-eso.mmoui.com/preview/tiny/pvw3588.jpg"]
        );
        assert_eq!(
            entry.screenshots,
            ["https://cdn-eso.mmoui.com/preview/pvw3588.jpg"]
        );
    }

    #[test]
    fn decodes_html_entities_in_text_fields() {
        let json =
            r#"{"UID": "1", "UIName": "&#1052;&#1086;&#1081;", "UIAuthorName": "A &amp; B"}"#;

        let entry: FileListEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.name, "Мой");
        assert_eq!(entry.author, "A & B");
    }

    #[test]
    fn tolerates_missing_and_null_fields() {
        let json = r#"{"UID": "1", "UIAuthorName": null, "UIDownloadTotal": null, "UIDir": null, "UIIMGs": null}"#;

        let entry: FileListEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.author, "");
        assert_eq!(entry.downloads, 0);
        assert!(entry.directories.is_empty());
        assert!(entry.screenshots.is_empty());
    }

    #[test]
    fn parses_v4_file_list_and_folds_nested_bundles_to_top_level() {
        let json = r#"[{
            "id": 3648, "categoryId": 39, "version": "1", "lastUpdate": 42,
            "title": "SuperStar", "author": "author",
            "fileInfoUri": "https://www.esoui.com/downloads/info3648-SuperStar.html",
            "downloads": 10, "downloadsMonthly": 2, "favorites": 3, "checksum": "x",
            "addonStatus": "Live",
            "addons": [
                { "path": "SuperStar" },
                { "path": "SuperStar/libs/LibSkillsFactory" }
            ]
        }]"#;

        let addons = parse_file_list(ApiVersion::V4, json).unwrap();

        assert_eq!(addons.len(), 1);
        assert_eq!(addons[0].uid, "3648");
        assert_eq!(addons[0].category_id, "39");
        assert_eq!(addons[0].name, "SuperStar");
        assert_eq!(addons[0].downloads, 10);
        assert_eq!(addons[0].directories, ["SuperStar"]);
        assert!(addons[0].screenshots.is_empty());
    }

    #[test]
    fn keeps_top_level_sibling_bundles_in_v4() {
        let json = r#"[{
            "id": 1135, "title": "TeamFormation", "author": "author",
            "addons": [
                { "path": "ProvisionsTeamFormation" },
                { "path": "LibAddonMenu-2.0" },
                { "path": "LibStub" }
            ]
        }]"#;

        let addons = parse_file_list(ApiVersion::V4, json).unwrap();

        assert_eq!(
            addons[0].directories,
            ["ProvisionsTeamFormation", "LibAddonMenu-2.0", "LibStub"]
        );
    }

    #[test]
    fn parses_v4_details_with_images_as_screenshots() {
        let json = r#"[{
            "id": 7, "version": "2.0", "checksum": "abc",
            "downloadUri": "https://cdn.example/getfile?id=7",
            "description": "d", "changeLog": "c",
            "images": [
                { "thumbUrl": "t1", "imageUrl": "i1" },
                { "thumbUrl": "t2", "imageUrl": "i2" }
            ]
        }]"#;

        let details = parse_details(ApiVersion::V4, json)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(details.md5, "abc");
        assert_eq!(details.download_url, "https://cdn.example/getfile?id=7");
        assert_eq!(details.screenshots.len(), 2);
        assert_eq!(details.screenshots[0].thumbnail, "t1");
        assert_eq!(details.screenshots[0].full, "i1");
    }

    #[test]
    fn parses_v4_categories_with_mixed_id_types() {
        let json = r#"[
            { "id": "157", "title": "Discontinued & Outdated", "iconUrl": "u", "parentIds": ["0"] },
            { "id": 53, "title": "Libraries", "parentIds": [] }
        ]"#;

        let categories = parse_categories(ApiVersion::V4, json).unwrap();

        assert_eq!(categories.len(), 2);
        assert_eq!(categories[0].id, "157");
        assert_eq!(categories[1].id, "53");
    }

    #[test]
    fn v4_tolerates_null_sequences() {
        let list = r#"[{ "id": 1, "title": "x", "author": "y", "addons": null }]"#;
        let addons = parse_file_list(ApiVersion::V4, list).unwrap();
        assert!(addons[0].directories.is_empty());

        let details = r#"[{ "id": 1, "version": "1", "images": null }]"#;
        let details = parse_details(ApiVersion::V4, details)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert!(details.screenshots.is_empty());
    }
}
