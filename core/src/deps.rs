use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::{Component, Path};
use std::sync::Arc;

use crate::catalogue::Addon;
use crate::config::Config;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("cannot read the archive: {0}")]
    Io(Arc<io::Error>),
    #[error("invalid archive: {0}")]
    Zip(Arc<zip::result::ZipError>),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Zip(Arc::new(error))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifests {
    pub provides: Vec<String>,
    pub depends_on: Vec<String>,
    pub folders: Vec<String>,
}

pub fn read_archive(path: &Path) -> Result<Manifests, Error> {
    read(File::open(path)?)
}

pub fn read(reader: impl Read + Seek) -> Result<Manifests, Error> {
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut manifests = Manifests::default();

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(name) = entry_path(&file) else {
            continue;
        };
        if let Some((folder, _)) = name.split_once('/') {
            push_unique(&mut manifests.folders, folder);
        }
        if !file.is_file() {
            continue;
        }

        let path = Path::new(&name);
        if !is_manifest(path) {
            continue;
        }

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let text = String::from_utf8_lossy(&bytes);

        if declares_title(&text)
            && let Some(directory) = path.file_stem().and_then(|stem| stem.to_str())
        {
            push_unique(&mut manifests.provides, directory);
        }
        for directory in dependencies(&text) {
            push_unique(&mut manifests.depends_on, directory);
        }
    }

    Ok(manifests)
}

#[derive(Debug, Clone, Copy)]
pub enum Decision<'a> {
    Single(&'a Addon),
    Installed(&'a Addon),
    Selected(&'a Addon),
}

impl<'a> Decision<'a> {
    pub fn addon(self) -> &'a Addon {
        match self {
            Self::Single(addon) | Self::Installed(addon) | Self::Selected(addon) => addon,
        }
    }
}

pub fn provider<'a>(
    directory: &str,
    candidates: &[&'a Addon],
    config: &Config,
) -> Option<Decision<'a>> {
    if let [only] = candidates {
        return Some(Decision::Single(only));
    }

    let mut installed = candidates
        .iter()
        .filter(|addon| config.addons.contains_key(&addon.uid));
    if let (Some(only), None) = (installed.next(), installed.next()) {
        return Some(Decision::Installed(only));
    }

    config
        .selected_libraries
        .get(directory)
        .and_then(|uid| candidates.iter().find(|addon| &addon.uid == uid))
        .map(|addon| Decision::Selected(addon))
}

pub fn entry_path<R: Read>(file: &zip::read::ZipFile<'_, R>) -> Option<String> {
    let enclosed = file.enclosed_name()?;
    let parts: Vec<&str> = enclosed
        .components()
        .map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;

    if parts.is_empty() {
        return None;
    }

    let path = parts.join("/");
    Some(if file.is_dir() { path + "/" } else { path })
}

fn is_manifest(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("txt") || extension.eq_ignore_ascii_case("addon")
        })
}

fn declares_title(manifest: &str) -> bool {
    lines(manifest).any(|line| line.starts_with("## Title:"))
}

fn dependencies(manifest: &str) -> impl Iterator<Item = &str> {
    lines(manifest)
        .filter_map(|line| {
            line.strip_prefix("## DependsOn:")
                .or_else(|| line.strip_prefix("## PCDependsOn:"))
        })
        .flat_map(str::split_whitespace)
        .filter_map(|token| token.split(['=', '<', '>']).next())
        .filter(|directory| !directory.is_empty())
}

fn lines(manifest: &str) -> std::str::Lines<'_> {
    manifest.trim_start_matches('\u{feff}').lines()
}

