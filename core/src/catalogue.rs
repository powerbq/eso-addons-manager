use std::cmp::Ordering;
use std::collections::HashMap;

use crate::api::{CategoryEntry, FileDetails, FileListEntry, v4};
use crate::config::Config;

const DISCONTINUED_CATEGORY: &str = "157";

#[derive(Debug, Clone)]
pub struct Addon {
    pub uid: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub category_id: String,
    pub info_url: String,
    pub updated_ms: u64,
    pub downloads: u64,
    pub monthly_downloads: u64,
    pub favourites: u64,
    pub directories: Vec<String>,
    pub screenshots: Vec<Screenshot>,
    name_key: String,
    author_key: String,
    directory_keys: Vec<String>,
}

impl From<FileListEntry> for Addon {
    fn from(entry: FileListEntry) -> Self {
        Self {
            name_key: entry.name.to_lowercase(),
            author_key: entry.author.to_lowercase(),
            directory_keys: entry
                .directories
                .iter()
                .map(|directory| directory.to_lowercase())
                .collect(),
            uid: entry.uid,
            name: entry.name,
            author: entry.author,
            version: entry.version,
            category_id: entry.category_id,
            info_url: entry.info_url,
            updated_ms: entry.date,
            downloads: entry.downloads,
            monthly_downloads: entry.monthly_downloads,
            favourites: entry.favourites,
            directories: entry.directories,
            screenshots: entry
                .screenshots
                .into_iter()
                .zip(entry.thumbnails)
                .map(|(full, thumbnail)| Screenshot { thumbnail, full })
                .collect(),
        }
    }
}

impl From<v4::FileListEntry> for Addon {
    fn from(entry: v4::FileListEntry) -> Self {
        let directories = top_level_dirs(entry.addons);

        Self {
            name_key: entry.title.to_lowercase(),
            author_key: entry.author.to_lowercase(),
            directory_keys: directories
                .iter()
                .map(|directory| directory.to_lowercase())
                .collect(),
            uid: entry.id,
            name: entry.title,
            author: entry.author,
            version: entry.version,
            category_id: entry.category_id,
            info_url: entry.file_info_uri,
            updated_ms: entry.last_update,
            downloads: entry.downloads,
            monthly_downloads: entry.downloads_monthly,
            favourites: entry.favorites,
            directories,
            screenshots: Vec::new(),
        }
    }
}

