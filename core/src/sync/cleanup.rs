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

pub fn clean_config(plan: &Plan, catalogue: &Catalogue, config: &mut Config) {
    let known: HashSet<&str> = catalogue
        .addons()
        .iter()
        .map(|addon| addon.uid.as_str())
        .collect();

    config.addons.retain(|uid, _| known.contains(uid.as_str()));
    config
        .favourites
        .retain(|uid, _| known.contains(uid.as_str()));

    config
        .selected_libraries
        .retain(|directory, _| plan.selected.contains(directory));
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