fn push_unique(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|existing| existing == item) {
        list.push(item.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::{Cursor, Write};

    use zip::write::SimpleFileOptions;

    use super::*;
    use crate::api::FileListEntry;
    use crate::config::Archive;

    fn zip(files: &[(&str, &str)]) -> Cursor<Vec<u8>> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in files {
            if name.ends_with('/') {
                writer
                    .add_directory(*name, SimpleFileOptions::default())
                    .unwrap();
            } else {
                writer
                    .start_file(*name, SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(content.as_bytes()).unwrap();
            }
        }
        writer.finish().unwrap()
    }

    fn addon(uid: &str, directories: &[&str]) -> Addon {
        Addon::from(FileListEntry {
            uid: uid.into(),
            name: String::new(),
            author: String::new(),
            version: String::new(),
            category_id: String::new(),
            info_url: String::new(),
            date: 0,
            downloads: 0,
            monthly_downloads: 0,
            favourites: 0,
            directories: directories
                .iter()
                .map(|&directory| directory.into())
                .collect(),
            thumbnails: Vec::new(),
            screenshots: Vec::new(),
        })
    }

    fn chosen(directory: &str, candidates: &[&Addon], config: &Config) -> Option<String> {
        provider(directory, candidates, config).map(|decision| decision.addon().uid.clone())
    }

    fn config(installed: &[&str], selected: &[(&str, &str)]) -> Config {
        recorded(installed, selected, &[])
    }

    fn recorded(installed: &[&str], selected: &[(&str, &str)], archives: &[&str]) -> Config {
        Config {
            addons: installed
                .iter()
                .map(|&uid| (uid.to_owned(), String::new()))
                .collect(),
            selected_libraries: selected
                .iter()
                .map(|&(directory, uid)| (directory.to_owned(), uid.to_owned()))
                .collect::<BTreeMap<_, _>>(),
            archives: archives
                .iter()
                .map(|&uid| (uid.to_owned(), Archive::default()))
                .collect(),
            ..Config::default()
        }
    }

    #[test]
    fn reads_titles_and_dependencies_from_manifests() {
        let archive = zip(&[
            ("VotansMiniMap/", ""),
            (
                "VotansMiniMap/VotansMiniMap.addon",
                "## Title: Votan's Minimap\r\n\
                 ## DependsOn: LibHarvensAddonSettings>=20106 LibAsync>=30001\r\n\
                 ## OptionalDependsOn: LibGPS>=73 TweakIt\r\n",
            ),
            (
                "VotansMiniMap/Libs/LibBundled/LibBundled.TXT",
                "\u{feff}## Title: LibBundled\n## PCDependsOn: LibAsync LibStub<3\n",
            ),
            (
                "VotansMiniMap/readme.txt",
                "Not a manifest\n## DependsOn: LibReadme\n",
            ),
            ("VotansMiniMap/Main.lua", "## Title: not a manifest\n"),
        ]);

        assert_eq!(
            read(archive).unwrap(),
            Manifests {
                provides: vec!["VotansMiniMap".to_owned(), "LibBundled".to_owned()],
                depends_on: vec![
                    "LibHarvensAddonSettings".to_owned(),
                    "LibAsync".to_owned(),
                    "LibStub".to_owned(),
                    "LibReadme".to_owned(),
                ],
                folders: vec!["VotansMiniMap".to_owned()],
            }
        );
    }

    #[test]
    fn rejects_broken_archive() {
        assert!(matches!(
            read(Cursor::new(b"not a zip".to_vec())),
            Err(Error::Zip(_))
        ));
    }

    #[test]
    fn single_provider_is_used() {
        let only = addon("1", &["Lib"]);

        assert_eq!(
            chosen("Lib", &[&only], &config(&[], &[("Lib", "9")])),
            Some("1".to_owned())
        );
        assert_eq!(chosen("Lib", &[], &Config::default()), None);
    }

    #[test]
    fn several_providers_need_the_only_installed_one_or_the_selection() {
        let (a, b) = (addon("1", &["Lib"]), addon("2", &["Lib", "Extra"]));
        let candidates = [&a, &b];

        assert_eq!(
            chosen("Lib", &candidates, &config(&["2"], &[("Lib", "1")])),
            Some("2".to_owned()),
            "the only installed candidate decides"
        );
        assert_eq!(
            chosen("Lib", &candidates, &config(&["1", "2"], &[("Lib", "2")])),
            Some("2".to_owned())
        );
        assert_eq!(chosen("Lib", &candidates, &config(&["1", "2"], &[])), None);
        assert_eq!(
            chosen("Lib", &candidates, &config(&[], &[("Lib", "1")])),
            Some("1".to_owned())
        );
        assert_eq!(
            chosen("Lib", &candidates, &recorded(&[], &[("Lib", "9")], &["1"])),
            None
        );
        assert_eq!(
            chosen("Lib", &candidates, &recorded(&[], &[], &["2"])),
            None
        );
    }
}
