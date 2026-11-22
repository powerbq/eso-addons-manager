use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::Path;
use std::{fs, io};

use zip::ZipArchive;

use crate::cache::Index;
use crate::catalogue::{Addon, Catalogue};
use crate::config::Config;
use crate::deps;

pub fn folders(target: &Path) -> io::Result<Vec<String>> {
    let entries = match fs::read_dir(target) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    let mut folders = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if !path.is_dir() || is_library(&path) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            folders.push(name.to_owned());
        }
    }

    folders.sort();
    tracing::info!(
        "scanned {}: {} addon folders",
        target.display(),
        folders.len()
    );
    Ok(folders)
}

#[derive(Debug, Default)]
pub struct Found<'a> {
    pub addons: Vec<&'a Addon>,
    pub choices: BTreeMap<String, Vec<&'a Addon>>,
}

pub fn new_addons<'a>(
    folders: &[String],
    catalogue: &'a Catalogue,
    config: &Config,
    index: &Index,
    mut provides: impl FnMut(&Addon) -> Option<Vec<String>>,
) -> Found<'a> {
    let providers = catalogue.providers();
    let mut found = Found::default();
    let mut undecided = Vec::new();

    for folder in folders {
        let Some(candidates) = providers.get(folder.as_str()) else {
            continue;
        };
        if candidates
            .iter()
            .any(|addon| config.addons.contains_key(&addon.uid))
        {
            continue;
        }

        match deps::provider(folder, candidates, config)
            .map(deps::Decision::addon)
            .or_else(|| recorded(candidates, index))
        {
            Some(provider) => {
                if !found.addons.iter().any(|addon| addon.uid == provider.uid) {
                    tracing::debug!("{folder} -> {} ({})", provider.name, provider.uid);
                    found.addons.push(provider);
                }
            }
            None => undecided.push((folder, candidates)),
        }
    }

    for (folder, candidates) in undecided {
        if candidates
            .iter()
            .any(|candidate| found.addons.iter().any(|addon| addon.uid == candidate.uid))
        {
            continue;
        }

        // Drop candidates that do not really provide the folder (e.g. language patches that only
        // contain the folder without a manifest declaring it); a lone survivor needs no choice.
        let real = deps::real_providers(folder, candidates, &mut provides);
        let pool = if real.is_empty() {
            candidates.clone()
        } else {
            real
        };
        match pool.as_slice() {
            [only] => {
                if !found.addons.iter().any(|addon| addon.uid == only.uid) {
                    tracing::debug!(
                        "{folder} -> {} ({}) after filtering patches",
                        only.name,
                        only.uid
                    );
                    found.addons.push(only);
                }
            }
            _ => {
                found.choices.insert(folder.clone(), pool);
            }
        }
    }

    tracing::debug!(
        "scan: {} addons to install, {} folders need a choice",
        found.addons.len(),
        found.choices.len()
    );
    found
}

fn recorded<'a>(candidates: &[&'a Addon], index: &Index) -> Option<&'a Addon> {
    let mut recorded = candidates
        .iter()
        .filter(|addon| index.contains_key(&addon.uid));

    match (recorded.next(), recorded.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

pub fn unrecognised<'a>(
    folders: &'a [String],
    catalogue: &Catalogue,
    custom: &BTreeSet<String>,
) -> Vec<&'a str> {
    let providers = catalogue.providers();

    folders
        .iter()
        .map(String::as_str)
        .filter(|folder| !providers.contains_key(folder) && !custom.contains(*folder))
        .collect()
}

