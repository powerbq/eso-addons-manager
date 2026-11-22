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
    pub index: &'a cache::Index,
    pub cache: &'a Path,
    pub custom: &'a Path,
    pub target: &'a Path,
}

/// A zip fed into the sync. `prefix` is prepended to every entry path before it is mirrored, so a
/// source whose archive is laid out at its own root can still be installed into a subfolder of the
/// AddOns directory (the TTC price archive goes under `TamrielTradeCentre/`, for example). Addon
/// and custom sources use an empty prefix (installed at the AddOns root).
#[derive(Debug, Clone)]
pub struct Source {
    pub path: PathBuf,
    pub prefix: String,
}

impl Source {
    fn root(path: PathBuf) -> Self {
        Self {
            path,
            prefix: String::new(),
        }
    }

    pub fn prefixed(path: PathBuf, prefix: impl Into<String>) -> Self {
        Self {
            path,
            prefix: prefix.into(),
        }
    }
}

pub fn plan(
    input: &Input<'_>,
    fetch: impl FnMut(&str, &Path) -> Result<cache::Download, cache::Error>,
    fetch_links: impl FnMut(&str) -> Option<Vec<u32>>,
    fetch_prices: impl FnOnce(crate::ttc::Region) -> Option<(PathBuf, bool)>,
    mut on_event: impl FnMut(Event),
) -> Result<Plan, Error> {
    let mut resolver = Resolver {
        input,
        fetch,
        fetch_links,
        on_event: &mut on_event,
        config: input.config.clone(),
        archives: input.index.clone(),
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
        required_by: HashMap::new(),
        links: HashMap::new(),
        inspected_provides: HashMap::new(),
    };
    let mut plan = resolver.resolve()?;

    if plan.is_applicable() {
        // When TTC price syncing is on and TTC is installed (directly or as a dependency), the price
        // archive joins the plan as a source under the TTC folder, so the sync writes/refreshes the
        // price files and drops the wrong region's table as an extraneous file — just like an addon.
        let ttc_managed = if input.config.update_prices && plan.used.contains(crate::ttc::ADDON_UID)
        {
            match fetch_prices(input.config.price_region) {
                Some((path, downloaded)) => {
                    let prefix = format!("{}/", crate::ttc::ADDON_DIRECTORY);
                    plan.sources.push(Source::prefixed(path, prefix));
                    plan.ttc = Some(if downloaded {
                        crate::ttc::Outcome::Updated
                    } else {
                        crate::ttc::Outcome::UpToDate
                    });
                    true
                }
                None => {
                    plan.ttc = Some(crate::ttc::Outcome::Failed(
                        "Tamriel Trade Centre is unreachable and no cached prices are available"
                            .to_owned(),
                    ));
                    false
                }
            }
        } else {
            false
        };

        // Built-in exclusions key off the addons actually installed — directly or pulled in as a
        // dependency — so TTC/HarvestMap generated files are protected even when the addon only
        // comes in through another addon's `DependsOn`. When the sync manages the TTC price files
        // itself (`ttc_managed`), their exclusions are dropped: they only guard client-generated
        // files while the manager is not managing prices.
        let exclusions = Exclusions::new(input.config, &plan.used, ttc_managed)
            .map_err(|error| Error::Exclusion(Arc::new(error)))?;
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

struct Resolver<'a, F, L, E> {
    input: &'a Input<'a>,
    fetch: F,
    fetch_links: L,
    on_event: &'a mut E,
    config: Config,
    archives: cache::Index,
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
    required_by: HashMap<String, Vec<String>>,
    links: HashMap<String, Vec<u32>>,
    inspected_provides: HashMap<String, Vec<String>>,
}

enum Resolution<'a> {
    Use { addon: &'a Addon, save: bool },
    Ask(Vec<&'a Addon>),
}

impl<F, L, E> Resolver<'_, F, L, E>
where
    F: FnMut(&str, &Path) -> Result<cache::Download, cache::Error>,
    L: FnMut(&str) -> Option<Vec<u32>>,
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
            self.read(path, None)?;
        }

        loop {
            self.drain()?;
            if !self.settle_shared_folders()? {
                break;
            }
        }

        self.plan
            .sources
            .extend(custom.into_iter().map(Source::root));
        self.plan.missing = self
            .unresolved
            .iter()
            .filter(|directory| !self.provided.contains(*directory))
            .cloned()
            .collect();
        self.plan.dependents = self.dependents();

        Ok(std::mem::take(&mut self.plan))
    }

    // For every installed addon, the addons that pull it in: the dependers of any folder it
    // provides. Used by the UI to explain why an addon is installed automatically.
    fn dependents(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut dependents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for uid in &self.plan.used {
            let Some(&source) = self.source_of.get(uid) else {
                continue;
            };
            let Some(folders) = self.folders.get(&self.plan.sources[source].path) else {
                continue;
            };
            let dependers: BTreeSet<String> = folders
                .iter()
                .filter_map(|folder| self.required_by.get(folder))
                .flatten()
                .filter(|depender| *depender != uid)
                .cloned()
                .collect();
            if !dependers.is_empty() {
                dependents.insert(uid.clone(), dependers);
            }
        }
        dependents
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
            match self.decide(&directory, &candidates) {
                Resolution::Use { addon, save } => {
                    if save {
                        self.plan.selected.insert(directory);
                    }
                    self.process(addon)?;
                }
                Resolution::Ask(pruned) => self.ask(directory, &pruned),
            }
        }
        Ok(())
    }

    fn settle_shared_folders(&mut self) -> Result<bool, Error> {
        let mut holders: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, source) in self.plan.sources.iter().enumerate() {
            for folder in self.folders.get(&source.path).into_iter().flatten() {
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
            let (provider, save) = match self.decide(&folder, &candidates) {
                Resolution::Use { addon, save } => (addon, save),
                Resolution::Ask(pruned) => {
                    self.ask(folder, &pruned);
                    continue;
                }
            };
            if !self.processed.contains(&provider.uid) {
                self.process(provider)?;
                processed_more = true;
                continue;
            }
            if let Some(&owner) = self.source_of.get(&provider.uid)
                && holders.contains(&owner)
            {
                self.keep(&folder, &holders, owner, save);
            }
        }
        Ok(processed_more)
    }

    fn keep(&mut self, folder: &str, holders: &[usize], owner: usize, save: bool) {
        if save {
            self.plan.selected.insert(folder.to_owned());
        }
        for &holder in holders.iter().filter(|holder| **holder != owner) {
            self.plan.superseded.insert((holder, folder.to_owned()));
        }
    }

    fn decide<'c>(&mut self, folder: &str, candidates: &[&'c Addon]) -> Resolution<'c> {
        // Candidates recommended by a depending addon's description links; recorded for the
        // UI (the choice dialog and the Libraries tab mark them) and used for the auto-pick.
        let recommended = if candidates.len() > 1 {
            self.recommended(folder, candidates)
        } else {
            Vec::new()
        };
        if !recommended.is_empty() {
            self.plan
                .recommended
                .insert(folder.to_owned(), recommended.clone());
        }

        // A single/only-installed provider decides; a saved user selection wins and is kept
        // — the manual escape hatch, never overridden or cleared by the heuristics.
        if let Some(decision) = deps::provider(folder, candidates, &self.config) {
            let save = matches!(decision, deps::Decision::Selected(_));
            return Resolution::Use {
                addon: decision.addon(),
                save,
            };
        }

        // Cheap: an unambiguous description link from a depending addon.
        if let [uid] = recommended.as_slice()
            && let Some(addon) = candidates.iter().copied().find(|addon| &addon.uid == uid)
        {
            tracing::info!("provider of {folder} chosen from a description link: UID {uid}");
            return Resolution::Use { addon, save: false };
        }

        // Expensive: drop candidates that do not really provide the folder — a language
        // patch or file dump that contains the folder without a manifest declaring it.
        let real = self.real_providers(folder, candidates);
        let pool = if real.is_empty() {
            candidates.to_vec()
        } else {
            real
        };
        if pool.len() == 1 {
            return Resolution::Use {
                addon: pool[0],
                save: false,
            };
        }

        Resolution::Ask(pool)
    }

    fn recommended(&mut self, folder: &str, candidates: &[&Addon]) -> Vec<String> {
        let Some(dependers) = self.required_by.get(folder).cloned() else {
            return Vec::new();
        };
        let candidate_uids: HashSet<&str> =
            candidates.iter().map(|addon| addon.uid.as_str()).collect();

        let mut recommended: BTreeSet<String> = BTreeSet::new();
        for depender in dependers {
            for uid in self.links_of(&depender) {
                let uid = uid.to_string();
                if candidate_uids.contains(uid.as_str()) {
                    recommended.insert(uid);
                }
            }
        }

        recommended.into_iter().collect()
    }

    fn real_providers<'c>(&mut self, folder: &str, candidates: &[&'c Addon]) -> Vec<&'c Addon> {
        candidates
            .iter()
            .copied()
            .filter(|addon| self.provides_folder(addon, folder) != Some(false))
            .collect()
    }

    fn provides_folder(&mut self, addon: &Addon, folder: &str) -> Option<bool> {
        if let Some(provides) = self.inspected_provides.get(&addon.uid) {
            return Some(provides.iter().any(|provided| provided == folder));
        }

        let path = self.inspect(addon)?;
        let provides = deps::read_archive(&path)
            .map(|manifests| manifests.provides)
            .unwrap_or_default();
        let result = provides.iter().any(|provided| provided == folder);
        self.inspected_provides.insert(addon.uid.clone(), provides);
        Some(result)
    }

    fn inspect(&mut self, addon: &Addon) -> Option<PathBuf> {
        let path = cache::archive_path(self.input.cache, addon);

        // A downloaded inspection archive stays in the cache while the conflict is unresolved
        // (an unresolved plan is non-applicable, so the cache is never swept); a re-inspection is
        // then a cheap MD5 cache-hit. Once the conflict resolves, the applicable sync's cleanup
        // removes the losing candidates like any other non-source.
        if cache::is_valid(&path, addon, self.archives.get(&addon.uid)) {
            return Some(path);
        }

        (self.on_event)(Event::Downloading {
            name: addon.name.clone(),
        });

        match (self.fetch)(&addon.uid, &path).and_then(|download| cache::store(&path, &download)) {
            Ok(()) => Some(path),
            Err(error) => {
                tracing::debug!("cannot inspect provider {}: {error}", addon.uid);
                None
            }
        }
    }

    fn links_of(&mut self, uid: &str) -> Vec<u32> {
        if let Some(links) = self.links.get(uid) {
            return links.clone();
        }

        let links = (self.fetch_links)(uid).unwrap_or_default();
        self.links.insert(uid.to_owned(), links.clone());
        links
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
            self.read(&path, Some(addon.uid.as_str()))?;
            self.source_of
                .insert(addon.uid.clone(), self.plan.sources.len());
            self.plan.sources.push(Source::root(path));
            self.plan.used.insert(addon.uid.clone());
        }
        Ok(())
    }

    fn obtain(&mut self, addon: &Addon) -> Option<PathBuf> {
        let path = cache::archive_path(self.input.cache, addon);
        if cache::is_valid(&path, addon, self.archives.get(&addon.uid)) {
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
                self.archives.insert(addon.uid.clone(), archive.clone());
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

    fn read(&mut self, path: &Path, depender: Option<&str>) -> Result<(), Error> {
        let manifests = deps::read_archive(path).map_err(|error| Error::Archive {
            path: path.to_owned(),
            error,
        })?;

        self.provided.extend(manifests.provides);
        if let Some(uid) = depender {
            for folder in &manifests.depends_on {
                self.required_by
                    .entry(folder.clone())
                    .or_default()
                    .push(uid.to_owned());
            }
        }
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
    sources: &[Source],
    superseded: &BTreeSet<(usize, String)>,
    target: &Path,
    exclusions: &Exclusions,
) -> Result<Vec<Action>, Error> {
    let mut archives = sources
        .iter()
        .map(|source| {
            File::open(&source.path)
                .map_err(deps::Error::from)
                .and_then(|file| ZipArchive::new(file).map_err(deps::Error::from))
                .map_err(|error| Error::Archive {
                    path: source.path.clone(),
                    error,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut files: BTreeMap<String, SourceFile> = BTreeMap::new();
    let mut directories: BTreeSet<String> = BTreeSet::new();

    for (source, archive) in archives.iter_mut().enumerate() {
        let prefix = sources[source].prefix.as_str();
        for entry in 0..archive.len() {
            let file = archive.by_index(entry).map_err(|error| Error::Archive {
                path: sources[source].path.clone(),
                error: error.into(),
            })?;
            let Some(entry_path) = deps::entry_path(&file) else {
                continue;
            };
            let path = if prefix.is_empty() {
                entry_path
            } else {
                format!("{prefix}{entry_path}")
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
                            path: sources[file.source].path.clone(),
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
    use crate::cache::{Archive, Index};

    struct Setup {
        _root: tempfile::TempDir,
        cache: PathBuf,
        custom: PathBuf,
        target: PathBuf,
        index: Index,
    }

    impl Setup {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let setup = Self {
                cache: root.path().join(crate::cache::DIRECTORY),
                custom: root.path().join("custom"),
                target: root.path().join("target").join("AddOns"),
                index: Index::default(),
                _root: root,
            };
            fs::create_dir_all(&setup.target).unwrap();
            setup
        }

        fn input<'a>(&'a self, catalogue: &'a Catalogue, config: &'a Config) -> Input<'a> {
            Input {
                catalogue,
                config,
                index: &self.index,
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
        run_with_links(setup, catalogue, config, server, &HashMap::new())
    }

    fn run_with_prices(
        setup: &Setup,
        catalogue: &Catalogue,
        config: &Config,
        server: &HashMap<String, Vec<u8>>,
        prices: Option<(PathBuf, bool)>,
    ) -> Plan {
        let fetched = RefCell::new(Vec::new());
        plan(
            &setup.input(catalogue, config),
            |uid, _| {
                fetched.borrow_mut().push(uid.to_owned());
                server.get(uid).map(|bytes| download(bytes)).ok_or_else(|| {
                    cache::Error::Request(crate::api::Error::NotFound(uid.to_owned()))
                })
            },
            |_| None,
            |_| prices.clone(),
            |_| {},
        )
        .unwrap()
    }

    fn run_with_links(
        setup: &Setup,
        catalogue: &Catalogue,
        config: &Config,
        server: &HashMap<String, Vec<u8>>,
        links: &HashMap<&str, Vec<u32>>,
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
            |uid| links.get(uid).cloned(),
            |_| None,
            |_| {},
        )
        .unwrap();
        (plan, fetched.into_inner())
    }

    fn source_paths(plan: &Plan) -> Vec<PathBuf> {
        plan.sources
            .iter()
            .map(|source| source.path.clone())
            .collect()
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
            source_paths(&plan),
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
        let mut setup = Setup::new();
        let catalogue = Catalogue::new([entry("1", "Addon", &["Addon"])], []);
        let bytes = zip(&[("Addon/Addon.txt", "## Title: Addon\n")]);
        fs::create_dir_all(&setup.cache).unwrap();
        fs::write(setup.cache.join("Addon_1.zip"), &bytes).unwrap();
        setup.index.insert("1".to_owned(), download(&bytes).archive);

        let (plan, fetched) = run(&setup, &catalogue, &installed(&["1"]), &HashMap::new());

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
        assert_eq!(
            plan.sources.last().map(|source| &source.path),
            Some(&setup.custom.join("Mine.zip"))
        );
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

        // both candidates really provide the folder, so inspection prunes nothing and we ask.
        assert_eq!(fetched, ["1", "2", "3"]);
        assert_eq!(
            plan.choices,
            BTreeMap::from([("Lib".to_owned(), vec!["2".to_owned(), "3".to_owned()])])
        );
        assert!(!plan.is_applicable());
        assert!(plan.actions.is_empty());

        // nothing was applied, so nothing is recorded (the inspected zips stay in the cache only
        // because an unresolved plan is non-applicable and the cache is not swept).
        let mut index = Index::default();
        super::super::record_archives(&plan, &mut index);
        assert!(index.is_empty());
    }

    #[test]
    fn chooses_the_provider_from_a_description_link() {
        let setup = Setup::new();
        let (catalogue, server) = shared_library();
        let links = HashMap::from([("1", vec![3])]);

        let (plan, _) = run_with_links(&setup, &catalogue, &installed(&["1"]), &server, &links);

        assert!(plan.choices.is_empty());
        assert!(plan.is_applicable());
        assert!(plan.used.contains("3"));
        assert!(!plan.used.contains("2"));
        assert!(
            plan.selected.is_empty(),
            "an auto-choice is not saved as a selection"
        );
    }

    #[test]
    fn ignores_a_link_that_is_not_a_candidate() {
        let setup = Setup::new();
        let (catalogue, server) = shared_library();
        let links = HashMap::from([("1", vec![999])]);

        let (plan, _) = run_with_links(&setup, &catalogue, &installed(&["1"]), &server, &links);

        assert_eq!(
            plan.choices,
            BTreeMap::from([("Lib".to_owned(), vec!["2".to_owned(), "3".to_owned()])])
        );
    }

    #[test]
    fn asks_when_a_link_matches_several_candidates() {
        let setup = Setup::new();
        let (catalogue, server) = shared_library();
        let links = HashMap::from([("1", vec![2, 3])]);

        let (plan, _) = run_with_links(&setup, &catalogue, &installed(&["1"]), &server, &links);

        assert!(!plan.is_applicable());
        assert!(plan.choices.contains_key("Lib"));
    }

    #[test]
    fn a_saved_selection_wins_and_is_kept_even_against_a_link() {
        let setup = Setup::new();
        let (catalogue, server) = shared_library();
        let mut config = installed(&["1"]);
        config.selected_libraries = BTreeMap::from([("Lib".to_owned(), "3".to_owned())]);
        // the link points elsewhere, but the user's manual choice is the escape hatch and wins.
        let links = HashMap::from([("1", vec![2])]);

        let (plan, _) = run_with_links(&setup, &catalogue, &config, &server, &links);

        assert!(plan.used.contains("3"));
        assert!(!plan.used.contains("2"));
        assert!(plan.selected.contains("Lib"));

        super::super::clean_config(&plan, &catalogue, &mut config);
        assert_eq!(
            config.selected_libraries.get("Lib").map(String::as_str),
            Some("3")
        );
    }

    fn patched_library() -> (Catalogue, HashMap<String, Vec<u8>>) {
        let catalogue = Catalogue::new(
            [
                entry("1", "Addon", &["Addon"]),
                entry("2", "Lib", &["Lib"]),
                entry("3", "Lib Language Patch", &["Lib"]),
            ],
            [],
        );
        let server = server(vec![
            (
                "1",
                zip(&[("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n")]),
            ),
            ("2", zip(&[("Lib/Lib.txt", "## Title: Lib\n")])),
            ("3", zip(&[("Lib/Lang.lua", "-- translation only\n")])),
        ]);
        (catalogue, server)
    }

    #[test]
    fn prunes_a_candidate_without_a_manifest() {
        let setup = Setup::new();
        let (catalogue, server) = patched_library();

        let (plan, _) = run(&setup, &catalogue, &installed(&["1"]), &server);

        assert!(
            plan.choices.is_empty(),
            "the patch is pruned, leaving one real provider"
        );
        assert!(plan.is_applicable());
        assert!(plan.used.contains("2"));
        assert!(!plan.used.contains("3"));
    }

    #[test]
    fn asks_even_if_a_candidate_is_recorded_or_bundled() {
        let mut setup = Setup::new();
        setup.index = Index::from(BTreeMap::from([("2".to_owned(), Archive::default())]));
        let (catalogue, mut server) = shared_library();
        server.insert(
            "1".to_owned(),
            zip(&[
                ("Addon/Addon.txt", "## Title: Addon\n## DependsOn: Lib\n"),
                ("Addon/Lib/Lib.txt", "## Title: Lib\n"),
            ]),
        );

        let (plan, _) = run(&setup, &catalogue, &installed(&["1"]), &server);

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

        super::super::record(&catalogue, &mut config);
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
        let mut setup = Setup::new();
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
        let config = installed(&["1"]);

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

        super::super::record_archives(&plan, &mut setup.index);
        let (again, fetched) = run(&setup, &catalogue, &config, &HashMap::new());

        assert!(fetched.is_empty());
        assert!(again.actions.is_empty());
    }

    #[test]
    fn cleans_cache_and_config_after_sync() {
        let mut setup = Setup::new();
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
        setup.index = Index::from(BTreeMap::from([("4".to_owned(), Archive::default())]));
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
            ..installed(&["1", "99"])
        };

        let (plan, _) = run(&setup, &catalogue, &config, &server);
        super::super::record(&catalogue, &mut config);
        super::super::clean_config(&plan, &catalogue, &mut config);
        super::super::record_archives(&plan, &mut setup.index);
        super::super::clean_archives(&plan, &mut setup.index);
        super::super::clean_cache(&plan, &setup.cache).unwrap();

        assert_eq!(
            config.addons,
            BTreeMap::from([("1".to_owned(), "Addon".to_owned())])
        );
        assert_eq!(config.favourites.keys().collect::<Vec<_>>(), ["4"]);
        assert_eq!(setup.index.keys().collect::<Vec<_>>(), ["1", "3"]);
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

    #[test]
    fn ttc_prices_are_synced_as_a_prefixed_source() {
        let setup = Setup::new();
        let catalogue = Catalogue::new([entry("1245", "TTC", &["TamrielTradeCentre"])], []);
        let server = server(vec![(
            "1245",
            zip(&[(
                "TamrielTradeCentre/TamrielTradeCentre.txt",
                "## Title: TTC\n",
            )]),
        )]);

        // A price archive on disk, laid out at its own root like the real TTC download.
        fs::create_dir_all(&setup.cache).unwrap();
        let prices = setup.cache.join("PriceTableEU.zip");
        fs::write(
            &prices,
            zip(&[("PriceTableEU.lua", "--Version = 1\nprices")]),
        )
        .unwrap();

        // The addon is installed, but the price table is missing and a stale NA table lingers.
        setup.existing(
            "TamrielTradeCentre/TamrielTradeCentre.txt",
            "## Title: TTC\n",
        );
        setup.existing("TamrielTradeCentre/PriceTableNA.lua", "old NA");

        let config = Config {
            update_prices: true,
            ..installed(&["1245"])
        };

        let plan = run_with_prices(&setup, &catalogue, &config, &server, Some((prices, true)));

        // The price table is written under the TTC folder from the prefixed source.
        assert!(
            written(&plan)
                .iter()
                .any(|(path, _, _)| *path == "TamrielTradeCentre/PriceTableEU.lua"),
            "price table is installed"
        );
        // The wrong region's table is gone as an extraneous file (TTC exclusions dropped).
        assert!(
            removed(&plan).contains(&"TamrielTradeCentre/PriceTableNA.lua"),
            "the stale NA table is removed"
        );
        assert!(matches!(plan.ttc, Some(crate::ttc::Outcome::Updated)));
    }
}
