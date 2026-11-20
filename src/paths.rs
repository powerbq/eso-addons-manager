use std::path::PathBuf;

use directories::BaseDirs;
use eso_addons_manager_core::cache;

#[derive(Debug, Clone)]
pub struct Paths {
    pub config_file: PathBuf,
    pub cache_dir: PathBuf,
    pub custom_dir: PathBuf,
    pub log_dir: PathBuf,
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
        ];
        for directory in directories.into_iter().flatten() {
            if let Err(error) = std::fs::create_dir_all(directory) {
                tracing::warn!("cannot create {}: {error}", directory.display());
            }
        }
    }

    fn fallback() -> Self {
        Self {
            config_file: PathBuf::from("app.ini"),
            cache_dir: PathBuf::from("cache"),
            custom_dir: PathBuf::from("custom"),
            log_dir: PathBuf::from("logs"),
        }
    }

    #[cfg(target_os = "windows")]
    fn from_base(base: BaseDirs) -> Self {
        let root = base.data_local_dir().join("EsoAddonsManager");
        Self {
            config_file: root.join("config").join("app.ini"),
            cache_dir: root.join("cache"),
            custom_dir: root.join("data").join("custom"),
            log_dir: root.join("logs"),
        }
    }

    #[cfg(target_os = "macos")]
    fn from_base(base: BaseDirs) -> Self {
        const ID: &str = "io.github.powerbq.eso-addons-manager";
        let support = base.config_dir().join(ID);
        Self {
            config_file: support.join("app.ini"),
            cache_dir: base.cache_dir().join(ID),
            custom_dir: support.join("custom"),
            log_dir: base.home_dir().join("Library").join("Logs").join(ID),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    fn from_base(base: BaseDirs) -> Self {
        const NAME: &str = "eso-addons-manager";
        let state = base.state_dir().unwrap_or_else(|| base.data_dir()).join(NAME);
        Self {
            config_file: base.config_dir().join(NAME).join("app.ini"),
            cache_dir: base.cache_dir().join(NAME),
            custom_dir: base.data_dir().join(NAME).join("custom"),
            log_dir: state.join("logs"),
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
