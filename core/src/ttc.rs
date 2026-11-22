use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use zip::ZipArchive;

use crate::api;

pub const DIRECTORY: &str = "ttc";
pub const ADDON_UID: &str = "1245";
pub const ADDON_DIRECTORY: &str = "TamrielTradeCentre";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Region {
    #[default]
    Eu,
    Na,
}

impl Region {
    pub const ALL: [Region; 2] = [Region::Eu, Region::Na];

    fn host(self) -> &'static str {
        match self {
            Region::Eu => "eu.tamrieltradecentre.com",
            Region::Na => "us.tamrieltradecentre.com",
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            Region::Eu => "EU",
            Region::Na => "NA",
        }
    }

    fn archive(self) -> &'static str {
        match self {
            Region::Eu => "PriceTableEU.zip",
            Region::Na => "PriceTableNA.zip",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|region| region.as_key() == key)
    }
}

pub fn is_cache_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|name| name.to_str());
    Region::ALL
        .into_iter()
        .any(|region| Some(region.archive()) == name)
}

#[derive(Debug, Clone)]
pub enum Outcome {
    UpToDate,
    Updated,
    Failed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot reach Tamriel Trade Centre: {0}")]
    Request(#[from] api::Error),
    #[error("cannot read the Tamriel Trade Centre response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("cannot update the price table files: {0}")]
    Io(#[from] io::Error),
    #[error("cannot read the price table archive: {0}")]
    Zip(#[from] zip::result::ZipError),
}

#[derive(Deserialize)]
struct ClientVersion {
    #[serde(rename = "PriceTableVersion")]
    price_table_version: u64,
}

// Make sure the region's price-table archive is cached and current, then return its path and
// whether a fresh copy was downloaded. The archive is fed into the sync plan as a source, so the
// plan itself writes the price files into the TTC folder (and removes the other region's table as
// an extraneous file). The download decision is the same as before: compare the remote
// `PriceTableVersion` with the version stored inside the cached archive, download only when they
// differ. Offline, the existing cached archive is reused so prices are still mirrored; with no
// cache at all the caller is told prices are unavailable.
pub async fn ensure_cached(
    client: &api::Client,
    region: Region,
    cache: &Path,
) -> Result<(PathBuf, bool), Error> {
    let archive = cache.join(region.archive());

    let remote = match remote_version(client, region).await {
        Ok(remote) => remote,
        Err(error) => {
            if archive.is_file() {
                tracing::warn!("cannot check TTC price version ({error}); using cached prices");
                return Ok((archive, false));
            }
            return Err(error);
        }
    };

    if local_version(&archive) == Some(remote) {
        tracing::info!("TTC prices already current in cache (version {remote})");
        return Ok((archive, false));
    }

    tracing::info!("downloading TTC prices (version {remote})");
    let bytes = client
        .download(&format!("https://{}/Download/PriceTable", region.host()))
        .await?;

    fs::create_dir_all(cache)?;
    let temporary = archive.with_extension("zip.tmp");
    fs::write(&temporary, &bytes)?;
    fs::rename(&temporary, &archive)?;

    Ok((archive, true))
}

async fn remote_version(client: &api::Client, region: Region) -> Result<u64, Error> {
    let url = format!("https://{}/api/GetTradeClientVersion", region.host());
    let version: ClientVersion = serde_json::from_str(&client.text(&url).await?)?;
    Ok(version.price_table_version)
}

fn local_version(archive: &Path) -> Option<u64> {
    let mut zip = ZipArchive::new(fs::File::open(archive).ok()?).ok()?;
    for index in 0..zip.len() {
        let Ok(entry) = zip.by_index(index) else {
            continue;
        };
        let is_price = entry
            .enclosed_name()
            .and_then(|path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
            .is_some_and(|name| name.starts_with("PriceTable") && name.ends_with(".lua"));
        if is_price {
            let mut line = String::new();
            io::BufReader::new(entry).read_line(&mut line).ok()?;
            return line
                .strip_prefix("--Version = ")
                .and_then(|value| value.trim().parse().ok());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use zip::write::SimpleFileOptions;

    use super::*;

    #[test]
    fn region_keys_round_trip() {
        for region in Region::ALL {
            assert_eq!(Region::from_key(region.as_key()), Some(region));
        }
        assert_eq!(Region::from_key("nonsense"), None);
    }

    #[test]
    fn local_version_reads_the_price_table_version() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("PriceTableEU.zip");

        let mut writer = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        writer
            .start_file("PriceTableEU.lua", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"--Version = 3776\nreturn {}\n").unwrap();
        writer.finish().unwrap();

        assert_eq!(local_version(&archive), Some(3776));
        assert_eq!(local_version(&root.path().join("missing.zip")), None);
    }
}
