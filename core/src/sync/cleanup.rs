use std::collections::HashSet;
use std::io;
use std::path::Path;

use super::Plan;
use crate::catalogue::Catalogue;
use crate::config::Config;

pub fn record(plan: &Plan, catalogue: &Catalogue, config: &mut Config) {
    if plan.is_applicable() {
        config.archives.extend(plan.archives.clone());
    }

    for addon in catalogue.addons() {
        if let Some(name) = config.addons.get_mut(&addon.uid) {
            name.clone_from(&addon.name);
        }
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
    config.archives.retain(|uid, _| plan.used.contains(uid));

    config
        .selected_libraries
        .retain(|directory, _| plan.selected.contains(directory));
}

pub fn clean_cache(plan: &Plan, cache: &Path) -> io::Result<()> {
    crate::clean::retain(cache, |path| {
        plan.sources.iter().any(|source| source.as_path() == path)
    })
}
