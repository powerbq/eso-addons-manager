use std::borrow::Cow;
use std::collections::BTreeSet;

use regex::Regex;
use regex::bytes::Regex as BytesRegex;

use crate::config::{Config, Patch};

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
    pub(crate) fn new(config: &Config, installed: &BTreeSet<String>) -> Result<Self, regex::Error> {
        let mut rules = Vec::new();

        // Automatic (built-in) rules fold first, gated on the installed addon…
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

        // …then the user's custom rules, in their order; each sees the previous rules' output.
        for patch in &config.patches {
            if !patch.enabled {
                continue;
            }
            rules.push(Rule {
                path: Regex::new(&patch.path)?,
                from: BytesRegex::new(&patch.from)?,
                to: patch.to.as_bytes().to_vec(),
            });
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

pub fn validate(patches: &[Patch]) -> Result<(), regex::Error> {
    for patch in patches {
        Regex::new(&patch.path)?;
        BytesRegex::new(&patch.from)?;
    }
    Ok(())
}

pub fn automatic(installed: &BTreeSet<String>) -> Vec<(String, Patch)> {
    built_in()
        .into_iter()
        .filter(|(uid, _)| installed.contains(*uid))
        .flat_map(|(uid, specs)| {
            specs.into_iter().map(move |(path, from, to)| {
                (
                    uid.to_owned(),
                    Patch {
                        path: path.to_owned(),
                        from: from.to_owned(),
                        to: to.to_owned(),
                        enabled: true,
                    },
                )
            })
        })
        .collect()
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
        let patches = Patches::new(&Config::default(), &installed(&[DOVAHMOVA])).unwrap();
        let out = patches.apply(TTC_LUA, GUARD.as_bytes());
        assert_eq!(out.as_ref(), PATCHED.as_bytes());
    }

    #[test]
    fn insert_is_byte_identical_to_the_setup_script() {
        let patches = Patches::new(&Config::default(), &installed(&[DOVAHMOVA])).unwrap();
        let out = patches.apply(TTC_LUA, GUARD.as_bytes());
        assert!(
            std::str::from_utf8(&out)
                .unwrap()
                .ends_with(r#"clientCulture ~= "jp" and clientCulture ~= "ua") then"#)
        );
    }

    #[test]
    fn patch_survives_other_languages_in_the_list() {
        let patches = Patches::new(&Config::default(), &installed(&[DOVAHMOVA])).unwrap();
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
        let patches = Patches::new(&Config::default(), &installed(&[])).unwrap();
        let out = patches.apply(TTC_LUA, GUARD.as_bytes());
        assert_eq!(out.as_ref(), GUARD.as_bytes());
        assert!(matches!(out, Cow::Borrowed(_)));
        assert!(!patches.matches(TTC_LUA));
    }

    #[test]
    fn only_touches_the_matching_path() {
        let patches = Patches::new(&Config::default(), &installed(&[DOVAHMOVA])).unwrap();
        assert!(patches.matches(TTC_LUA));
        assert!(!patches.matches("TamrielTradeCentre/Main.lua"));

        let out = patches.apply("SomeAddon/Main.lua", GUARD.as_bytes());
        assert_eq!(out.as_ref(), GUARD.as_bytes());
        assert!(matches!(out, Cow::Borrowed(_)));
    }

    #[test]
    fn tolerates_the_no_space_and_spaced_first_token() {
        let patches = Patches::new(&Config::default(), &installed(&[DOVAHMOVA])).unwrap();
        let spaced = r#"if (clientCulture ~= "en" and clientCulture ~= "jp") then"#;
        let out = patches.apply(TTC_LUA, spaced.as_bytes());
        assert_eq!(
            out.as_ref(),
            r#"if (clientCulture ~= "en" and clientCulture ~= "jp" and clientCulture ~= "ua") then"#
                .as_bytes()
        );
    }

    fn config(patches: Vec<Patch>) -> Config {
        Config {
            patches,
            ..Config::default()
        }
    }

    #[test]
    fn custom_rules_apply_without_an_addon_gate() {
        let config = config(vec![Patch {
            path: r"^Foo/Bar\.lua$".to_owned(),
            from: "alpha".to_owned(),
            to: "beta".to_owned(),
            enabled: true,
        }]);
        let patches = Patches::new(&config, &installed(&[])).unwrap();

        assert!(patches.matches("Foo/Bar.lua"));
        let out = patches.apply("Foo/Bar.lua", b"alpha gamma");
        assert_eq!(out.as_ref(), b"beta gamma");
    }

    #[test]
    fn disabled_custom_rules_are_skipped() {
        let config = config(vec![Patch {
            path: r"^Foo/Bar\.lua$".to_owned(),
            from: "alpha".to_owned(),
            to: "beta".to_owned(),
            enabled: false,
        }]);
        let patches = Patches::new(&config, &installed(&[])).unwrap();

        assert!(!patches.matches("Foo/Bar.lua"));
    }

    #[test]
    fn custom_rules_fold_after_built_ins_in_order() {
        // Built-in UA guard runs first; two custom rules then rewrite its output in sequence.
        let config = config(vec![
            Patch {
                path: r"\.lua$".to_owned(),
                from: r#""ua""#.to_owned(),
                to: r#""uk""#.to_owned(),
                enabled: true,
            },
            Patch {
                path: r"\.lua$".to_owned(),
                from: r#""uk""#.to_owned(),
                to: r#""ua-UA""#.to_owned(),
                enabled: true,
            },
        ]);
        let patches = Patches::new(&config, &installed(&[DOVAHMOVA])).unwrap();
        let out = patches.apply(TTC_LUA, GUARD.as_bytes());

        assert!(
            std::str::from_utf8(&out)
                .unwrap()
                .ends_with(r#"clientCulture ~= "ua-UA") then"#)
        );
    }

    #[test]
    fn to_supports_group_refs_and_escaped_dollar() {
        let config = config(vec![Patch {
            path: r"^f$".to_owned(),
            from: r"(\w+)=(\w+)".to_owned(),
            to: "$2=$1 ($$)".to_owned(),
            enabled: true,
        }]);
        let patches = Patches::new(&config, &installed(&[])).unwrap();

        let out = patches.apply("f", b"left=right");
        assert_eq!(out.as_ref(), b"right=left ($)");
    }

    #[test]
    fn validates_path_and_from() {
        let ok = [Patch {
            path: r"^a$".to_owned(),
            from: r"b+".to_owned(),
            to: "c".to_owned(),
            enabled: true,
        }];
        assert!(validate(&ok).is_ok());

        let bad_path = [Patch {
            path: "(".to_owned(),
            ..ok[0].clone()
        }];
        assert!(validate(&bad_path).is_err());

        let bad_from = [Patch {
            from: "(".to_owned(),
            ..ok[0].clone()
        }];
        assert!(validate(&bad_from).is_err());
    }

    #[test]
    fn invalid_custom_rule_is_an_error() {
        let config = config(vec![Patch {
            path: "(".to_owned(),
            from: "x".to_owned(),
            to: "y".to_owned(),
            enabled: true,
        }]);
        assert!(Patches::new(&config, &installed(&[])).is_err());
    }

    #[test]
    fn automatic_lists_built_ins_for_installed_addons() {
        assert!(automatic(&installed(&[])).is_empty());

        let dovahmova = automatic(&installed(&[DOVAHMOVA]));
        assert_eq!(dovahmova.len(), 1);
        assert_eq!(dovahmova[0].0, DOVAHMOVA);
        assert_eq!(
            dovahmova[0].1.path,
            r"^TamrielTradeCentre/TamrielTradeCentre\.lua$"
        );
        assert!(dovahmova[0].1.enabled);
    }
}
