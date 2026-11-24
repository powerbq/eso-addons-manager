use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

use encoding_rs::{Encoding, UTF_8};
use ini::{EscapePolicy, Ini, LineSeparator, ParseOption, WriteOption};

use crate::api::ApiVersion;
use crate::catalogue::{Filter, SortOrder};
use crate::ttc::Region;

pub const FILE: &str = "app.ini";

const GENERAL: &str = "General";
const CATALOGUE: &str = "Catalogue";
const TTC: &str = "TTC";
const ADDONS: &str = "AddOns";
const FAVOURITES: &str = "Favourites";
const SELECTED_LIBRARIES: &str = "SelectedLibraries";
const EXCLUSIONS: &str = "Exclusions";

const TARGET_DIRECTORY: &str = "TargetDirectory";
const SYNC_ON_LAUNCH: &str = "SyncOnLaunch";
const LANGUAGE: &str = "Language";
const LOG_TO_FILE: &str = "LogToFile";
const CHECK_FOR_UPDATES: &str = "CheckForUpdates";
const NOTIFY_PRERELEASES: &str = "NotifyPrereleases";
const API_VERSION: &str = "ApiVersion";
const UPDATE_PRICES: &str = "UpdatePrices";
const PRICE_REGION: &str = "PriceRegion";
const SORT: &str = "Sort";
const SORT_REVERSED: &str = "SortReversed";
const FILTER_INSTALLED: &str = "FilterInstalled";
const FILTER_FAVOURITES: &str = "FilterFavourites";

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("cannot access the settings file: {0}")]
    Io(Arc<io::Error>),
    #[error("cannot parse the settings file: {0}")]
    Parse(Arc<ini::ParseError>),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

impl From<ini::ParseError> for Error {
    fn from(error: ini::ParseError) -> Self {
        Self::Parse(Arc::new(error))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub target_directory: String,
    pub sync_on_launch: bool,
    pub language: String,
    pub log_to_file: bool,
    pub check_for_updates: bool,
    pub notify_prereleases: bool,
    pub api_version: ApiVersion,
    pub update_prices: bool,
    pub price_region: Region,
    pub sort: SortOrder,
    pub sort_reversed: bool,
    pub filter_installed: Filter,
    pub filter_favourites: Filter,
    pub addons: BTreeMap<String, String>,
    pub favourites: BTreeMap<String, String>,
    pub selected_libraries: BTreeMap<String, String>,
    pub exclusions: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target_directory: String::new(),
            sync_on_launch: false,
            language: String::new(),
            log_to_file: true,
            check_for_updates: true,
            notify_prereleases: false,
            api_version: ApiVersion::default(),
            update_prices: true,
            price_region: Region::default(),
            sort: SortOrder::default(),
            sort_reversed: false,
            filter_installed: Filter::default(),
            filter_favourites: Filter::default(),
            addons: BTreeMap::new(),
            favourites: BTreeMap::new(),
            selected_libraries: BTreeMap::new(),
            exclusions: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, Error> {
        Self::load_encoded(path, system_encoding())
    }

    pub fn save(&self, path: &Path) -> Result<(), Error> {
        self.save_encoded(path, system_encoding())
    }

    fn load_encoded(path: &Path, encoding: &'static Encoding) -> Result<Self, Error> {
        match fs::read(path) {
            Ok(bytes) => {
                let (text, _) = encoding.decode_without_bom_handling(&bytes);
                let config = Self::parse(&text)?;
                tracing::info!("loaded config from {}", path.display());
                Ok(config)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                tracing::debug!("no config at {}, using defaults", path.display());
                Ok(Self::default())
            }
            Err(error) => Err(error.into()),
        }
    }

    fn save_encoded(&self, path: &Path, encoding: &'static Encoding) -> Result<(), Error> {
        let mut temporary = path.as_os_str().to_owned();
        temporary.push(".tmp");

        fs::write(&temporary, encode(&self.to_ini_string(), encoding))?;
        fs::rename(&temporary, path)?;
        tracing::debug!("saved config to {}", path.display());
        Ok(())
    }

    fn parse(text: &str) -> Result<Self, ini::ParseError> {
        let options = ParseOption {
            enabled_quote: false,
            enabled_escape: false,
            ..ParseOption::default()
        };
        let ini = Ini::load_from_str_opt(text, options)?;

        let mut config = Self::default();

        for (section, properties) in ini.iter() {
            let Some(section) = section else {
                continue;
            };
            let entries = properties
                .iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()));

            match section {
                GENERAL => {
                    for (key, value) in entries {
                        match key.as_str() {
                            TARGET_DIRECTORY => config.target_directory = value,
                            SYNC_ON_LAUNCH => config.sync_on_launch = value == "true",
                            LANGUAGE => config.language = value,
                            LOG_TO_FILE => config.log_to_file = value == "true",
                            CHECK_FOR_UPDATES => config.check_for_updates = value == "true",
                            NOTIFY_PRERELEASES => config.notify_prereleases = value == "true",
                            API_VERSION => {
                                config.api_version =
                                    ApiVersion::from_key(&value).unwrap_or_default()
                            }
                            _ => {}
                        }
                    }
                }
                CATALOGUE => {
                    for (key, value) in entries {
                        match key.as_str() {
                            SORT => config.sort = SortOrder::from_key(&value).unwrap_or_default(),
                            SORT_REVERSED => config.sort_reversed = value == "true",
                            FILTER_INSTALLED => {
                                config.filter_installed =
                                    Filter::from_key(&value).unwrap_or_default()
                            }
                            FILTER_FAVOURITES => {
                                config.filter_favourites =
                                    Filter::from_key(&value).unwrap_or_default()
                            }
                            _ => {}
                        }
                    }
                }
                TTC => {
                    for (key, value) in entries {
                        match key.as_str() {
                            UPDATE_PRICES => config.update_prices = value == "true",
                            PRICE_REGION => {
                                config.price_region = Region::from_key(&value).unwrap_or_default()
                            }
                            _ => {}
                        }
                    }
                }
                ADDONS => config.addons.extend(entries),
                FAVOURITES => config.favourites.extend(entries),
                SELECTED_LIBRARIES => config.selected_libraries.extend(entries),
                EXCLUSIONS => config
                    .exclusions
                    .extend(entries.map(|(_, pattern)| pattern)),
                _ => {}
            }
        }

