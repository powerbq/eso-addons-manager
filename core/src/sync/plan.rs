use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use zip::ZipArchive;

use super::{Action, Error, Event, Failure, Plan};
use crate::catalogue::{Addon, Catalogue};
use crate::config::Config;
use crate::exclusions::Exclusions;
use crate::{cache, deps};

pub struct Input<'a> {
    pub catalogue: &'a Catalogue,
    pub config: &'a Config,
    pub cache: &'a Path,
    pub custom: &'a Path,
    pub target: &'a Path,
}

pub fn plan(
    input: &Input<'_>,
    fetch: impl FnMut(&str, &Path) -> Result<cache::Download, cache::Error>,
    mut on_event: impl FnMut(Event),
) -> Result<Plan, Error> {
    let exclusions =
        Exclusions::new(input.config).map_err(|error| Error::Exclusion(Arc::new(error)))?;

    let mut resolver = Resolver {
        input,
        fetch,
        on_event: &mut on_event,
        config: input.config.clone(),
        addons: input
            .catalogue
            .addons()
            .iter()
            .map(|addon| (addon.uid.as_str(), addon))
            .collect(),
        providers: input.catalogue.providers(),
        plan: Plan::default(),
        processed: HashSet::new(),
        resolved: HashSet::new(),
        provided: HashSet::new(),
        unresolved: BTreeSet::new(),
        queue: VecDeque::new(),
        folders: HashMap::new(),
        source_of: HashMap::new(),
    };
    let mut plan = resolver.resolve()?;

    if plan.is_applicable() {
        on_event(Event::Comparing);
        plan.actions = compare(&plan.sources, &plan.superseded, input.target, &exclusions)?;
    }

    tracing::info!(
        "sync plan: {} sources, {} actions, {} missing, {} unknown, {} failed",
        plan.sources.len(),
        plan.actions.len(),
        plan.missing.len(),
        plan.unknown.len(),
        plan.failed.len()
    );
    Ok(plan)
}

struct Resolver<'a, F, E> {
    input: &'a Input<'a>,
    fetch: F,
    on_event: &'a mut E,
    config: Config,
    addons: HashMap<&'a str, &'a Addon>,
    providers: HashMap<&'a str, Vec<&'a Addon>>,
    plan: Plan,
    processed: HashSet<String>,
    resolved: HashSet<String>,
    provided: HashSet<String>,
    unresolved: BTreeSet<String>,
    queue: VecDeque<String>,
    folders: HashMap<PathBuf, Vec<String>>,
    source_of: HashMap<String, usize>,
}

