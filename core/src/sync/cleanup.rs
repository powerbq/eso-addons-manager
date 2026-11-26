use std::collections::HashSet;
use std::io;
use std::path::Path;

use super::Plan;
use crate::cache::{self, Index};
use crate::catalogue::Catalogue;
use crate::config::Config;

pub fn record(catalogue: &Catalogue, config: &mut Config) {
    for addon in catalogue.addons() {
        if let Some(name) = config.addons.get_mut(&addon.uid) {
            name.clone_from(&addon.name);
        }
    }
}

pub fn record_archives(plan: &Plan, index: &mut Index) {
    if plan.is_applicable() {
        index.extend(plan.archives.clone());
    }
}

/// What `clean_config` cut automatically (not at the user's request) from the installed set and
/// favourites, so a caller can snapshot the settings file before it is overwritten.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pruned {
    pub addons: Vec<String>,
    pub favourites: Vec<String>,
}

impl Pruned {
    pub fn is_empty(&self) -> bool {
        self.addons.is_empty() && self.favourites.is_empty()
    }
}

pub fn clean_config(plan: &Plan, catalogue: &Catalogue, config: &mut Config) -> Pruned {
    let known: HashSet<&str> = catalogue
        .addons()
        .iter()
        .map(|addon| addon.uid.as_str())
        .collect();

    let unknown = |map: &std::collections::BTreeMap<String, String>| {
        map.keys()
            .filter(|uid| !known.contains(uid.as_str()))
            .cloned()
            .collect()
    };
    let pruned = Pruned {
        addons: unknown(&config.addons),
        favourites: unknown(&config.favourites),
    };

    config.addons.retain(|uid, _| known.contains(uid.as_str()));
    config
        .favourites
        .retain(|uid, _| known.contains(uid.as_str()));

    config
        .selected_libraries
        .retain(|directory, _| plan.selected.contains(directory));

    pruned
}

pub fn clean_archives(plan: &Plan, index: &mut Index) {
    index.retain(|uid, _| plan.used.contains(uid));
}

pub fn clean_cache(plan: &Plan, cache: &Path) -> io::Result<()> {
    crate::clean::retain(cache, |path| {
        path.file_name().and_then(|name| name.to_str()) == Some(cache::INDEX_FILE)
            || plan.sources.iter().any(|source| source.path == path)
    })
}