        Ok(config)
    }

    fn to_ini_string(&self) -> String {
        let mut ini = Ini::new();
        let mut set = |section: &str, key: &str, value: &str| {
            ini.set_to(Some(section), key.to_owned(), value.to_owned());
        };

        set(GENERAL, TARGET_DIRECTORY, &self.target_directory);
        set(
            GENERAL,
            SYNC_ON_LAUNCH,
            if self.sync_on_launch { "true" } else { "false" },
        );
        if !self.language.is_empty() {
            set(GENERAL, LANGUAGE, &self.language);
        }
        set(
            GENERAL,
            LOG_TO_FILE,
            if self.log_to_file { "true" } else { "false" },
        );
        set(
            GENERAL,
            CHECK_FOR_UPDATES,
            if self.check_for_updates {
                "true"
            } else {
                "false"
            },
        );
        set(
            GENERAL,
            NOTIFY_PRERELEASES,
            if self.notify_prereleases {
                "true"
            } else {
                "false"
            },
        );
        set(GENERAL, API_VERSION, self.api_version.as_key());

        set(CATALOGUE, SORT, self.sort.as_key());
        set(
            CATALOGUE,
            SORT_REVERSED,
            if self.sort_reversed { "true" } else { "false" },
        );
        set(CATALOGUE, FILTER_INSTALLED, self.filter_installed.as_key());
        set(
            CATALOGUE,
            FILTER_FAVOURITES,
            self.filter_favourites.as_key(),
        );

        set(
            TTC,
            UPDATE_PRICES,
            if self.update_prices { "true" } else { "false" },
        );
        set(TTC, PRICE_REGION, self.price_region.as_key());

        for (section, entries) in [
            (ADDONS, &self.addons),
            (FAVOURITES, &self.favourites),
            (SELECTED_LIBRARIES, &self.selected_libraries),
        ] {
            for (key, value) in entries {
                set(section, key, value);
            }
        }

        for (index, pattern) in self.exclusions.iter().enumerate() {
            set(EXCLUSIONS, &index.to_string(), pattern);
        }

        let options = WriteOption {
            escape_policy: EscapePolicy::Nothing,
            line_separator: LineSeparator::SystemDefault,
            kv_separator: " = ",
        };
        let mut buffer = Vec::new();
        ini.write_to_opt(&mut buffer, options)
            .expect("writing to memory cannot fail");

        String::from_utf8(buffer).expect("ini output is built from strings")
    }
}

#[cfg(windows)]
fn system_encoding() -> &'static Encoding {
    let code_page = unsafe { windows_sys::Win32::Globalization::GetACP() };

    u16::try_from(code_page)
        .ok()
        .and_then(codepage::to_encoding_no_replacement)
        .unwrap_or(UTF_8)
}

#[cfg(not(windows))]
fn system_encoding() -> &'static Encoding {
    UTF_8
}