impl<F, E> Resolver<'_, F, E>
where
    F: FnMut(&str, &Path) -> Result<cache::Download, cache::Error>,
    E: FnMut(Event),
{
    fn resolve(&mut self) -> Result<Plan, Error> {
        let installed: Vec<String> = self.config.addons.keys().cloned().collect();
        for uid in installed {
            match self.addons.get(uid.as_str()).copied() {
                Some(addon) => self.process(addon)?,
                None => self.plan.unknown.push(uid),
            }
        }

        let custom = custom_archives(self.input.custom);
        for path in &custom {
            self.read(path)?;
        }

        loop {
            self.drain()?;
            if !self.settle_shared_folders()? {
                break;
            }
        }

        self.plan.sources.extend(custom);
        self.plan.missing = self
            .unresolved
            .iter()
            .filter(|directory| !self.provided.contains(*directory))
            .cloned()
            .collect();

        Ok(std::mem::take(&mut self.plan))
    }

    fn drain(&mut self) -> Result<(), Error> {
        while let Some(directory) = self.queue.pop_front() {
            if !self.resolved.insert(directory.clone()) {
                continue;
            }

            let Some(candidates) = self.providers.get(directory.as_str()).cloned() else {
                self.unresolved.insert(directory);
                continue;
            };
            let Some(decision) = deps::provider(&directory, &candidates, &self.config) else {
                self.ask(directory, &candidates);
                continue;
            };
            if let deps::Decision::Selected(_) = decision {
                self.plan.selected.insert(directory);
            }
            self.process(decision.addon())?;
        }
        Ok(())
    }

    fn settle_shared_folders(&mut self) -> Result<bool, Error> {
        let mut holders: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, path) in self.plan.sources.iter().enumerate() {
            for folder in self.folders.get(path).into_iter().flatten() {
                holders.entry(folder.clone()).or_default().push(index);
            }
        }

        self.plan.superseded.clear();
        let mut processed_more = false;
        for (folder, holders) in holders {
            if holders.len() < 2 {
                continue;
            }

            let Some(candidates) = self.providers.get(folder.as_str()).cloned() else {
                continue;
            };
            let Some(decision) = deps::provider(&folder, &candidates, &self.config) else {
                self.ask(folder, &candidates);
                continue;
            };
            let provider = decision.addon();
            if !self.processed.contains(&provider.uid) {
                self.process(provider)?;
                processed_more = true;
                continue;
            }
            if let Some(&owner) = self.source_of.get(&provider.uid)
                && holders.contains(&owner)
            {
                self.keep(&folder, &holders, owner, decision);
            }
        }
        Ok(processed_more)
    }

    fn keep(&mut self, folder: &str, holders: &[usize], owner: usize, decision: deps::Decision) {
        if let deps::Decision::Selected(_) = decision {
            self.plan.selected.insert(folder.to_owned());
        }
        for &holder in holders.iter().filter(|holder| **holder != owner) {
            self.plan.superseded.insert((holder, folder.to_owned()));
        }
    }

    fn ask(&mut self, directory: String, candidates: &[&Addon]) {
        let uids = candidates.iter().map(|addon| addon.uid.clone()).collect();
        self.plan.choices.insert(directory, uids);
    }

    fn process(&mut self, addon: &Addon) -> Result<(), Error> {
        if !self.processed.insert(addon.uid.clone()) {
            return Ok(());
        }

        if let Some(path) = self.obtain(addon) {
            self.read(&path)?;
            self.source_of
                .insert(addon.uid.clone(), self.plan.sources.len());
            self.plan.sources.push(path);
            self.plan.used.insert(addon.uid.clone());
        }
        Ok(())
    }

    fn obtain(&mut self, addon: &Addon) -> Option<PathBuf> {
        let path = cache::archive_path(self.input.cache, addon);
        if cache::is_valid(&path, addon, self.config.archives.get(&addon.uid)) {
            return Some(path);
        }

        (self.on_event)(Event::Downloading {
            name: addon.name.clone(),
        });

        let stored = (self.fetch)(&addon.uid, &path).and_then(|download| {
            cache::store(&path, &download)?;
            Ok(download.archive)
        });

        match stored {
            Ok(archive) => {
                self.config
                    .archives
                    .insert(addon.uid.clone(), archive.clone());
                self.plan.archives.insert(addon.uid.clone(), archive);
                Some(path)
            }
            Err(error) => {
                self.plan.failed.push(Failure {
                    uid: addon.uid.clone(),
                    name: addon.name.clone(),
                    error: error.to_string(),
                });
                None
            }
        }
    }

    fn read(&mut self, path: &Path) -> Result<(), Error> {
        let manifests = deps::read_archive(path).map_err(|error| Error::Archive {
            path: path.to_owned(),
            error,
        })?;

        self.provided.extend(manifests.provides);
        self.queue.extend(manifests.depends_on);
        self.folders.insert(path.to_owned(), manifests.folders);
        Ok(())
    }
}

pub(crate) fn custom_archives(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut archives: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().is_some_and(|extension| extension == "zip")
        })
        .collect();
    archives.sort();
    archives
}

struct SourceFile {
    source: usize,
    entry: usize,
    size: u64,
}

enum Existing {
    File(u64),
    Directory,
}

