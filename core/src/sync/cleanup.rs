use std::collections::HashSet;
use std::fs;
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
    let entries = match fs::read_dir(cache) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };

    for entry in entries {
        let path = entry?.path();
        if plan.sources.contains(&path) {
            continue;
        }

        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}
