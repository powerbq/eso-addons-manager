use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct FileListEntry {
    #[serde(deserialize_with = "flexible_string")]
    pub id: String,
    #[serde(rename = "categoryId", default, deserialize_with = "flexible_string")]
    pub category_id: String,
    #[serde(default, deserialize_with = "super::html_text")]
    pub version: String,
    #[serde(
        rename = "lastUpdate",
        default,
        deserialize_with = "super::lenient_number"
    )]
    pub last_update: u64,
    #[serde(default, deserialize_with = "super::html_text")]
    pub title: String,
    #[serde(default, deserialize_with = "super::html_text")]
    pub author: String,
    #[serde(rename = "fileInfoUri", default, deserialize_with = "super::html_text")]
    pub file_info_uri: String,
    #[serde(default, deserialize_with = "super::lenient_number")]
    pub downloads: u64,
    #[serde(
        rename = "downloadsMonthly",
        default,
        deserialize_with = "super::lenient_number"
    )]
    pub downloads_monthly: u64,
    #[serde(default, deserialize_with = "super::lenient_number")]
    pub favorites: u64,
    #[serde(default, deserialize_with = "super::nullable")]
    pub addons: Vec<AddonEntry>,
}

#[derive(Debug, Deserialize)]
pub struct AddonEntry {
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct CategoryEntry {
    #[serde(deserialize_with = "flexible_string")]
    pub id: String,
    #[serde(default, deserialize_with = "super::html_text")]
    pub title: String,
    #[serde(rename = "iconUrl", default, deserialize_with = "super::html_text")]
    pub icon_url: String,
    #[serde(rename = "parentIds", default, deserialize_with = "flexible_strings")]
    pub parent_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileDetails {
    #[serde(default, deserialize_with = "super::html_text")]
    pub version: String,
    #[serde(default, deserialize_with = "super::nullable")]
    pub checksum: String,
    #[serde(rename = "downloadUri", default, deserialize_with = "super::html_text")]
    pub download_uri: String,
    #[serde(default, deserialize_with = "super::html_text")]
    pub description: String,
    #[serde(rename = "changeLog", default, deserialize_with = "super::html_text")]
    pub change_log: String,
    #[serde(default, deserialize_with = "super::nullable")]
    pub images: Vec<Image>,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    #[serde(rename = "thumbUrl", default, deserialize_with = "super::html_text")]
    pub thumb_url: String,
    #[serde(rename = "imageUrl", default, deserialize_with = "super::html_text")]
    pub image_url: String,
}

fn flexible_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Text(String),
        Number(i64),
    }

    Ok(match Option::<Value>::deserialize(deserializer)? {
        Some(Value::Text(text)) => text,
        Some(Value::Number(number)) => number.to_string(),
        None => String::new(),
    })
}

fn flexible_strings<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Text(String),
        Number(i64),
    }

    Ok(Option::<Vec<Value>>::deserialize(deserializer)?
        .unwrap_or_default()
        .into_iter()
        .map(|value| match value {
            Value::Text(text) => text,
            Value::Number(number) => number.to_string(),
        })
        .collect())
}
