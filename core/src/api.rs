use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use crate::html;

const API_PREFIX: &str = "https://api.mmoui.com/v3/game/ESO";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("network request failed: {0}")]
    Request(Arc<reqwest::Error>),
    #[error("addon {0} not found")]
    NotFound(String),
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

    pub async fn file_list(&self) -> Result<Vec<FileListEntry>, Error> {
        self.get_json("filelist.json").await
    }

    pub async fn categories(&self) -> Result<Vec<CategoryEntry>, Error> {
        self.get_json("categorylist.json").await
    }

    pub async fn file_details(&self, uid: &str) -> Result<FileDetails, Error> {
        let details: Vec<FileDetails> = self.get_json(&format!("filedetails/{uid}.json")).await?;

        details
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

    async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let url = format!("{API_PREFIX}/{path}");
        tracing::debug!("GET {url}");
        let response = self.http.get(url).send().await?.error_for_status()?;

        Ok(response.json().await?)
    }
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
    #[serde(rename = "UID")]
    pub uid: String,
    #[serde(rename = "UIName", default, deserialize_with = "html_text")]
    pub name: String,
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

fn html_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let text: String = nullable(deserializer)?;

    Ok(html::decode_entities(&text))
}

fn nullable<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn lenient_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
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
}