fn compare(
    sources: &[PathBuf],
    superseded: &BTreeSet<(usize, String)>,
    target: &Path,
    exclusions: &Exclusions,
) -> Result<Vec<Action>, Error> {
    let mut archives = sources
        .iter()
        .map(|path| {
            File::open(path)
                .map_err(deps::Error::from)
                .and_then(|file| ZipArchive::new(file).map_err(deps::Error::from))
                .map_err(|error| Error::Archive {
                    path: path.clone(),
                    error,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut files: BTreeMap<String, SourceFile> = BTreeMap::new();
    let mut directories: BTreeSet<String> = BTreeSet::new();

    for (source, archive) in archives.iter_mut().enumerate() {
        for entry in 0..archive.len() {
            let file = archive.by_index(entry).map_err(|error| Error::Archive {
                path: sources[source].clone(),
                error: error.into(),
            })?;
            let Some(path) = deps::entry_path(&file) else {
                continue;
            };
            if exclusions.matches(&path) {
                continue;
            }
            if let Some((folder, _)) = path.split_once('/')
                && superseded.contains(&(source, folder.to_owned()))
            {
                continue;
            }

            if file.is_dir() {
                directories.insert(path);
            } else {
                files.insert(
                    path,
                    SourceFile {
                        source,
                        entry,
                        size: file.size(),
                    },
                );
            }
        }
    }

    let parents: Vec<String> = files
        .keys()
        .chain(directories.iter())
        .flat_map(|path| ancestors(path))
        .collect();
    directories.extend(parents);

    let existing = walk(target).map_err(|error| Error::Target(Arc::new(error)))?;
    let protected: HashSet<String> = existing
        .keys()
        .filter(|path| exclusions.matches(path))
        .flat_map(|path| std::iter::once(path.clone()).chain(ancestors(path)))
        .collect();

    let mut actions: Vec<Action> = existing
        .iter()
        .rev()
        .filter(|(path, _)| {
            !files.contains_key(*path) && !directories.contains(*path) && !protected.contains(*path)
        })
        .map(|(path, kind)| match kind {
            Existing::File(_) => Action::RemoveFile(path.clone()),
            Existing::Directory => Action::RemoveDirectory(path.clone()),
        })
        .collect();

    actions.extend(
        directories
            .iter()
            .filter(|path| !existing.contains_key(*path))
            .map(|path| Action::CreateDirectory(path.clone())),
    );

    for (path, file) in &files {
        let replace = match existing.get(path) {
            Some(Existing::File(size)) => {
                if *size == file.size
                    && !differs(&mut archives[file.source], file.entry, &target.join(path))
                        .map_err(|error| Error::Archive {
                            path: sources[file.source].clone(),
                            error,
                        })?
                {
                    continue;
                }
                true
            }
            _ => false,
        };

        actions.push(Action::WriteFile {
            path: path.clone(),
            source: file.source,
            entry: file.entry,
            replace,
        });
    }

    Ok(actions)
}

fn ancestors(path: &str) -> impl Iterator<Item = String> + '_ {
    path.trim_end_matches('/')
        .match_indices('/')
        .map(|(index, _)| path[..=index].to_owned())
}

fn walk(target: &Path) -> io::Result<BTreeMap<String, Existing>> {
    let mut existing = BTreeMap::new();
    let mut pending = vec![(target.to_owned(), String::new())];

    while let Some((directory, prefix)) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound && prefix.is_empty() => {
                return Ok(existing);
            }
            Err(error) => return Err(error),
        };

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let metadata = entry.path().symlink_metadata()?;

            if metadata.is_dir() {
                let path = format!("{prefix}{name}/");
                pending.push((entry.path(), path.clone()));
                existing.insert(path, Existing::Directory);
            } else {
                existing.insert(format!("{prefix}{name}"), Existing::File(metadata.len()));
            }
        }
    }

    Ok(existing)
}

