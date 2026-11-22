use std::fmt;
use std::sync::LazyLock;

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::unic_langid::LanguageIdentifier;
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("the fallback language is embedded");
    loader.set_use_isolating(false);
    loader
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    pub code: &'static str,
    name: &'static str,
}

pub const LANGUAGES: [Language; 6] = [
    Language {
        code: "en",
        name: "English",
    },
    Language {
        code: "fr",
        name: "Français",
    },
    Language {
        code: "de",
        name: "Deutsch",
    },
    Language {
        code: "es",
        name: "Español",
    },
    Language {
        code: "ru",
        name: "Русский",
    },
    Language {
        code: "uk",
        name: "Українська",
    },
];

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

pub fn select(code: &str) -> Language {
    let language = find(code).or_else(system).unwrap_or(LANGUAGES[0]);
    let identifier: LanguageIdentifier = language
        .code
        .parse()
        .expect("language codes are valid identifiers");

    let mut languages = vec![identifier];
    if languages[0] != *LOADER.fallback_language() {
        languages.push(LOADER.fallback_language().clone());
    }

    if let Err(error) = LOADER.load_languages(&Localizations, &languages) {
        tracing::error!("cannot load language {}: {error}", language.code);
    }

    language
}

fn find(code: &str) -> Option<Language> {
    LANGUAGES
        .iter()
        .copied()
        .find(|language| language.code == code)
}

fn system() -> Option<Language> {
    DesktopLanguageRequester::requested_languages()
        .iter()
        .find_map(|identifier| find(identifier.language.as_str()))
}

macro_rules! fl {
    ($id:literal) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $id)
    };
    ($id:literal, $($name:ident = $value:expr),* $(,)?) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $id, $($name = $value),*)
    };
}

pub(crate) use fl;
