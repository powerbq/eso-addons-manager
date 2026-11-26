use std::fs;
use std::path::Path;

use eso_addons_manager_core::sync::Pruned;

const KEEP: usize = 10;

pub fn before_auto_removal(config_file: &Path, pruned: &Pruned) {
    if !config_file.exists() {
        return;
    }
    let Some(directory) = config_file.parent() else {
        return;
    };
    let Some(name) = config_file.file_stem().and_then(|stem| stem.to_str()) else {
        return;
    };
    let suffix = config_file
        .extension()
        .and_then(|extension| extension.to_str())
        .map_or_else(String::new, |extension| format!(".{extension}"));

    crate::rotate::make_room(directory, name, &suffix, KEEP);
    let slot = directory.join(format!("{name}.1{suffix}"));
    match fs::copy(config_file, &slot) {
        Ok(_) => tracing::info!(
            "backed up {} to {} before auto-removing {} add-on(s) and {} favourite(s)",
            config_file.display(),
            slot.display(),
            pruned.addons.len(),
            pruned.favourites.len()
        ),
        Err(error) => tracing::warn!("cannot back up {}: {error}", config_file.display()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use eso_addons_manager_core::sync::Pruned;

    use super::before_auto_removal;

    #[test]
    fn copies_the_live_file_into_slot_one() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("app.ini");
        fs::write(&config, "live").unwrap();

        before_auto_removal(
            &config,
            &Pruned {
                addons: vec!["99".to_owned()],
                favourites: Vec::new(),
            },
        );

        assert_eq!(fs::read_to_string(&config).unwrap(), "live");
        assert_eq!(
            fs::read_to_string(directory.path().join("app.1.ini")).unwrap(),
            "live"
        );
    }

    #[test]
    fn shifts_an_existing_backup_before_copying() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("app.ini");
        fs::write(&config, "new").unwrap();
        fs::write(directory.path().join("app.1.ini"), "old").unwrap();

        before_auto_removal(
            &config,
            &Pruned {
                addons: vec!["1".to_owned()],
                favourites: Vec::new(),
            },
        );

        assert_eq!(
            fs::read_to_string(directory.path().join("app.1.ini")).unwrap(),
            "new"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("app.2.ini")).unwrap(),
            "old"
        );
    }

    #[test]
    fn does_nothing_without_a_live_file() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("app.ini");

        before_auto_removal(
            &config,
            &Pruned {
                addons: vec!["1".to_owned()],
                favourites: Vec::new(),
            },
        );

        assert!(!directory.path().join("app.1.ini").exists());
    }
}
