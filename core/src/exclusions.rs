use regex::RegexSet;

use crate::config::Config;

const TTC: &str = "1245";
const HARVEST_MAP: &str = "3034";
const TTC_LANGUAGES: [&str; 7] = ["DE", "EN", "ES", "FR", "JP", "RU", "ZH"];
const HARVEST_MAP_REGIONS: [&str; 5] = ["AD", "DC", "EP", "NF", "DLC"];

#[derive(Debug, Clone)]
pub struct Exclusions {
    patterns: RegexSet,
}

impl Exclusions {
    pub fn new(config: &Config) -> Result<Self, regex::Error> {
        let patterns = config.exclusions.iter().cloned().chain(automatic(config));

        Ok(Self {
            patterns: RegexSet::new(patterns)?,
        })
    }

    pub fn matches(&self, path: &str) -> bool {
        self.patterns.is_match(path)
    }
}

pub fn validate(patterns: &[String]) -> Result<(), regex::Error> {
    RegexSet::new(patterns).map(|_| ())
}

pub fn automatic(config: &Config) -> Vec<String> {
    built_in()
        .into_iter()
        .filter(|(uid, _)| config.addons.contains_key(*uid))
        .flat_map(|(_, patterns)| patterns)
        .collect()
}

pub fn built_in() -> Vec<(&'static str, Vec<String>)> {
    let ttc = [
        r"^TamrielTradeCentre/$",
        r"^TamrielTradeCentre/Client/$",
        r"^TamrielTradeCentre/Client/TTC_Lock$",
        r"^TamrielTradeCentre/PriceTableEU\.lua$",
        r"^TamrielTradeCentre/PriceTableNA\.lua$",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(
        TTC_LANGUAGES
            .iter()
            .map(|language| format!(r"^TamrielTradeCentre/ItemLookUpTable_{language}\.lua$")),
    )
    .collect();

    let harvest_map = [r"^HarvestMapData/$", r"^HarvestMapData/Modules/$"]
        .into_iter()
        .map(str::to_owned)
        .chain(HARVEST_MAP_REGIONS.iter().flat_map(|region| {
            [
                format!(r"^HarvestMapData/Modules/HarvestMap{region}/$"),
                format!(r"^HarvestMapData/Modules/HarvestMap{region}/HarvestMap{region}\.lua$"),
            ]
        }))
        .collect();

    vec![(TTC, ttc), (HARVEST_MAP, harvest_map)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(uids: &[&str]) -> Config {
        Config {
            addons: uids
                .iter()
                .map(|&uid| (uid.to_owned(), String::new()))
                .collect(),
            ..Config::default()
        }
    }

    #[test]
    fn built_in_rules_apply_only_to_installed_addons() {
        let path = "TamrielTradeCentre/PriceTableEU.lua";

        assert!(!Exclusions::new(&installed(&[])).unwrap().matches(path));
        assert!(Exclusions::new(&installed(&[TTC])).unwrap().matches(path));
    }

    #[test]
    fn built_in_rules_cover_ttc_and_harvest_map() {
        let exclusions = Exclusions::new(&installed(&[TTC, HARVEST_MAP])).unwrap();

        for path in [
            "TamrielTradeCentre/",
            "TamrielTradeCentre/Client/",
            "TamrielTradeCentre/Client/TTC_Lock",
            "TamrielTradeCentre/PriceTableNA.lua",
            "TamrielTradeCentre/ItemLookUpTable_ZH.lua",
            "HarvestMapData/",
            "HarvestMapData/Modules/",
            "HarvestMapData/Modules/HarvestMapDLC/",
            "HarvestMapData/Modules/HarvestMapAD/HarvestMapAD.lua",
        ] {
            assert!(exclusions.matches(path), "{path}");
        }
        for path in [
            "TamrielTradeCentre/Main.lua",
            "TamrielTradeCentre/Client",
            "HarvestMapData/Modules/HarvestMapAD/Other.lua",
        ] {
            assert!(!exclusions.matches(path), "{path}");
        }
    }

    #[test]
    fn user_rules_match_anywhere_in_the_path() {
        let config = Config {
            exclusions: vec![r"SavedData/".to_owned(), r"^Foo/Dynamic/$".to_owned()],
            ..Config::default()
        };
        let exclusions = Exclusions::new(&config).unwrap();

        assert!(exclusions.matches("Bar/SavedData/file.lua"));
        assert!(exclusions.matches("Foo/Dynamic/"));
        assert!(!exclusions.matches("Foo/Dynamic/file.lua"));
    }

    #[test]
    fn validates_patterns() {
        assert!(validate(&[r"^Foo/$".to_owned(), r"Bar\.lua$".to_owned()]).is_ok());
        assert!(validate(&["(".to_owned()]).is_err());
    }

    #[test]
    fn automatic_rules_follow_installed_addons() {
        assert!(automatic(&installed(&[])).is_empty());
        assert_eq!(automatic(&installed(&[TTC])).len(), 12);
    }

    #[test]
    fn invalid_user_rule_is_an_error() {
        let config = Config {
            exclusions: vec!["(".to_owned()],
            ..Config::default()
        };

        assert!(Exclusions::new(&config).is_err());
    }
}
