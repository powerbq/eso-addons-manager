use std::fs;
use std::path::{Path, PathBuf};

use crate::paths::Paths;

const OLD_CONFIG: &str = "app.ini";
const OLD_CACHE: &str = "addons";
const OLD_CUSTOM: &str = "custom";

pub enum Outcome {
    Nothing,
    Migrated,
    Conflict(Conflict),
}

pub struct Conflict {
    pub config: bool,
    pub custom: Vec<String>,
    pub old_config: PathBuf,
    pub old_custom: PathBuf,
    pub folders: Vec<PathBuf>,
}

pub fn run(paths: &Paths) -> Outcome {
    let Ok(cwd) = std::env::current_dir() else {
        return Outcome::Nothing;
    };

    let old_config = cwd.join(OLD_CONFIG);
    let old_cache = cwd.join(OLD_CACHE);
    let old_custom = cwd.join(OLD_CUSTOM);

    if !old_config.is_file() && !old_cache.is_dir() && !old_custom.is_dir() {
        return Outcome::Nothing;
    }

    tracing::info!("migrating data from the working directory to the profile location");

    if old_cache.is_dir() {
        merge_into(&old_cache, &paths.cache_dir, &[]);
        remove_if_empty(&old_cache);
    }

    let config_conflict = old_config.is_file() && paths.config_file.exists();
    let custom_conflicts = if old_custom.is_dir() {
        conflicting_names(&old_custom, &paths.custom_dir)
    } else {
        Vec::new()
    };

    if old_config.is_file() && !config_conflict {
        move_entry(&old_config, &paths.config_file);
    }
    if old_custom.is_dir() {
        merge_into(&old_custom, &paths.custom_dir, &custom_conflicts);
        if custom_conflicts.is_empty() {
            remove_if_empty(&old_custom);
        }
    }

    if !config_conflict && custom_conflicts.is_empty() {
        return Outcome::Migrated;
    }

    let mut folders = Vec::new();
    if config_conflict {
        folders.push(cwd.clone());
        if let Some(parent) = paths.config_file.parent() {
            folders.push(parent.to_path_buf());
        }
    }
    if !custom_conflicts.is_empty() {
        folders.push(old_custom.clone());
        folders.push(paths.custom_dir.clone());
    }

    Outcome::Conflict(Conflict {
        config: config_conflict,
        custom: custom_conflicts,
        old_config,
        old_custom,
        folders,
    })
}

pub fn resolve(conflict: &Conflict, keep_old: bool, paths: &Paths) {
    if keep_old {
        if conflict.config {
            move_entry(&conflict.old_config, &paths.config_file);
        }
        for name in &conflict.custom {
            move_entry(&conflict.old_custom.join(name), &paths.custom_dir.join(name));
        }
    } else {
        if conflict.config {
            let _ = fs::remove_file(&conflict.old_config);
        }
        for name in &conflict.custom {
            let _ = fs::remove_file(conflict.old_custom.join(name));
        }
    }
    remove_if_empty(&conflict.old_custom);
}

fn conflicting_names(src: &Path, dest: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(src) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| dest.join(name).exists())
        .collect()
}

fn merge_into(src: &Path, dest: &Path, skip: &[String]) {
    let Ok(entries) = fs::read_dir(src) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if skip.contains(&name) {
            continue;
        }
        move_entry(&path, &dest.join(&name));
    }
}

fn move_entry(src: &Path, dest: &Path) {
    if let Some(parent) = dest.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        tracing::warn!("cannot create {}: {error}", parent.display());
        return;
    }
    let _ = fs::remove_file(dest);
    if fs::rename(src, dest).is_ok() {
        return;
    }
    match fs::copy(src, dest) {
        Ok(_) => {
            let _ = fs::remove_file(src);
        }
        Err(error) => tracing::warn!("cannot migrate {}: {error}", src.display()),
    }
}

fn remove_if_empty(dir: &Path) {
    if fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none()) {
        let _ = fs::remove_dir(dir);
    }
}
