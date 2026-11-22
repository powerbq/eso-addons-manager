use std::path::PathBuf;

use directories::BaseDirs;
use eso_addons_manager_core::{cache, config};

#[derive(Debug, Clone)]
pub struct Paths {
    pub config_file: PathBuf,
    pub cache_dir: PathBuf,
    pub custom_dir: PathBuf,
    pub log_dir: PathBuf,
    // Single-instance lock. Runtime state, so it lives in the OS runtime directory where there is
    // one (Linux `$XDG_RUNTIME_DIR`, cleared on logout), otherwise in the app's local data root —
    // never the config directory (which may be synced) nor cache/logs (which the app sweeps).
    pub lock_file: PathBuf,
}

impl Paths {
    pub fn resolve() -> Self {
        BaseDirs::new().map_or_else(Self::fallback, Self::from_base)
    }

    pub fn ensure(&self) {
        let addons = self.cache_dir.join(cache::DIRECTORY);
        let directories = [
            self.config_file.parent(),
            Some(addons.as_path()),
            Some(self.custom_dir.as_path()),
            Some(self.log_dir.as_path()),
            self.lock_file.parent(),
        ];
        for directory in directories.into_iter().flatten() {
            if let Err(error) = std::fs::create_dir_all(directory) {
                tracing::warn!("cannot create {}: {error}", directory.display());
            }
        }
    }

    fn fallback() -> Self {
        Self {
            config_file: PathBuf::from(config::FILE),
            cache_dir: PathBuf::from("cache"),
            custom_dir: PathBuf::from("custom"),
            log_dir: PathBuf::from("logs"),
            lock_file: PathBuf::from("run").join("lock"),
        }
    }

    #[cfg(target_os = "windows")]
    fn from_base(base: BaseDirs) -> Self {
        let root = base.data_local_dir().join("EsoAddonsManager");
        Self {
            config_file: root.join("config").join(config::FILE),
            cache_dir: root.join("cache"),
            custom_dir: root.join("data").join("custom"),
            log_dir: root.join("logs"),
            lock_file: root.join("run").join("lock"),
        }
    }

    #[cfg(target_os = "macos")]
    fn from_base(base: BaseDirs) -> Self {
        const ID: &str = "io.github.powerbq.eso-addons-manager";
        let support = base.config_dir().join(ID);
        Self {
            config_file: support.join(config::FILE),
            cache_dir: base.cache_dir().join(ID),
            custom_dir: support.join("custom"),
            log_dir: base.home_dir().join("Library").join("Logs").join(ID),
            lock_file: support.join("run").join("lock"),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    fn from_base(base: BaseDirs) -> Self {
        const NAME: &str = "eso-addons-manager";
        let state = base
            .state_dir()
            .unwrap_or_else(|| base.data_dir())
            .join(NAME);
        // Runtime dir (tmpfs, per-session) is the right home for a lock and is itself the "run"
        // location, so the file sits directly in the app folder there; without a runtime dir we
        // fall back to a `run/` subfolder of the local data root.
        let lock_file = base
            .runtime_dir()
            .map_or_else(
                || base.data_dir().join(NAME).join("run"),
                |runtime| runtime.join(NAME),
            )
            .join("lock");
        Self {
            config_file: base.config_dir().join(NAME).join(config::FILE),
            cache_dir: base.cache_dir().join(NAME),
            custom_dir: base.data_dir().join(NAME).join("custom"),
            log_dir: state.join("logs"),
            lock_file,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Paths;

    #[test]
    fn resolves_named_paths() {
        let paths = Paths::resolve();
        assert_eq!(paths.config_file.file_name().unwrap(), "app.ini");
        assert_eq!(paths.custom_dir.file_name().unwrap(), "custom");
        assert!(paths.cache_dir.file_name().is_some());
        assert!(paths.log_dir.file_name().is_some());
    }
}