fn top_level_dirs(addons: Vec<v4::AddonEntry>) -> Vec<String> {
    let mut directories = Vec::new();
    for addon in addons {
        let top = addon.path.split('/').next().unwrap_or(&addon.path);
        if !top.is_empty() && !directories.iter().any(|existing| existing == top) {
            directories.push(top.to_owned());
        }
    }

    directories
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screenshot {
    pub thumbnail: String,
    pub full: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Details {
    pub version: String,
    pub md5: String,
    pub download_url: String,
    pub description: String,
    pub changelog: String,
    pub screenshots: Vec<Screenshot>,
}

impl From<FileDetails> for Details {
    fn from(details: FileDetails) -> Self {
        Self {
            version: details.version,
            md5: details.md5,
            download_url: details.download_url,
            description: details.description,
            changelog: details.changelog,
            screenshots: Vec::new(),
        }
    }
}

impl From<v4::FileDetails> for Details {
    fn from(details: v4::FileDetails) -> Self {
        Self {
            version: details.version,
            md5: details.checksum,
            download_url: details.download_uri,
            description: details.description,
            changelog: details.change_log,
            screenshots: details
                .images
                .into_iter()
                .map(|image| Screenshot {
                    thumbnail: image.thumb_url,
                    full: image.image_url,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: String,
    pub title: String,
    pub icon_url: String,
    pub parent_ids: Vec<String>,
}

impl From<CategoryEntry> for Category {
    fn from(entry: CategoryEntry) -> Self {
        Self {
            id: entry.id,
            title: entry.title.trim().to_owned(),
            icon_url: entry.icon_url,
            parent_ids: entry.parent_ids,
        }
    }
}

impl From<v4::CategoryEntry> for Category {
    fn from(entry: v4::CategoryEntry) -> Self {
        Self {
            id: entry.id,
            title: entry.title.trim().to_owned(),
            icon_url: entry.icon_url,
            parent_ids: entry.parent_ids,
        }
    }
}

impl Category {
    fn group_key(&self) -> Vec<u64> {
        self.parent_ids
            .iter()
            .map(|id| id.trim().parse().unwrap_or(u64::MAX))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    Name,
    Author,
    Updated,
    Downloads,
    #[default]
    MonthlyDownloads,
    Favourites,
}

impl SortOrder {
    pub const ALL: [SortOrder; 6] = [
        SortOrder::Name,
        SortOrder::Author,
        SortOrder::Updated,
        SortOrder::Downloads,
        SortOrder::MonthlyDownloads,
        SortOrder::Favourites,
    ];

    pub fn is_ascending(self) -> bool {
        matches!(self, SortOrder::Name | SortOrder::Author)
    }

    pub fn as_key(self) -> &'static str {
        match self {
            SortOrder::Name => "Name",
            SortOrder::Author => "Author",
            SortOrder::Updated => "Updated",
            SortOrder::Downloads => "Downloads",
            SortOrder::MonthlyDownloads => "MonthlyDownloads",
            SortOrder::Favourites => "Favourites",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|order| order.as_key() == key)
    }

    fn compare(self, a: &Addon, b: &Addon) -> Ordering {
        match self {
            SortOrder::Name => a.name_key.cmp(&b.name_key),
            SortOrder::Author => a.author_key.cmp(&b.author_key),
            SortOrder::Downloads => b.downloads.cmp(&a.downloads),
            SortOrder::MonthlyDownloads => b.monthly_downloads.cmp(&a.monthly_downloads),
            SortOrder::Favourites => b.favourites.cmp(&a.favourites),
            SortOrder::Updated => b.updated_ms.cmp(&a.updated_ms),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    Any,
    Only,
    Except,
}

impl Filter {
    pub fn next(self) -> Self {
        match self {
            Self::Any => Self::Only,
            Self::Only => Self::Except,
            Self::Except => Self::Any,
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            Self::Any => "Any",
            Self::Only => "Only",
            Self::Except => "Except",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "Any" => Some(Self::Any),
            "Only" => Some(Self::Only),
            "Except" => Some(Self::Except),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub sort: SortOrder,
    pub reversed: bool,
    pub installed: Filter,
    pub favourites: Filter,
}

#[derive(Debug, Clone, Default)]
pub struct Catalogue {
    addons: Vec<Addon>,
    categories: Vec<Category>,
}

impl Catalogue {
    pub fn new(
        addons: impl IntoIterator<Item = Addon>,
        categories: impl IntoIterator<Item = Category>,
    ) -> Self {
        let addons = addons.into_iter().collect();

        let mut categories: Vec<Category> = categories.into_iter().collect();
        categories
            .sort_by_cached_key(|category| (category.group_key(), category.title.to_lowercase()));

        Self { addons, categories }
    }

    pub fn with_addons(&self, addons: impl IntoIterator<Item = Addon>) -> Self {
        Self {
            addons: addons.into_iter().collect(),
            categories: self.categories.clone(),
        }
    }

    pub fn len(&self) -> usize {
        self.addons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.addons.is_empty()
    }

    pub fn addons(&self) -> &[Addon] {
        &self.addons
    }

    pub fn providers(&self) -> HashMap<&str, Vec<&Addon>> {
        let mut providers: HashMap<&str, Vec<&Addon>> = HashMap::new();
        for addon in &self.addons {
            for directory in &addon.directories {
                providers.entry(directory).or_default().push(addon);
            }
        }
        for candidates in providers.values_mut() {
            if candidates
                .iter()
                .any(|addon| addon.category_id != DISCONTINUED_CATEGORY)
            {
                candidates.retain(|addon| addon.category_id != DISCONTINUED_CATEGORY);
            }
        }
        providers
    }

    pub fn addon(&self, index: usize) -> Option<&Addon> {
        self.addons.get(index)
    }

    pub fn position(&self, uid: &str) -> Option<usize> {
        self.addons.iter().position(|addon| addon.uid == uid)
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub fn category(&self, id: &str) -> Option<&Category> {
        self.categories.iter().find(|category| category.id == id)
    }

    pub fn search(&self, query: &Query, config: &Config) -> Vec<usize> {
        let text = query.text.trim().to_lowercase();

        let mut matches: Vec<usize> = self
            .addons
            .iter()
            .enumerate()
            .filter(|(_, addon)| {
                text.is_empty()
                    || addon.name_key.contains(&text)
                    || addon.author_key.contains(&text)
                    || addon
                        .directory_keys
                        .iter()
                        .any(|directory| directory.contains(&text))
            })
            .filter(|(_, addon)| {
                let filters = [
                    (query.installed, config.addons.contains_key(&addon.uid)),
                    (query.favourites, config.favourites.contains_key(&addon.uid)),
                ];
                let mut included = filters
                    .iter()
                    .filter(|(filter, _)| *filter == Filter::Only)
                    .peekable();
                let wanted = included.peek().is_none() || included.any(|(_, member)| *member);
                let excluded = filters
                    .iter()
                    .any(|(filter, member)| *filter == Filter::Except && *member);

                wanted && !excluded
            })
            .map(|(index, _)| index)
            .collect();

        matches.sort_by(|&a, &b| {
            let (a, b) = (&self.addons[a], &self.addons[b]);

            let ordering = query.sort.compare(a, b);
            let ordering = if query.reversed {
                ordering.reverse()
            } else {
                ordering
            };

            ordering
                .then_with(|| a.name_key.cmp(&b.name_key))
                .then_with(|| a.uid.cmp(&b.uid))
        });

        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(uid: &str, name: &str, author: &str, category: &str, stats: [u64; 4]) -> Addon {
        file_entry(uid, name, author, category, stats).into()
    }

    fn file_entry(
        uid: &str,
        name: &str,
        author: &str,
        category: &str,
        stats: [u64; 4],
    ) -> FileListEntry {
        let [downloads, monthly_downloads, favourites, date] = stats;

        FileListEntry {
            uid: uid.into(),
            name: name.into(),
            author: author.into(),
            version: String::new(),
            category_id: category.into(),
            info_url: String::new(),
            date,
            downloads,
            monthly_downloads,
            favourites,
            directories: Vec::new(),
            thumbnails: Vec::new(),
            screenshots: Vec::new(),
        }
    }

    fn catalogue() -> Catalogue {
        Catalogue::new(
            [
                entry(
                    "1",
                    "LibAddonMenu",
                    "sirinsidiator",
                    "53",
                    [600, 20, 30, 300],
                ),
                entry(
                    "2",
                    "Tamriel Trade Centre",
                    "cyxui",
                    "94",
                    [500, 10, 40, 200],
                ),
                entry("3", "HarvestMap", "Shinni", "94", [700, 30, 10, 100]),
                entry("4", "Azurah", "Phinix", "17", [100, 5, 5, 400]),
            ],
            [
                category("94", "Trade", &["0", "23"]),
                category("17", "Graphic UI Mods", &["0", "23"]),
                category("53", "Libraries", &["0", "144"]),
                category("33", " Plug-Ins & Patches", &["0", "154"]),
                category("56", "Templar", &["0", "23", "39"]),
                category("149", "DPS", &["0", "23", "39"]),
            ],
        )
    }

    fn category(id: &str, title: &str, parent_ids: &[&str]) -> Category {
        CategoryEntry {
            id: id.into(),
            title: title.into(),
            icon_url: String::new(),
            parent_ids: parent_ids.iter().map(|id| (*id).to_owned()).collect(),
        }
        .into()
    }

    fn uids(catalogue: &Catalogue, query: &Query) -> Vec<String> {
        uids_with(catalogue, query, &Config::default())
    }

    fn uids_with(catalogue: &Catalogue, query: &Query, config: &Config) -> Vec<String> {
        catalogue
            .search(query, config)
            .into_iter()
            .map(|index| catalogue.addon(index).unwrap().uid.clone())
            .collect()
    }

    #[test]
    fn replaces_addons_and_keeps_categories() {
        let refreshed = catalogue().with_addons([
            entry("5", "Votan's Minimap", "votan", "94", [0; 4]),
            entry("1", "LibAddonMenu", "sirinsidiator", "53", [0; 4]),
        ]);

        assert_eq!(refreshed.len(), 2);
        assert_eq!(refreshed.position("1"), Some(1));
        assert_eq!(refreshed.position("2"), None);
        assert_eq!(refreshed.categories(), catalogue().categories());
    }

    #[test]
    fn default_sort_is_monthly_downloads_descending() {
        assert_eq!(uids(&catalogue(), &Query::default()), ["3", "1", "2", "4"]);
    }

    #[test]
    fn sorts_by_each_order() {
        let catalogue = catalogue();
        let sorted = |sort| {
            uids(
                &catalogue,
                &Query {
                    sort,
                    ..Query::default()
                },
            )
        };

        assert_eq!(sorted(SortOrder::Name), ["4", "3", "1", "2"]);
        assert_eq!(sorted(SortOrder::Author), ["2", "4", "3", "1"]);
        assert_eq!(sorted(SortOrder::Downloads), ["3", "1", "2", "4"]);
        assert_eq!(sorted(SortOrder::Favourites), ["2", "1", "3", "4"]);
        assert_eq!(sorted(SortOrder::Updated), ["4", "1", "2", "3"]);
    }

    #[test]
    fn reverses_the_sort_order() {
        let catalogue = catalogue();
        let reversed = |sort| {
            uids(
                &catalogue,
                &Query {
                    sort,
                    reversed: true,
                    ..Query::default()
                },
            )
        };

        assert_eq!(reversed(SortOrder::Name), ["2", "1", "3", "4"]);
        assert_eq!(reversed(SortOrder::Downloads), ["4", "2", "1", "3"]);
    }

    #[test]
    fn searches_name_and_author_case_insensitively() {
        let catalogue = catalogue();
        let search = |text: &str| {
            uids(
                &catalogue,
                &Query {
                    text: text.into(),
                    ..Query::default()
                },
            )
        };

        assert_eq!(search("MAP"), ["3"]);
        assert_eq!(search("phinix"), ["4"]);
        assert_eq!(search("  lib  "), ["1"]);
        assert!(search("nothing").is_empty());
    }

    #[test]
    fn searches_folders_too() {
        let catalogue = Catalogue::new(
            [
                Addon::from(FileListEntry {
                    directories: vec!["ProvisionsTeamFormation".into(), "LibStub".into()],
                    ..file_entry("1", "Teammate Radar", "Provision", "1", [0; 4])
                }),
                entry("2", "Other", "Someone", "1", [0; 4]),
            ],
            [],
        );
        let query = Query {
            text: "libstub".into(),
            ..Query::default()
        };

        assert_eq!(uids(&catalogue, &query), ["1"]);
    }

    #[test]
    fn discontinued_is_a_last_resort_provider() {
        let current = Addon::from(FileListEntry {
            directories: vec!["SuperStar".into()],
            ..file_entry("3648", "SuperStar", "author", "33", [0; 4])
        });
        let discontinued = Addon::from(FileListEntry {
            directories: vec!["SuperStar".into()],
            ..file_entry("2961", "SuperStar", "author", "157", [0; 4])
        });
        let orphan = Addon::from(FileListEntry {
            directories: vec!["OldLib".into()],
            ..file_entry("99", "OldLib", "author", "157", [0; 4])
        });
        let catalogue = Catalogue::new([current, discontinued, orphan], []);

        let providers = catalogue.providers();

        assert_eq!(providers["SuperStar"].len(), 1);
        assert_eq!(providers["SuperStar"][0].uid, "3648");
        assert_eq!(providers["OldLib"].len(), 1);
        assert_eq!(providers["OldLib"][0].uid, "99");
    }

    #[test]
    fn filters_installed_and_favourites() {
        let catalogue = catalogue();
        let config = Config {
            addons: [("1".to_owned(), String::new())].into(),
            favourites: [
                ("4".to_owned(), String::new()),
                ("1".to_owned(), String::new()),
            ]
            .into(),
            ..Config::default()
        };
        let filtered = |installed, favourites| {
            uids_with(
                &catalogue,
                &Query {
                    installed,
                    favourites,
                    ..Query::default()
                },
                &config,
            )
        };

        use Filter::{Any, Except, Only};

        assert_eq!(filtered(Any, Any), ["3", "1", "2", "4"]);
        assert_eq!(filtered(Only, Any), ["1"]);
        assert_eq!(filtered(Any, Only), ["1", "4"]);
        assert_eq!(filtered(Only, Only), ["1", "4"]);
        assert_eq!(filtered(Except, Any), ["3", "2", "4"]);
        assert_eq!(filtered(Any, Except), ["3", "2"]);
        assert_eq!(filtered(Except, Only), ["4"]);
        assert_eq!(filtered(Only, Except), Vec::<&str>::new());
        assert_eq!(filtered(Except, Except), ["3", "2"]);
    }

    #[test]
    fn categories_are_sorted_by_group_id_then_title() {
        let titles: Vec<_> = catalogue()
            .categories()
            .iter()
            .map(|c| c.title.clone())
            .collect();

        assert_eq!(
            titles,
            [
                "Graphic UI Mods",
                "Trade",
                "DPS",
                "Templar",
                "Libraries",
                "Plug-Ins & Patches"
            ]
        );
    }
}
