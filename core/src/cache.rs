use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};

use md5::{Digest, Md5};

use crate::api;
use crate::catalogue::Addon;
use crate::config::Archive;

pub const DIRECTORY: &str = "addons";

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Request(#[from] api::Error),
    #[error("checksum mismatch: expected {expected}, got {actual}")]
    Checksum { expected: String, actual: String },
    #[error("cannot write the archive: {0}")]
    Io(Arc<io::Error>),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

#[derive(Debug, Clone)]
pub struct Download {
    pub bytes: Vec<u8>,
    pub archive: Archive,
}

pub fn archive_path(directory: &Path, addon: &Addon) -> PathBuf {
    let name: String = addon
        .name
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == '_')
        .collect();

    directory.join(format!("{name}_{}.zip", addon.uid))
}

pub fn is_valid(path: &Path, addon: &Addon, archive: Option<&Archive>) -> bool {
    let Some(archive) = archive else {
        return false;
    };
    archive.version == addon.version && cached(path, &archive.md5).is_some()
}

pub fn cached(path: &Path, md5: &str) -> Option<Vec<u8>> {
    fs::read(path)
        .ok()
        .filter(|bytes| md5.eq_ignore_ascii_case(&md5_hex(bytes)))
}

pub async fn fetch(client: &api::Client, uid: &str, path: &Path) -> Result<Download, Error> {
    let details = client.file_details(uid).await?;
    let bytes = match cached(path, &details.md5) {
        Some(bytes) => {
            tracing::debug!("cache hit for {uid} ({})", details.version);
            bytes
        }
        None => {
            tracing::debug!("downloading {uid} ({})", details.version);
            client.download(&details.download_url).await?
        }
    };

    Ok(Download {
        bytes,
        archive: Archive {
            version: details.version,
            md5: details.md5,
        },
    })
}

pub fn store(path: &Path, download: &Download) -> Result<(), Error> {
    let actual = md5_hex(&download.bytes);
    if !download.archive.md5.eq_ignore_ascii_case(&actual) {
        return Err(Error::Checksum {
            expected: download.archive.md5.clone(),
            actual,
        });
    }

    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }

    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");

    fs::write(&temporary, &download.bytes)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn md5_hex(bytes: &[u8]) -> String {
    Md5::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::FileListEntry;

    const CONTENT: &[u8] = b"zip bytes";
    const CONTENT_MD5: &str = "91e39f164925e8b875b65c81497a0106";

    fn addon(uid: &str, name: &str, version: &str) -> Addon {
        Addon::from(FileListEntry {
            uid: uid.into(),
            name: name.into(),
            author: String::new(),
            version: version.into(),
            category_id: String::new(),
            info_url: String::new(),
            date: 0,
            downloads: 0,
            monthly_downloads: 0,
            favourites: 0,
            directories: Vec::new(),
            thumbnails: Vec::new(),
            screenshots: Vec::new(),
        })
    }

    fn archive(version: &str, md5: &str) -> Archive {
        Archive {
            version: version.to_owned(),
            md5: md5.to_owned(),
        }
    }

    #[test]
    fn computes_md5_as_hex() {
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(CONTENT), CONTENT_MD5);
    }

    #[test]
    fn names_archives_like_the_old_version() {
        let directory = Path::new(DIRECTORY);

        assert_eq!(
            archive_path(directory, &addon("1399", "Votan's Minimap", "")),
            Path::new("addons/VotansMinimap_1399.zip")
        );
        assert_eq!(
            archive_path(directory, &addon("1", "Slightly Improved™ Bars_2", "")),
            Path::new("addons/SlightlyImprovedBars_2_1.zip")
        );
        assert_eq!(
            archive_path(directory, &addon("2", "EsoUA — Українська", "")),
            Path::new("addons/EsoUAУкраїнська_2.zip")
        );
    }

    #[test]
    fn archive_is_valid_only_with_matching_version_and_checksum() {
        let directory = tempfile::tempdir().unwrap();
        let addon = addon("1", "Addon", "1.0");
        let path = archive_path(directory.path(), &addon);

        assert!(!is_valid(&path, &addon, Some(&archive("1.0", CONTENT_MD5))));

        fs::write(&path, CONTENT).unwrap();

        assert!(is_valid(&path, &addon, Some(&archive("1.0", CONTENT_MD5))));
        assert!(is_valid(
            &path,
            &addon,
            Some(&archive("1.0", &CONTENT_MD5.to_uppercase()))
        ));
        assert!(!is_valid(&path, &addon, None));
        assert!(!is_valid(&path, &addon, Some(&archive("0.9", CONTENT_MD5))));
        assert!(!is_valid(&path, &addon, Some(&archive("1.0", "0000"))));
    }

    #[test]
    fn reuses_a_cached_file_only_with_the_same_checksum() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Addon_1.zip");

        assert_eq!(cached(&path, CONTENT_MD5), None);

        fs::write(&path, CONTENT).unwrap();

        assert_eq!(cached(&path, CONTENT_MD5).as_deref(), Some(CONTENT));
        assert_eq!(cached(&path, "0000"), None);
    }

    #[test]
    fn stores_verified_download() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(DIRECTORY).join("Addon_1.zip");
        let download = Download {
            bytes: CONTENT.to_vec(),
            archive: archive("1.0", CONTENT_MD5),
        };

        store(&path, &download).unwrap();

        assert_eq!(fs::read(&path).unwrap(), CONTENT);
        assert_eq!(
            fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1,
            "no temporary file is left"
        );
    }

    #[test]
    fn rejects_download_with_wrong_checksum() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Addon_1.zip");
        let download = Download {
            bytes: CONTENT.to_vec(),
            archive: archive("1.0", "d41d8cd98f00b204e9800998ecf8427e"),
        };

        assert!(matches!(
            store(&path, &download),
            Err(Error::Checksum { .. })
        ));
        assert!(!path.exists());
    }
}