fn encode(text: &str, encoding: &'static Encoding) -> Vec<u8> {
    let (bytes, _, unmappable) = encoding.encode(text);
    if !unmappable {
        return bytes.into_owned();
    }

    let mut buffer = [0; 4];
    text.chars()
        .flat_map(|character| {
            let (bytes, _, unmappable) = encoding.encode(character.encode_utf8(&mut buffer));
            if unmappable {
                vec![b'?']
            } else {
                bytes.into_owned()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use encoding_rs::{WINDOWS_1251, WINDOWS_1252};

    use super::*;

    const SAMPLE: &str = "[General]\r\n\
        SyncOnLaunch = true\r\n\
        TargetDirectory = C:\\Users\\User\\Documents\\Elder Scrolls Online\\live\\AddOns\r\n\
        \r\n\
        [AddOns]\r\n\
        5058 = Group Builds\r\n\
        1399 = Votan's Minimap\r\n\
        \r\n\
        [Favourites]\r\n\
        \r\n\
        [SelectedLibraries]\r\n\
        LibAddonMenu-2.0 = 7\r\n\
        \r\n\
        [7]\r\n\
        UIMD5 = 59b38ee81d94d365426c71a368fa1df4\r\n\
        UIVersion = 2.0 r43\r\n\
        \r\n\
        [Exclusions]\r\n\
        \r\n\
        [3649]\r\n\
        UIMD5 = c95ee7a92e8ef33314b39d7bc22b2bba\r\n\
        UIVersion = 26\r\n\
        \r\n";

    #[test]
    fn reads_file_written_by_old_version() {
        let config = Config::parse(SAMPLE).unwrap();

        assert_eq!(
            config.target_directory,
            r"C:\Users\User\Documents\Elder Scrolls Online\live\AddOns"
        );
        assert!(config.sync_on_launch);
        assert_eq!(config.language, "");
        assert_eq!(config.addons["5058"], "Group Builds");
        assert_eq!(config.addons["1399"], "Votan's Minimap");
        assert!(config.favourites.is_empty());
        assert_eq!(config.selected_libraries["LibAddonMenu-2.0"], "7");
        assert!(config.exclusions.is_empty());
    }

    #[test]
    fn defaults_when_values_are_missing() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
        assert_eq!(Config::default().target_directory, "");
    }

    #[test]
    fn writes_what_it_reads() {
        let mut config = Config::parse(SAMPLE).unwrap();
        config.language = "ru".to_owned();
        config
            .favourites
            .insert("1399".to_owned(), "Votan's Minimap".to_owned());
        config.exclusions = vec![r"^Foo/Bar\.lua$".to_owned(), "^Baz/$".to_owned()];

        assert_eq!(Config::parse(&config.to_ini_string()).unwrap(), config);
    }

    #[test]
    fn keeps_exclusions_in_order() {
        let config = Config {
            exclusions: vec!["^z$".to_owned(), "^a$".to_owned(), "^m$".to_owned()],
            ..Config::default()
        };

        assert_eq!(
            Config::parse(&config.to_ini_string()).unwrap().exclusions,
            config.exclusions
        );
    }

    #[test]
    fn keeps_semicolons_hashes_and_quotes_in_values() {
        let config = Config::parse("[Exclusions]\n0 = ^a;b#c\"d'\\.lua$\n").unwrap();

        assert_eq!(config.exclusions, [r#"^a;b#c"d'\.lua$"#]);
    }

    #[test]
    fn drops_unknown_keys_and_sections() {
        let config = Config::parse(
            "[General]\nObsolete = 1\n\
             [AddOns]\n12 = Fine\n\
             [Window]\nWidth = 100\n\
             [12]\nUIVersion = 1\nExtra = x\n",
        )
        .unwrap();

        assert_eq!(
            config,
            Config {
                addons: BTreeMap::from([("12".to_owned(), "Fine".to_owned())]),
                ..Config::default()
            }
        );

        let text = config.to_ini_string();
        assert!(!text.contains("Obsolete"));
        assert!(!text.contains("Window"));
        assert!(!text.contains("Extra"));
        assert!(!text.contains("UIVersion"));
    }

    #[test]
    fn writes_language_only_when_set() {
        assert!(!Config::default().to_ini_string().contains(LANGUAGE));
    }

    #[test]
    fn missing_file_gives_defaults() {
        let directory = tempfile::tempdir().unwrap();

        assert_eq!(
            Config::load(&directory.path().join(FILE)).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn saves_and_loads() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        let config = Config::parse(SAMPLE).unwrap();

        config.save(&path).unwrap();

        assert_eq!(Config::load(&path).unwrap(), config);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn reads_and_writes_utf8() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        let config = Config {
            target_directory: r"C:\Тест\AddOns".to_owned(),
            addons: BTreeMap::from([("1".to_owned(), "Slightly Improved™ Bars".to_owned())]),
            ..Config::default()
        };

        config.save_encoded(&path, UTF_8).unwrap();

        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains(r"TargetDirectory = C:\Тест\AddOns")
        );
        assert_eq!(Config::load_encoded(&path, UTF_8).unwrap(), config);
    }

    #[test]
    fn reads_and_writes_windows_code_page() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        fs::write(
            &path,
            b"[General]\r\nTargetDirectory = C:\\\xD2\xE5\xF1\xF2\\AddOns\r\n",
        )
        .unwrap();

        let config = Config::load_encoded(&path, WINDOWS_1251).unwrap();
        assert_eq!(config.target_directory, r"C:\Тест\AddOns");

        config.save_encoded(&path, WINDOWS_1251).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.windows(4).any(|window| window == b"\xD2\xE5\xF1\xF2"));
        assert_eq!(Config::load_encoded(&path, WINDOWS_1251).unwrap(), config);
    }

    #[test]
    fn characters_missing_from_code_page_become_question_marks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        let config = Config {
            addons: BTreeMap::from([
                ("1".to_owned(), "EsoBR - Tradução".to_owned()),
                ("2".to_owned(), "EsoUA — Українська".to_owned()),
            ]),
            ..Config::default()
        };

        config.save_encoded(&path, WINDOWS_1251).unwrap();
        let in_1251 = Config::load_encoded(&path, WINDOWS_1251).unwrap();
        assert_eq!(in_1251.addons["1"], "EsoBR - Tradu??o");
        assert_eq!(in_1251.addons["2"], "EsoUA — Українська");

        config.save_encoded(&path, WINDOWS_1252).unwrap();
        let in_1252 = Config::load_encoded(&path, WINDOWS_1252).unwrap();
        assert_eq!(in_1252.addons["1"], "EsoBR - Tradução");
        assert_eq!(in_1252.addons["2"], "EsoUA — ??????????");
    }

    #[test]
    fn round_trips_query_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        let config = Config {
            sort: SortOrder::Author,
            sort_reversed: true,
            filter_installed: Filter::Only,
            filter_favourites: Filter::Except,
            ..Config::default()
        };

        config.save_encoded(&path, UTF_8).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("Sort = Author"));
        assert!(text.contains("SortReversed = true"));
        assert!(text.contains("FilterInstalled = Only"));
        assert!(text.contains("FilterFavourites = Except"));
        assert_eq!(Config::load_encoded(&path, UTF_8).unwrap(), config);
    }

    #[test]
    fn check_for_updates_defaults_on_and_round_trips() {
        assert!(Config::default().check_for_updates);
        assert!(Config::parse("").unwrap().check_for_updates);

        let config = Config {
            check_for_updates: false,
            ..Config::default()
        };
        let text = config.to_ini_string();
        assert!(text.contains("CheckForUpdates = false"));
        assert_eq!(Config::parse(&text).unwrap(), config);
    }

    #[test]
    fn notify_prereleases_defaults_off_and_round_trips() {
        assert!(!Config::default().notify_prereleases);
        assert!(!Config::parse("").unwrap().notify_prereleases);

        let config = Config {
            notify_prereleases: true,
            ..Config::default()
        };
        let text = config.to_ini_string();
        assert!(text.contains("NotifyPrereleases = true"));
        assert_eq!(Config::parse(&text).unwrap(), config);
    }

    #[test]
    fn api_version_defaults_v3_and_round_trips() {
        assert_eq!(Config::default().api_version, ApiVersion::V3);
        assert_eq!(Config::parse("").unwrap().api_version, ApiVersion::V3);

        let config = Config {
            api_version: ApiVersion::V4,
            ..Config::default()
        };
        let text = config.to_ini_string();
        assert!(text.contains("ApiVersion = v4"));
        assert_eq!(Config::parse(&text).unwrap(), config);

        let fallback = Config::parse("[General]\nApiVersion = v9\n").unwrap();
        assert_eq!(fallback.api_version, ApiVersion::V3);
    }

    #[test]
    fn price_settings_default_and_round_trip() {
        assert!(Config::default().update_prices);
        assert_eq!(Config::default().price_region, Region::Eu);

        let config = Config {
            update_prices: false,
            price_region: Region::Na,
            ..Config::default()
        };
        let text = config.to_ini_string();
        assert!(text.contains("UpdatePrices = false"));
        assert!(text.contains("PriceRegion = NA"));
        assert_eq!(Config::parse(&text).unwrap(), config);

        let fallback = Config::parse("[TTC]\nPriceRegion = Mars\n").unwrap();
        assert_eq!(fallback.price_region, Region::Eu);
    }

    #[test]
    fn invalid_query_preferences_fall_back_to_defaults() {
        let config =
            Config::parse("[Catalogue]\nSort = Nonsense\nFilterInstalled = Maybe\n").unwrap();
        assert_eq!(config.sort, SortOrder::default());
        assert_eq!(config.filter_installed, Filter::default());
    }
}