pub fn custom_folders(directory: &Path) -> BTreeSet<String> {
    crate::sync::custom_archives(directory)
        .iter()
        .filter_map(|path| {
            let file = match File::open(path) {
                Ok(file) => file,
                Err(error) => {
                    tracing::warn!("cannot open custom archive {}: {error}", path.display());
                    return None;
                }
            };
            match ZipArchive::new(file) {
                Ok(archive) => Some(archive),
                Err(error) => {
                    tracing::warn!("cannot read custom archive {}: {error}", path.display());
                    None
                }
            }
        })
        .flat_map(|mut archive| {
            (0..archive.len())
                .filter_map(|index| {
                    let file = archive.by_index(index).ok()?;
                    let path = file.enclosed_name()?;
                    let mut components = path.components();
                    let first = components.next()?;
                    (file.is_dir() || components.next().is_some())
                        .then(|| first.as_os_str().to_str().map(str::to_owned))
                        .flatten()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn is_library(folder: &Path) -> bool {
    let Ok(entries) = fs::read_dir(folder) else {
        return false;
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extension.eq_ignore_ascii_case("txt") || extension.eq_ignore_ascii_case("addon")
                })
        })
        .filter(|path| path.is_file())
        .any(|path| {
            fs::read(&path).is_ok_and(|bytes| declares_library(&String::from_utf8_lossy(&bytes)))
        })
}

fn declares_library(manifest: &str) -> bool {
    manifest.trim_start_matches('\u{feff}').lines().any(|line| {
        line.trim()
            .strip_prefix("##")
            .and_then(|line| line.split_once(':'))
            .is_some_and(|(field, value)| {
                field.trim().eq_ignore_ascii_case("IsLibrary")
                    && value.trim().eq_ignore_ascii_case("true")
            })
    })
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::api::FileListEntry;
    use crate::cache::Archive;

    fn entry(uid: &str, directories: &[&str]) -> FileListEntry {
        FileListEntry {
            uid: uid.into(),
            name: format!("Addon {uid}"),
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
        }
    }

    fn uids<'a>(found: &Found<'a>) -> Vec<&'a str> {
        found
            .addons
            .iter()
            .map(|addon| addon.uid.as_str())
            .collect()
    }

    fn choices<'a>(found: &'a Found<'a>) -> Vec<(&'a str, Vec<&'a str>)> {
        found
            .choices
            .iter()
            .map(|(folder, candidates)| {
                (
                    folder.as_str(),
                    candidates.iter().map(|addon| addon.uid.as_str()).collect(),
                )
            })
            .collect()
    }

    fn folders_of(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    fn find<'a>(
        folders: &[String],
        catalogue: &'a Catalogue,
        config: &Config,
        index: &Index,
    ) -> Found<'a> {
        new_addons(folders, catalogue, config, index, |_| None)
    }

    #[test]
    fn drops_candidates_without_a_manifest_for_the_folder() {
        let catalogue = Catalogue::new([entry("1", &["Notifier"]), entry("2", &["Notifier"])], []);
        // addon 1 really provides the folder; addon 2 is a language patch that only contains it.
        let provides = |addon: &Addon| match addon.uid.as_str() {
            "1" => Some(vec!["Notifier".to_owned()]),
            _ => Some(Vec::new()),
        };

        let found = new_addons(
            &folders_of(&["Notifier"]),
            &catalogue,
            &Config::default(),
            &Index::default(),
            provides,
        );

        assert_eq!(uids(&found), ["1"]);
        assert!(found.choices.is_empty());
    }

    #[test]
    fn still_asks_when_several_real_providers_remain() {
        let catalogue = Catalogue::new([entry("1", &["Notifier"]), entry("2", &["Notifier"])], []);
        let provides = |_: &Addon| Some(vec!["Notifier".to_owned()]);

        let found = new_addons(
            &folders_of(&["Notifier"]),
            &catalogue,
            &Config::default(),
            &Index::default(),
            provides,
        );

        assert_eq!(choices(&found), [("Notifier", vec!["1", "2"])]);
    }

    #[test]
    fn lists_folders_except_libraries() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        for folder in ["Zeta", "Alpha", "LibFoo", "LibBom"] {
            fs::create_dir(root.join(folder)).unwrap();
        }
        fs::write(root.join("Alpha/Alpha.txt"), "## Title: Alpha\n").unwrap();
        fs::write(
            root.join("LibFoo/LibFoo.addon"),
            "## Title: LibFoo\r\n  ## islibrary :  TRUE \r\n",
        )
        .unwrap();
        fs::write(
            root.join("LibBom/LibBom.TXT"),
            "\u{feff}## IsLibrary: true\n",
        )
        .unwrap();
        fs::write(root.join("readme.txt"), "## IsLibrary: true\n").unwrap();

        assert_eq!(folders(root).unwrap(), ["Alpha", "Zeta"]);
    }

    #[test]
    fn missing_folder_has_no_addons() {
        let directory = tempfile::tempdir().unwrap();

        assert!(
            folders(&directory.path().join("AddOns"))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn matches_folders_to_catalogue() {
        let catalogue = Catalogue::new(
            [
                entry("1", &["Alpha"]),
                entry("2", &["Beta", "BetaData"]),
                entry("3", &["Shared", "Extra"]),
                entry("4", &["Shared"]),
                entry("5", &["Picked"]),
                entry("6", &["Picked"]),
                entry("7", &["Installed"]),
            ],
            [],
        );
        let config = Config {
            addons: BTreeMap::from([("7".to_owned(), String::new())]),
            selected_libraries: BTreeMap::from([("Picked".to_owned(), "6".to_owned())]),
            ..Config::default()
        };

        let found = find(
            &folders_of(&[
                "Alpha",
                "Beta",
                "BetaData",
                "Shared",
                "Picked",
                "Installed",
                "Unknown",
            ]),
            &catalogue,
            &config,
            &Index::default(),
        );

        assert_eq!(uids(&found), ["1", "2", "6"]);
        assert_eq!(choices(&found), [("Shared", vec!["3", "4"])]);
    }

    #[test]
    fn skips_folders_provided_by_an_installed_addon() {
        let catalogue = Catalogue::new(
            [entry("1", &["Radar", "LibMenu"]), entry("2", &["Radar"])],
            [],
        );
        let config = Config {
            addons: BTreeMap::from([("1".to_owned(), String::new())]),
            ..Config::default()
        };

        let found = find(
            &folders_of(&["Radar"]),
            &catalogue,
            &config,
            &Index::default(),
        );

        assert!(found.addons.is_empty());
        assert!(found.choices.is_empty());
    }

    #[test]
    fn uses_the_recorded_archive() {
        let catalogue = Catalogue::new([entry("1", &["Lib"]), entry("2", &["Lib"])], []);
        let index = Index::from(BTreeMap::from([("2".to_owned(), Archive::default())]));

        let found = find(
            &folders_of(&["Lib"]),
            &catalogue,
            &Config::default(),
            &index,
        );

        assert_eq!(uids(&found), ["2"]);
        assert!(found.choices.is_empty());
    }

    #[test]
    fn needs_no_choice_when_a_candidate_is_found_through_another_folder() {
        let catalogue =
            Catalogue::new([entry("1", &["Radar", "Menu"]), entry("2", &["Radar"])], []);

        let found = find(
            &folders_of(&["Radar", "Menu"]),
            &catalogue,
            &Config::default(),
            &Index::default(),
        );

        assert_eq!(uids(&found), ["1"]);
        assert!(found.choices.is_empty());
    }

    #[test]
    fn lists_folders_unknown_to_the_catalogue() {
        let catalogue = Catalogue::new([entry("1", &["Alpha", "AlphaData"])], []);

        let custom = BTreeSet::from(["Mine".to_owned()]);

        assert_eq!(
            unrecognised(
                &folders_of(&["Alpha", "Mine", "AlphaData", "Old"]),
                &catalogue,
                &custom
            ),
            ["Old"]
        );
    }

    #[test]
    fn reads_top_level_folders_of_custom_archives() {
        use std::io::Write;

        let directory = tempfile::tempdir().unwrap();
        let mut writer =
            zip::ZipWriter::new(File::create(directory.path().join("Mine.zip")).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        writer.add_directory("Empty/", options).unwrap();
        writer.start_file("Mine/Mine.txt", options).unwrap();
        writer.write_all(b"## Title: Mine").unwrap();
        writer.start_file("readme.txt", options).unwrap();
        writer.finish().unwrap();
        fs::write(directory.path().join("notes.txt"), "").unwrap();

        assert_eq!(
            custom_folders(directory.path()),
            BTreeSet::from(["Empty".to_owned(), "Mine".to_owned()])
        );
        assert!(custom_folders(&directory.path().join("missing")).is_empty());
    }

    #[test]
    fn ignores_selection_that_does_not_provide_the_folder() {
        let catalogue = Catalogue::new([entry("1", &["Lib"]), entry("2", &["Lib"])], []);
        let config = Config {
            selected_libraries: BTreeMap::from([("Lib".to_owned(), "9".to_owned())]),
            ..Config::default()
        };

        let found = find(
            &folders_of(&["Lib"]),
            &catalogue,
            &config,
            &Index::default(),
        );

        assert!(found.addons.is_empty());
        assert_eq!(choices(&found), [("Lib", vec!["1", "2"])]);
    }
}
