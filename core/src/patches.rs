use std::borrow::Cow;
use std::collections::BTreeSet;

use regex::Regex;
use regex::bytes::Regex as BytesRegex;

const DOVAHMOVA: &str = "4202";

type Spec = (&'static str, &'static str, &'static str);
type Group = (&'static str, Vec<Spec>);

#[derive(Debug, Clone)]
struct Rule {
    path: Regex,
    from: BytesRegex,
    to: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Patches {
    rules: Vec<Rule>,
}

impl Patches {
    pub(crate) fn new(installed: &BTreeSet<String>) -> Result<Self, regex::Error> {
        let mut rules = Vec::new();

        for (uid, specs) in built_in() {
            if !installed.contains(uid) {
                continue;
            }
            for (path, from, to) in specs {
                rules.push(Rule {
                    path: Regex::new(path)?,
                    from: BytesRegex::new(from)?,
                    to: to.as_bytes().to_vec(),
                });
            }
        }

        Ok(Self { rules })
    }

    pub(crate) fn matches(&self, path: &str) -> bool {
        self.rules.iter().any(|rule| rule.path.is_match(path))
    }

    pub(crate) fn apply<'a>(&self, path: &str, content: &'a [u8]) -> Cow<'a, [u8]> {
        let mut out = Cow::Borrowed(content);

        for rule in &self.rules {
            if rule.path.is_match(path) {
                let patched = rule.from.replace_all(&out, rule.to.as_slice()).into_owned();
                out = Cow::Owned(patched);
            }
        }

        out
    }
}

fn built_in() -> Vec<Group> {
    let dovahmova = vec![(
        r"^TamrielTradeCentre/TamrielTradeCentre\.lua$",
        r#"(if\s*\(\s*clientCulture\s*~=\s*"en".*?)(\)\s*then)"#,
        r#"$1 and clientCulture ~= "ua"$2"#,
    )];

    vec![(DOVAHMOVA, dovahmova)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(uids: &[&str]) -> BTreeSet<String> {
        uids.iter().map(|&uid| uid.to_owned()).collect()
    }

    const TTC_LUA: &str = "TamrielTradeCentre/TamrielTradeCentre.lua";
    const GUARD: &str = r#"if (clientCulture~= "en" and clientCulture ~= "de" and clientCulture ~= "fr" and clientCulture ~= "zh" and clientCulture ~= "ru" and clientCulture ~= "es" and clientCulture ~= "jp") then"#;
    const PATCHED: &str = r#"if (clientCulture~= "en" and clientCulture ~= "de" and clientCulture ~= "fr" and clientCulture ~= "zh" and clientCulture ~= "ru" and clientCulture ~= "es" and clientCulture ~= "jp" and clientCulture ~= "ua") then"#;

    #[test]
    fn adds_ukrainian_to_ttc_guard_when_dovahmova_installed() {
        let patches = Patches::new(&installed(&[DOVAHMOVA])).unwrap();
        let out = patches.apply(TTC_LUA, GUARD.as_bytes());
        assert_eq!(out.as_ref(), PATCHED.as_bytes());
    }

    #[test]
    fn insert_is_byte_identical_to_the_setup_script() {
        let patches = Patches::new(&installed(&[DOVAHMOVA])).unwrap();
        let out = patches.apply(TTC_LUA, GUARD.as_bytes());
        assert!(
            std::str::from_utf8(&out)
                .unwrap()
                .ends_with(r#"clientCulture ~= "jp" and clientCulture ~= "ua") then"#)
        );
    }

    #[test]
    fn patch_survives_other_languages_in_the_list() {
        let patches = Patches::new(&installed(&[DOVAHMOVA])).unwrap();
        let extended = r#"if (clientCulture ~= "en" and clientCulture ~= "de" and clientCulture ~= "xx" and clientCulture ~= "jp") then"#;
        let out = patches.apply(TTC_LUA, extended.as_bytes());
        assert_eq!(
            out.as_ref(),
            r#"if (clientCulture ~= "en" and clientCulture ~= "de" and clientCulture ~= "xx" and clientCulture ~= "jp" and clientCulture ~= "ua") then"#
                .as_bytes()
        );
    }

    #[test]
    fn leaves_content_untouched_without_dovahmova() {
        let patches = Patches::new(&installed(&[])).unwrap();
        let out = patches.apply(TTC_LUA, GUARD.as_bytes());
        assert_eq!(out.as_ref(), GUARD.as_bytes());
        assert!(matches!(out, Cow::Borrowed(_)));
        assert!(!patches.matches(TTC_LUA));
    }

    #[test]
    fn only_touches_the_matching_path() {
        let patches = Patches::new(&installed(&[DOVAHMOVA])).unwrap();
        assert!(patches.matches(TTC_LUA));
        assert!(!patches.matches("TamrielTradeCentre/Main.lua"));

        let out = patches.apply("SomeAddon/Main.lua", GUARD.as_bytes());
        assert_eq!(out.as_ref(), GUARD.as_bytes());
        assert!(matches!(out, Cow::Borrowed(_)));
    }

    #[test]
    fn tolerates_the_no_space_and_spaced_first_token() {
        let patches = Patches::new(&installed(&[DOVAHMOVA])).unwrap();
        let spaced = r#"if (clientCulture ~= "en" and clientCulture ~= "jp") then"#;
        let out = patches.apply(TTC_LUA, spaced.as_bytes());
        assert_eq!(
            out.as_ref(),
            r#"if (clientCulture ~= "en" and clientCulture ~= "jp" and clientCulture ~= "ua") then"#
                .as_bytes()
        );
    }
}