fn differs(archive: &mut ZipArchive<File>, entry: usize, path: &Path) -> Result<bool, deps::Error> {
    let mut expected = Vec::new();
    archive.by_index(entry)?.read_to_end(&mut expected)?;

    Ok(fs::read(path)? != expected)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::Write;

    use md5::{Digest, Md5};
    use zip::write::SimpleFileOptions;

    use super::*;
    use crate::api::FileListEntry;
    use crate::config::Archive;

    struct Setup {
        _root: tempfile::TempDir,
        cache: PathBuf,
        custom: PathBuf,
        target: PathBuf,
    }

    impl Setup {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let setup = Self {
                cache: root.path().join(crate::cache::DIRECTORY),
                custom: root.path().join("custom"),
                target: root.path().join("target").join("AddOns"),
                _root: root,
            };
            fs::create_dir_all(&setup.target).unwrap();
            setup
        }

        fn input<'a>(&'a self, catalogue: &'a Catalogue, config: &'a Config) -> Input<'a> {
            Input {
                catalogue,
                config,
                cache: &self.cache,
                custom: &self.custom,
                target: &self.target,
            }
        }

        fn existing(&self, path: &str, content: &str) {
            let path = self.target.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
    }

    fn zip(files: &[(&str, &str)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(io::Cursor::new(Vec::new()));
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
        writer.finish().unwrap().into_inner()
    }

    fn entry(uid: &str, name: &str, directories: &[&str]) -> FileListEntry {
        FileListEntry {
            uid: uid.into(),
            name: name.into(),
            author: String::new(),
            version: "1".into(),
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

    fn md5_hex(bytes: &[u8]) -> String {
        Md5::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn download(bytes: &[u8]) -> cache::Download {
        cache::Download {
            bytes: bytes.to_vec(),
            archive: Archive {
                version: "1".to_owned(),
                md5: md5_hex(bytes),
            },
        }
    }

    fn installed(uids: &[&str]) -> Config {
        Config {
            addons: uids
                .iter()
                .map(|&uid| (uid.to_owned(), String::new()))
                .collect(),
            ..Config::default()
        }
    }

    fn server(archives: Vec<(&str, Vec<u8>)>) -> HashMap<String, Vec<u8>> {
        archives
            .into_iter()
            .map(|(uid, bytes)| (uid.to_owned(), bytes))
            .collect()
    }

    fn run(
        setup: &Setup,
        catalogue: &Catalogue,
        config: &Config,
        server: &HashMap<String, Vec<u8>>,
    ) -> (Plan, Vec<String>) {
        let fetched = RefCell::new(Vec::new());
        let plan = plan(
            &setup.input(catalogue, config),
            |uid, _| {
                fetched.borrow_mut().push(uid.to_owned());
                server.get(uid).map(|bytes| download(bytes)).ok_or_else(|| {
                    cache::Error::Request(crate::api::Error::NotFound(uid.to_owned()))
                })
            },
            |_| {},
        )
        .unwrap();
        (plan, fetched.into_inner())
    }

    fn written(plan: &Plan) -> Vec<(&str, usize, bool)> {
        plan.actions
            .iter()
            .filter_map(|action| match action {
                Action::WriteFile {
                    path,
                    source,
                    replace,
                    ..
                } => Some((path.as_str(), *source, *replace)),
                _ => None,
            })
            .collect()
    }

    fn removed(plan: &Plan) -> Vec<&str> {
        plan.actions
            .iter()
            .filter_map(|action| match action {
                Action::RemoveFile(path) | Action::RemoveDirectory(path) => Some(path.as_str()),
                _ => None,
            })
            .collect()
    }

    fn created(plan: &Plan) -> Vec<&str> {
        plan.actions
            .iter()
            .filter_map(|action| match action {
                Action::CreateDirectory(path) => Some(path.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn installs_addon_with_its_dependency() {
        let setup = Setup::new();
        let catalogue = Catalogue::new(
            [entry("1", "Addon", &["Addon"]), entry("2", "Lib", &["Lib"])],
            [],
        );
        let server = server(vec![
            (
                "1",
                zip(&[
                    ("Addon/", ""),
                    ("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib>=2\n"),
                ]),
            ),
            ("2", zip(&[("Lib/Lib.txt", "## Title: Lib\n")])),
        ]);

        let (plan, fetched) = run(&setup, &catalogue, &installed(&["1"]), &server);

        assert_eq!(fetched, ["1", "2"]);
        assert_eq!(
            plan.sources,
            [
                setup.cache.join("Addon_1.zip"),
                setup.cache.join("Lib_2.zip")
            ]
        );
        assert_eq!(created(&plan), ["Addon/", "Lib/"]);
        assert_eq!(
            written(&plan),
            [("Addon/Addon.txt", 0, false), ("Lib/Lib.txt", 1, false)]
        );
        assert_eq!(plan.archives.keys().collect::<Vec<_>>(), ["1", "2"]);
        assert!(plan.missing.is_empty() && plan.failed.is_empty());
    }

    #[test]
    fn valid_cache_is_not_downloaded_again() {
        let setup = Setup::new();
        let catalogue = Catalogue::new([entry("1", "Addon", &["Addon"])], []);
        let bytes = zip(&[("Addon/Addon.txt", "## Title: Addon\n")]);
        fs::create_dir_all(&setup.cache).unwrap();
        fs::write(setup.cache.join("Addon_1.zip"), &bytes).unwrap();
        let mut config = installed(&["1"]);
        config
            .archives
            .insert("1".to_owned(), download(&bytes).archive);

        let (plan, fetched) = run(&setup, &catalogue, &config, &HashMap::new());

        assert!(fetched.is_empty());
        assert!(plan.archives.is_empty());
        assert_eq!(written(&plan), [("Addon/Addon.txt", 0, false)]);
    }

    #[test]
    fn mirrors_the_target_folder() {
        let setup = Setup::new();
        let catalogue = Catalogue::new([entry("1", "Addon", &["Addon"])], []);
        let server = server(vec![(
            "1",
            zip(&[
                ("Addon/Addon.txt", "## Title: Addon\n"),
                ("Addon/Same.lua", "same"),
                ("Addon/Changed.lua", "new!"),
                ("Addon/Resized.lua", "longer"),
            ]),
        )]);
        setup.existing("Addon/Addon.txt", "## Title: Addon\n");
        setup.existing("Addon/Same.lua", "same");
        setup.existing("Addon/Changed.lua", "old!");
        setup.existing("Addon/Resized.lua", "short");
        setup.existing("Addon/Stale.lua", "");
        setup.existing("Old/Deep/File.lua", "");

        let (plan, _) = run(&setup, &catalogue, &installed(&["1"]), &server);

        assert_eq!(
            removed(&plan),
            ["Old/Deep/File.lua", "Old/Deep/", "Old/", "Addon/Stale.lua"]
        );
        assert!(created(&plan).is_empty());
        assert_eq!(
            written(&plan),
            [
                ("Addon/Changed.lua", 0, true),
                ("Addon/Resized.lua", 0, true)
            ]
        );
    }

    #[test]
    fn exclusions_protect_paths_and_their_folders() {
        let setup = Setup::new();
        let catalogue = Catalogue::new([entry("1", "Addon", &["Addon"])], []);
        let server = server(vec![(
            "1",
            zip(&[
                ("Addon/Addon.txt", "## Title: Addon\n"),
                ("Addon/Data.lua", "from archive"),
            ]),
        )]);
        setup.existing("Addon/Data.lua", "user data");
        setup.existing("Keep/Saved/Data.lua", "");
        setup.existing("Keep/Saved/Other.lua", "");
        setup.existing("Empty/Dynamic/File.lua", "");
        let config = Config {
            exclusions: vec![
                r"^Addon/Data\.lua$".to_owned(),
                r"^Keep/Saved/Data\.lua$".to_owned(),
                r"^Empty/Dynamic/$".to_owned(),
            ],
            ..installed(&["1"])
        };

        let (plan, _) = run(&setup, &catalogue, &config, &server);

        assert_eq!(
            removed(&plan),
            ["Keep/Saved/Other.lua", "Empty/Dynamic/File.lua"]
        );
        assert_eq!(written(&plan), [("Addon/Addon.txt", 0, false)]);
    }

    #[test]
    fn later_sources_win_and_custom_archives_come_last() {
        let setup = Setup::new();
        let catalogue = Catalogue::new(
            [entry("1", "Addon", &["Addon"]), entry("2", "Lib", &["Lib"])],
            [],
        );
        let server = server(vec![
            (
                "1",
                zip(&[
                    ("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n"),
                    ("Lib/Lib.txt", "## Title: Lib\n"),
                    ("Lib/Lib.lua", "bundled"),
                ]),
            ),
            (
                "2",
                zip(&[
                    ("Lib/Lib.txt", "## Title: Lib\n"),
                    ("Lib/Lib.lua", "standalone"),
                ]),
            ),
        ]);
        fs::create_dir_all(&setup.custom).unwrap();
        fs::write(
            setup.custom.join("Mine.zip"),
            zip(&[("Lib/Lib.lua", "custom")]),
        )
        .unwrap();
        fs::write(setup.custom.join("notes.txt"), "").unwrap();

        let (plan, fetched) = run(&setup, &catalogue, &installed(&["1"]), &server);

        assert_eq!(fetched, ["1", "2"]);
        assert_eq!(plan.sources.last(), Some(&setup.custom.join("Mine.zip")));
        assert_eq!(
            written(&plan),
            [
                ("Addon/Addon.txt", 0, false),
                ("Lib/Lib.lua", 2, false),
                ("Lib/Lib.txt", 1, false),
            ]
        );
    }

    #[test]
    fn download_failure_stops_before_comparing() {
        let setup = Setup::new();
        let catalogue = Catalogue::new([entry("1", "Addon", &["Addon"])], []);
        setup.existing("Other/File.lua", "");

        let (plan, _) = run(&setup, &catalogue, &installed(&["1"]), &HashMap::new());

        assert_eq!(plan.failed.len(), 1);
        assert_eq!(plan.failed[0].uid, "1");
        assert!(plan.actions.is_empty());
    }

    #[test]
    fn reports_unknown_addons_and_missing_dependencies() {
        let setup = Setup::new();
        let catalogue = Catalogue::new([entry("1", "Addon", &["Addon"])], []);
        let server = server(vec![(
            "1",
            zip(&[
                (
                    "Addon/Addon.txt",
                    "## Title: Addon\n## DependsOn: Nowhere Bundled\n",
                ),
                ("Addon/Libs/Bundled/Bundled.txt", "## Title: Bundled\n"),
            ]),
        )]);

        let (plan, _) = run(&setup, &catalogue, &installed(&["1", "99"]), &server);

        assert_eq!(plan.unknown, ["99"]);
        assert_eq!(plan.missing, ["Nowhere"]);
    }

    fn shared_library() -> (Catalogue, HashMap<String, Vec<u8>>) {
        let catalogue = Catalogue::new(
            [
                entry("1", "Addon", &["Addon"]),
                entry("2", "Big", &["Lib", "Extra"]),
                entry("3", "Small", &["Lib"]),
            ],
            [],
        );
        let server = server(vec![
            (
                "1",
                zip(&[("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n")]),
            ),
            (
                "2",
                zip(&[
                    ("Lib/Lib.txt", "## Title: Lib\n"),
                    ("Extra/Extra.txt", "## Title: Extra\n"),
                ]),
            ),
            ("3", zip(&[("Lib/Lib.txt", "## Title: Lib\n")])),
        ]);
        (catalogue, server)
    }

    #[test]
    fn asks_instead_of_guessing_the_provider() {
        let setup = Setup::new();
        let (catalogue, server) = shared_library();
        setup.existing("Stale.lua", "");

        let (plan, fetched) = run(&setup, &catalogue, &installed(&["1"]), &server);

        assert_eq!(fetched, ["1"]);
        assert_eq!(
            plan.choices,
            BTreeMap::from([("Lib".to_owned(), vec!["2".to_owned(), "3".to_owned()])])
        );
        assert!(!plan.is_applicable());
        assert!(plan.actions.is_empty());

        let mut config = installed(&["1"]);
        super::super::record(&plan, &catalogue, &mut config);
        assert!(config.archives.is_empty(), "nothing was applied");
    }

    #[test]
    fn asks_even_if_a_candidate_is_recorded_or_bundled() {
        let setup = Setup::new();
        let (catalogue, mut server) = shared_library();
        server.insert(
            "1".to_owned(),
            zip(&[
                ("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n"),
                ("Addon/Lib/Lib.txt", "## Title: Lib\n"),
            ]),
        );
        let config = Config {
            archives: BTreeMap::from([("2".to_owned(), Archive::default())]),
            ..installed(&["1"])
        };

        let (plan, _) = run(&setup, &catalogue, &config, &server);

        assert_eq!(
            plan.choices.keys().collect::<Vec<_>>(),
            ["Lib"],
            "a recorded archive or a bundled copy does not decide"
        );
    }

    #[test]
    fn the_only_installed_candidate_decides_and_drops_the_selection() {
        let setup = Setup::new();
        let (catalogue, server) = shared_library();
        let mut config = Config {
            selected_libraries: BTreeMap::from([("Lib".to_owned(), "2".to_owned())]),
            ..installed(&["1", "3"])
        };

        let (plan, fetched) = run(&setup, &catalogue, &config, &server);

        assert_eq!(fetched, ["1", "3"]);
        assert!(plan.choices.is_empty());

        super::super::record(&plan, &catalogue, &mut config);
        super::super::clean_config(&plan, &catalogue, &mut config);
        assert!(config.selected_libraries.is_empty());
    }

    #[test]
    fn installs_the_selected_library_next_to_an_addon_that_bundles_it() {
        let setup = Setup::new();
        let (catalogue, mut server) = shared_library();
        server.insert(
            "1".to_owned(),
            zip(&[
                ("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n"),
                ("Addon/Lib/Lib.txt", "## Title: Lib\n"),
            ]),
        );
        let config = Config {
            selected_libraries: BTreeMap::from([("Lib".to_owned(), "3".to_owned())]),
            ..installed(&["1"])
        };

        let (plan, fetched) = run(&setup, &catalogue, &config, &server);

        assert_eq!(fetched, ["1", "3"], "the bundled copy does not replace it");
        assert!(plan.choices.is_empty());
    }

    fn bundling_and_separate() -> (Catalogue, HashMap<String, Vec<u8>>) {
        let catalogue = Catalogue::new(
            [
                entry("2", "Big", &["Lib", "Extra"]),
                entry("3", "Small", &["Lib"]),
            ],
            [],
        );
        let server = server(vec![
            (
                "2",
                zip(&[
                    ("Lib/Lib.txt", "## Title: Lib\nbundled"),
                    ("Lib/Old.lua", "old"),
                    ("Extra/Extra.txt", "## Title: Extra\n"),
                ]),
            ),
            ("3", zip(&[("Lib/Lib.txt", "## Title: Lib\nseparate")])),
        ]);
        (catalogue, server)
    }

    #[test]
    fn takes_a_shared_folder_only_from_the_selected_archive() {
        let setup = Setup::new();
        let (catalogue, server) = bundling_and_separate();
        setup.existing("Lib/Old.lua", "old");

        for (selected, lib, old) in [
            ("3", "## Title: Lib\nseparate", false),
            ("2", "## Title: Lib\nbundled", true),
        ] {
            let config = Config {
                selected_libraries: BTreeMap::from([("Lib".to_owned(), selected.to_owned())]),
                ..installed(&["2", "3"])
            };

            let (plan, _) = run(&setup, &catalogue, &config, &server);
            assert!(plan.choices.is_empty());
            super::super::apply(&plan, &setup.target, |_| {}).unwrap();

            assert_eq!(
                fs::read_to_string(setup.target.join("Lib/Lib.txt")).unwrap(),
                lib
            );
            assert_eq!(
                setup.target.join("Lib/Old.lua").exists(),
                old,
                "no mixed copies"
            );
            assert!(setup.target.join("Extra/Extra.txt").is_file());
        }
    }

    #[test]
    fn asks_which_archive_gives_a_shared_folder() {
        let setup = Setup::new();
        let (catalogue, server) = bundling_and_separate();

        let (plan, _) = run(&setup, &catalogue, &installed(&["2", "3"]), &server);

        assert_eq!(plan.choices.keys().collect::<Vec<_>>(), ["Lib"]);
        assert!(!plan.is_applicable());
    }

    #[test]
    fn keeps_the_selection_for_a_shared_folder() {
        let setup = Setup::new();
        let (catalogue, server) = bundling_and_separate();
        let mut config = Config {
            selected_libraries: BTreeMap::from([("Lib".to_owned(), "3".to_owned())]),
            ..installed(&["2", "3"])
        };

        let (plan, _) = run(&setup, &catalogue, &config, &server);
        super::super::clean_config(&plan, &catalogue, &mut config);

        assert_eq!(
            config.selected_libraries,
            BTreeMap::from([("Lib".to_owned(), "3".to_owned())])
        );
    }

    #[test]
    fn keeps_entries_inside_the_target() {
        let setup = Setup::new();
        let catalogue = Catalogue::new([entry("1", "Addon", &["Addon"])], []);
        let server = server(vec![(
            "1",
            zip(&[
                ("Addon/Addon.txt", "## Title: Addon\n"),
                ("../Escape.lua", ""),
                ("/Absolute.lua", ""),
            ]),
        )]);

        let (plan, _) = run(&setup, &catalogue, &installed(&["1"]), &server);

        assert_eq!(
            written(&plan),
            [("Absolute.lua", 0, false), ("Addon/Addon.txt", 0, false)]
        );
    }

    #[test]
    fn applied_plan_leaves_nothing_to_do() {
        let setup = Setup::new();
        let catalogue = Catalogue::new(
            [entry("1", "Addon", &["Addon"]), entry("2", "Lib", &["Lib"])],
            [],
        );
        let server = server(vec![
            (
                "1",
                zip(&[
                    ("Addon/", ""),
                    ("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n"),
                    ("Addon/Sub/Code.lua", "code"),
                ]),
            ),
            ("2", zip(&[("Lib/Lib.txt", "## Title: Lib\n")])),
        ]);
        setup.existing("Addon/Sub/Code.lua", "old code");
        setup.existing("Stale/Deep/File.lua", "");
        let mut config = installed(&["1"]);

        let (plan, _) = run(&setup, &catalogue, &config, &server);
        let mut events = Vec::new();
        super::super::apply(&plan, &setup.target, |event| events.push(event)).unwrap();

        assert_eq!(
            fs::read_to_string(setup.target.join("Addon/Sub/Code.lua")).unwrap(),
            "code"
        );
        assert!(setup.target.join("Lib/Lib.txt").is_file());
        assert!(!setup.target.join("Stale").exists());
        assert!(matches!(
            events.last(),
            Some(Event::Applying { done, total }) if done == total
        ));

        super::super::record(&plan, &catalogue, &mut config);
        let (again, fetched) = run(&setup, &catalogue, &config, &HashMap::new());

        assert!(fetched.is_empty());
        assert!(again.actions.is_empty());
    }

    #[test]
    fn cleans_cache_and_config_after_sync() {
        let setup = Setup::new();
        let catalogue = Catalogue::new(
            [
                entry("1", "Addon", &["Addon"]),
                entry("2", "Big", &["Lib", "Extra"]),
                entry("3", "Small", &["Lib"]),
                entry("4", "Other", &["Other"]),
            ],
            [],
        );
        let server = server(vec![
            (
                "1",
                zip(&[("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n")]),
            ),
            ("3", zip(&[("Lib/Lib.txt", "## Title: Lib\n")])),
        ]);
        fs::create_dir_all(&setup.cache).unwrap();
        fs::write(setup.cache.join("Old_4.zip"), "").unwrap();
        fs::write(setup.cache.join("Addon_1.zip.tmp"), "").unwrap();
        let mut config = Config {
            favourites: [
                ("4".to_owned(), String::new()),
                ("77".to_owned(), String::new()),
            ]
            .into(),
            selected_libraries: [
                ("Unused".to_owned(), "4".to_owned()),
                ("Lib".to_owned(), "3".to_owned()),
            ]
            .into(),
            archives: [("4".to_owned(), Archive::default())].into(),
            ..installed(&["1", "99"])
        };

        let (plan, _) = run(&setup, &catalogue, &config, &server);
        super::super::record(&plan, &catalogue, &mut config);
        super::super::clean_config(&plan, &catalogue, &mut config);
        super::super::clean_cache(&plan, &setup.cache).unwrap();

        assert_eq!(
            config.addons,
            BTreeMap::from([("1".to_owned(), "Addon".to_owned())])
        );
        assert_eq!(config.favourites.keys().collect::<Vec<_>>(), ["4"]);
        assert_eq!(config.archives.keys().collect::<Vec<_>>(), ["1", "3"]);
        assert_eq!(
            config.selected_libraries,
            BTreeMap::from([("Lib".to_owned(), "3".to_owned())])
        );

        let mut cached: Vec<String> = fs::read_dir(&setup.cache)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        cached.sort();
        assert_eq!(cached, ["Addon_1.zip", "Small_3.zip"]);
    }
}
