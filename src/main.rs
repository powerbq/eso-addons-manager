#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod choice;
mod format;
mod html;
mod i18n;
mod icon;
mod log;
mod menu;
mod migrate;
mod modal;
mod paths;
mod report;
mod rotate;
mod screen;
mod support;
mod sync;
mod update;
mod widget;

use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use eso_addons_manager_core::clean as core_clean;
use eso_addons_manager_core::sync as core_sync;
use eso_addons_manager_core::{Config, api, cache, deps, scan, ttc};
use iced::futures::StreamExt;
use iced::futures::channel::mpsc;
use iced::theme::Palette;
use iced::theme::palette::{self, Extended, Pair, Secondary};
use iced::widget::{
    button, center, checkbox, column, container, mouse_area, opaque, radio, row, rule, scrollable,
    sensor, space, stack, text, text_editor, text_input, tooltip,
};
use iced::{
    Center, Element, Fill, Font, Padding, Right, Settings, Size, Subscription, Task, Theme, color,
    font, keyboard, mouse, window,
};

use crate::i18n::fl;
use crate::menu::{MENUS, Menu};
use crate::modal::{backdrop, dialog_style};
use crate::report::{Line, Report};
use crate::screen::log::Log;
use crate::screen::{Screen, catalogue, exclusions, libraries};
use crate::widget::{
    BAR_BUTTON_PADDING, BODY_TEXT_SIZE, SPACING_L, SPACING_M, SPACING_M_NUDGE, SPACING_S_NUDGE,
    SPACING_XL, SPACING_XXS, SURFACE_PADDING, button_style, open_url, secondary, single_line,
    spinner, surface,
};

const LOG_CHUNK: usize = 256;
const DIALOG_MAX_WIDTH: f32 = 560.0;
const BAR_SPINNER_SIZE: f32 = SPACING_L;
const BAR_CONTENT_HEIGHT: f32 = 36.0;
const BAR_VERTICAL_PADDING: f32 = SPACING_M_NUDGE;
const BAR_OVERLAP_GAP: f32 = SPACING_M_NUDGE;
const MENU_SPACING: f32 = SPACING_XXS;
const MENU_ICON_SIZE: f32 = SPACING_XL;
const MENU_SPINNER_SIZE: f32 = MENU_ICON_SIZE;
const REPORT_HEIGHT_RATIO: f32 = 0.5;
const DIALOG_SPACING: f32 = SPACING_M;

const PALETTE: Palette = Palette {
    background: color!(0x1e1e1e),
    text: color!(0xe5e5e5),
    ..Palette::DARK
};
const SECONDARY_TEXT_MIX: f32 = 0.35;
const SECONDARY_TEXT_STEP: f32 = 0.05;

fn theme() -> Theme {
    Theme::custom_with_fn("Dark", PALETTE, |palette| {
        let shade = |factor| {
            Pair::new(
                palette::mix(palette.background, palette.text, factor),
                palette.text,
            )
        };

        Extended {
            secondary: Secondary {
                base: shade(SECONDARY_TEXT_MIX),
                weak: shade(SECONDARY_TEXT_MIX - SECONDARY_TEXT_STEP),
                strong: shade(SECONDARY_TEXT_MIX + SECONDARY_TEXT_STEP),
            },
            ..Extended::generate(palette)
        }
    })
}

pub fn main() -> iced::Result {
    let paths = paths::Paths::resolve();
    let (receiver, file_log) = log::setup(paths.log_dir.clone());
    paths.ensure();

    // Single instance: hold an exclusive advisory lock for the whole run. A second launch finds it
    // held and exits instead of opening another window. The OS releases the lock when the process
    // ends (even on a crash), so there is no stale state to clean up. Kept alive until `main`
    // returns, which is when the app exits.
    let _instance_lock = match acquire_instance_lock(&paths) {
        Instance::Busy => {
            tracing::info!("another instance is already running; exiting");
            eprintln!("ESO Addons Manager is already running.");
            return Ok(());
        }
        Instance::Primary(lock) => lock,
    };

    let conflict = match migrate::run(&paths) {
        migrate::Outcome::Nothing => None,
        migrate::Outcome::Migrated => {
            tracing::info!("migration complete");
            None
        }
        migrate::Outcome::Conflict(conflict) => Some(conflict),
    };

    migrate::migrate_index(&paths.config_file, &paths.cache_dir.join(cache::DIRECTORY));

    core_clean::retain(&paths.cache_dir, |path| {
        matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some(cache::DIRECTORY | ttc::DIRECTORY)
        )
    })
    .unwrap_or_else(|error| tracing::warn!("cannot clean the cache directory: {error}"));
    core_clean::retain(&paths.cache_dir.join(ttc::DIRECTORY), ttc::is_cache_file)
        .unwrap_or_else(|error| tracing::warn!("cannot clean the TTC cache: {error}"));
    core_clean::retain(&paths.log_dir, log::is_log)
        .unwrap_or_else(|error| tracing::warn!("cannot clean the log directory: {error}"));
    if let Some(run_dir) = paths.lock_file.parent() {
        let lock_name = paths.lock_file.file_name();
        core_clean::retain(run_dir, |path| path.file_name() == lock_name)
            .unwrap_or_else(|error| tracing::warn!("cannot clean the run directory: {error}"));
    }

    // The updater launches the manager with `--no-update-check`: it has already applied the channel's
    // update, so the in-app check is redundant and the whole update section is switched off.
    let update_ui = !std::env::args().any(|arg| arg == "--no-update-check");
    if !update_ui {
        tracing::info!("launched with --no-update-check: in-app update checks are off");
    }

    let records = Mutex::new(Some(receiver));
    let conflict = Mutex::new(conflict);

    iced::application(
        move || {
            Manager::new(
                records.lock().ok().and_then(|mut records| records.take()),
                file_log.clone(),
                paths.clone(),
                conflict
                    .lock()
                    .ok()
                    .and_then(|mut conflict| conflict.take()),
                update_ui,
            )
        },
        Manager::update,
        Manager::view,
    )
    .title("ESO Addons Manager")
    .theme(theme())
    .settings(Settings {
        default_text_size: BODY_TEXT_SIZE.into(),
        ..Settings::default()
    })
    .font(icon::FONT)
    .window(window::Settings {
        maximized: true,
        exit_on_close_request: false,
        ..window::Settings::default()
    })
    .subscription(Manager::subscription)
    .run()
}

struct Manager {
    config: Config,
    index: cache::Index,
    screen: Screen,
    dialog: Option<Dialog>,
    confirm_open: Option<String>,
    report_fallback: Option<(Vec<u8>, bool)>,
    activity: Option<Activity>,
    started: Instant,
    frame: Instant,
    sync_since: Option<Instant>,
    sync_on_launch_pending: bool,
    update_ui: bool,
    pending_update: bool,
    update_page: Option<String>,
    synced_addons: BTreeSet<String>,
    recommended: BTreeMap<String, Vec<String>>,
    dependents: BTreeMap<String, Vec<String>>,
    page_height: f32,
    page_width: f32,
    controls_width: f32,
    menu: Option<Menu>,
    menu_widths: BTreeMap<Menu, f32>,
    menu_list_widths: BTreeMap<(Menu, usize), Size>,
    menu_total_widths: BTreeMap<Menu, f32>,
    tab: Tab,
    exclusions: exclusions::Exclusions,
    log: Log,
    file_log: log::FileLog,
    paths: paths::Paths,
    language: i18n::Language,
    scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Catalogue,
    Libraries,
    Exclusions,
    Log,
}

enum Activity {
    Scan,
    Sync {
        toggle: Option<Toggle>,
        on_launch: bool,
    },
}

struct Toggle {
    uid: String,
    previous: Option<String>,
}

enum Dialog {
    ScanPrompt {
        previous_folder: Option<String>,
    },
    Report {
        title: String,
        message: String,
    },
    Details {
        title: String,
        report: Report,
        expanded: Vec<bool>,
    },
    Choose {
        choices: Vec<choice::Choice>,
        then: AfterChoice,
    },
    ConfirmRemoval {
        report: Report,
        expanded: Vec<bool>,
        plan: Box<core_sync::Plan>,
        toggle: Option<Toggle>,
        on_launch: bool,
    },
    ReportProblem {
        selection: support::Selection,
        description: text_editor::Content,
        contact: String,
        sending: bool,
    },
    ReportSaved {
        path: PathBuf,
        after_failed_send: bool,
    },
    ClearLogs {
        choice: ClearChoice,
        writing: bool,
    },
    Migrate(migrate::Conflict),
    ChooseFolder {
        configured: bool,
    },
    UpdateAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ClearChoice {
    #[default]
    Others,
    AllDisable,
}

enum AfterChoice {
    Sync { toggle: Option<Toggle> },
    Scan { scanned: Scanned },
}

#[derive(Debug, Clone)]
struct Scanned {
    folders: Vec<String>,
    custom: BTreeSet<String>,
    addons: Vec<String>,
    choices: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
enum Message {
    Catalogue(catalogue::Message),
    ChangeFolder,
    ChooseFolder,
    OpenFolder,
    FolderChosen(Option<PathBuf>),
    ScanRequested,
    ScanConfirmed,
    FolderRestored,
    DialogClosed,
    FoldersScanned(Result<Scanned, String>),
    SyncRequested,
    Sync(sync::Update),
    Frame(Instant),
    SyncOnLaunchToggled(bool),
    LanguageSelected(i18n::Language),
    LogToFileToggled,
    CheckOnLaunchToggled,
    NotifyPrereleasesToggled,
    CheckUpdatesNow,
    UpdateChecked {
        result: Result<Option<update::Available>, String>,
        manual: bool,
    },
    UpdateOpenPage,
    UpdateDisable,
    UpdatePricesToggled,
    PriceRegionSelected(ttc::Region),
    DiscontinuedToggled,
    OpenLogsFolder,
    OpenConfigFolder,
    OpenCacheFolder,
    OpenCustomFolder,
    ClearLogsRequested,
    ClearLogsChoice(ClearChoice),
    ClearLogsConfirmed,
    SectionToggled(usize),
    PageResized(Size),
    TabSelected(Tab),
    Libraries(libraries::Message),
    Exclusions(exclusions::Message),
    CloseRequested,
    MenuToggled(Menu),
    MenuHovered(Menu),
    MenuClosed,
    MenuMeasured(Menu, Size),
    MenuListMeasured(Menu, usize, Size),
    MenuTotalMeasured(Menu, Size),
    ControlsMeasured(Size),
    ProviderSelected(usize, usize),
    OpenPage(String),
    OpenConfirmed,
    OpenCancelled,
    ProvidersConfirmed,
    RemovalConfirmed,
    ReportProblemRequested,
    ReportConfigToggled(bool),
    ReportConfigBackupsToggled(bool),
    ReportLogsToggled(bool),
    ReportLogsBackupsToggled(bool),
    ReportContactChanged(String),
    ReportDescriptionAction(text_editor::Action),
    ReportSend,
    ReportSave,
    ReportBundled(Vec<u8>),
    ReportSent(support::Outcome),
    ReportSaveChosen(Option<PathBuf>),
    OpenReportFolder,
    MigrateChoice(bool),
    MigrateOpenFolders,
    Logged(Vec<String>),
    Log(crate::screen::log::Message),
    WindowEvent(window::Id, window::Event),
    ScaleChanged(f32),
}

impl Manager {
    fn new(
        records: Option<mpsc::UnboundedReceiver<String>>,
        file_log: log::FileLog,
        paths: paths::Paths,
        conflict: Option<migrate::Conflict>,
        update_ui: bool,
    ) -> (Self, Task<Message>) {
        let logging = records.map_or_else(Task::none, |records| {
            Task::run(records.ready_chunks(LOG_CHUNK), Message::Logged)
        });

        let mut config = match Config::load(&paths.config_file) {
            Ok(config) => {
                file_log.configure(config.log_to_file);
                config
            }
            Err(error) => {
                file_log.configure(true);
                log::banner();
                let language = i18n::select("");
                tracing::error!("cannot load settings: {error}");
                let manager = Self {
                    config: Config::default(),
                    index: cache::Index::default(),
                    screen: Screen::Unavailable(error.to_string()),
                    dialog: None,
                    confirm_open: None,
                    report_fallback: None,
                    activity: None,
                    started: Instant::now(),
                    frame: Instant::now(),
                    sync_since: None,
                    sync_on_launch_pending: false,
                    update_ui,
                    pending_update: false,
                    update_page: None,
                    synced_addons: BTreeSet::new(),
                    recommended: BTreeMap::new(),
                    dependents: BTreeMap::new(),
                    page_height: 0.0,
                    page_width: 0.0,
                    controls_width: 0.0,
                    menu: None,
                    menu_widths: BTreeMap::new(),
                    menu_list_widths: BTreeMap::new(),
                    menu_total_widths: BTreeMap::new(),
                    tab: Tab::Catalogue,
                    exclusions: exclusions::Exclusions::new(
                        &Config::default(),
                        &BTreeSet::new(),
                        false,
                    ),
                    log: Log::default(),
                    file_log,
                    paths,
                    language,
                    scale: 1.0,
                };
                return (manager, logging);
            }
        };

        log::banner();

        // Under `--no-update-check` the update section is gone, so its settings must not linger: stop
        // writing the keys, and strip any left over from a past direct launch.
        if !update_ui {
            config.write_update_keys = false;
            if update_keys_present(&paths.config_file) {
                save(
                    &config,
                    &paths.config_file,
                    "running under --no-update-check: removed update settings",
                );
            }
        }

        let (screen, task) = match api::Client::new() {
            Ok(client) => {
                let (catalogue, task) = catalogue::Catalogue::new(
                    client,
                    api::sources(config.include_outdated),
                    theme(),
                );
                (Screen::Catalogue(catalogue), task.map(Message::Catalogue))
            }
            Err(error) => {
                tracing::error!("cannot create HTTP client: {error}");
                (Screen::Unavailable(error.to_string()), Task::none())
            }
        };

        let language = i18n::select(&config.language);
        let index = cache::Index::load(&paths.cache_dir.join(cache::DIRECTORY));

        let folder = if config.target_directory.trim().is_empty() {
            "(not set)".to_owned()
        } else {
            format!("{:?}", config.target_directory)
        };
        tracing::info!(
            "settings: AddOns folder {folder}, {} installed, {} favourites, {} exclusions, {} library choices, language {}, sync on launch {}",
            config.addons.len(),
            config.favourites.len(),
            config.exclusions.len(),
            config.selected_libraries.len(),
            language.code,
            config.sync_on_launch
        );

        let mut manager = Self {
            language,
            sync_on_launch_pending: config.sync_on_launch,
            update_ui,
            pending_update: false,
            update_page: None,
            synced_addons: config.addons.keys().cloned().collect(),
            recommended: BTreeMap::new(),
            dependents: BTreeMap::new(),
            page_height: 0.0,
            page_width: 0.0,
            controls_width: 0.0,
            menu: None,
            menu_widths: BTreeMap::new(),
            menu_list_widths: BTreeMap::new(),
            menu_total_widths: BTreeMap::new(),
            tab: Tab::Catalogue,
            exclusions: exclusions::Exclusions::new(
                &config,
                &installed_uids(&config, &index),
                config.update_prices,
            ),
            log: Log::default(),
            file_log,
            paths,
            config,
            index,
            screen,
            dialog: None,
            confirm_open: None,
            report_fallback: None,
            activity: None,
            started: Instant::now(),
            frame: Instant::now(),
            sync_since: None,
            scale: 1.0,
        };

        if let Some(conflict) = conflict {
            tracing::warn!(
                "migration conflict needs a choice: settings {}, {} custom file(s)",
                conflict.config,
                conflict.custom.len()
            );
            manager.sync_on_launch_pending = false;
            manager.dialog = Some(Dialog::Migrate(conflict));
        } else if manager.config.target_directory.trim().is_empty() {
            manager.dialog = Some(Dialog::ChooseFolder { configured: false });
        }

        let updates = if manager.update_ui && manager.config.check_for_updates {
            let beta = manager.config.notify_prereleases;
            Task::perform(check_updates(beta), |result| Message::UpdateChecked {
                result,
                manual: false,
            })
        } else {
            Task::none()
        };

        (manager, Task::batch([logging, task, updates]))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Catalogue(message) => {
                let Screen::Catalogue(catalogue) = &mut self.screen else {
                    return Task::none();
                };

                match catalogue.update(message, &self.config, &self.index) {
                    catalogue::Action::None => Task::none(),
                    catalogue::Action::Run(task) => task.map(Message::Catalogue),
                    catalogue::Action::Loaded(task) => {
                        let task = task.map(Message::Catalogue);
                        if !std::mem::take(&mut self.sync_on_launch_pending) {
                            return task;
                        }
                        tracing::info!("sync on launch");
                        Task::batch([task, self.start_sync(None, true, true)])
                    }
                    catalogue::Action::ToggleInstalled { uid, name } => {
                        if self.activity.is_some() {
                            return Task::none();
                        }
                        let previous = self.config.addons.remove(&uid);
                        let reason = if previous.is_none() {
                            self.config.addons.insert(uid.clone(), name.clone());
                            format!("install requested: {name} ({uid})")
                        } else {
                            format!("remove requested: {name} ({uid})")
                        };
                        save(&self.config, &self.paths.config_file, &reason);
                        catalogue.refresh(&self.config, &self.index);
                        self.start_sync(Some(Toggle { uid, previous }), true, false)
                    }
                    catalogue::Action::ToggleFavourite { uid, name } => {
                        let reason = if self.config.favourites.remove(&uid).is_none() {
                            let reason = format!("favourite added: {name} ({uid})");
                            self.config.favourites.insert(uid, name);
                            reason
                        } else {
                            format!("favourite removed: {name} ({uid})")
                        };
                        save(&self.config, &self.paths.config_file, &reason);
                        catalogue.refresh(&self.config, &self.index);
                        Task::none()
                    }
                    catalogue::Action::OpenUrl(url) => {
                        self.confirm_open = Some(url);
                        Task::none()
                    }
                    catalogue::Action::QueryChanged {
                        sort,
                        reversed,
                        installed,
                        favourites,
                        dependencies,
                    } => {
                        self.config.sort = sort;
                        self.config.sort_reversed = reversed;
                        self.config.filter_installed = installed;
                        self.config.filter_favourites = favourites;
                        self.config.filter_dependencies = dependencies;
                        save(
                            &self.config,
                            &self.paths.config_file,
                            "view filters/sort changed",
                        );
                        Task::none()
                    }
                }
            }
            Message::ChangeFolder => {
                self.menu = None;
                let configured = !self.config.target_directory.trim().is_empty();
                self.dialog = Some(Dialog::ChooseFolder { configured });
                Task::none()
            }
            Message::ChooseFolder => {
                self.menu = None;
                self.dialog = None;
                let current = std::path::absolute(&self.config.target_directory)
                    .unwrap_or_else(|_| PathBuf::from(&self.config.target_directory));

                Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_directory(current)
                            .pick_folder()
                            .await
                            .map(|folder| folder.path().to_path_buf())
                    },
                    Message::FolderChosen,
                )
            }
            Message::OpenFolder => {
                self.menu = None;
                let folder = std::path::absolute(&self.config.target_directory)
                    .unwrap_or_else(|_| PathBuf::from(&self.config.target_directory));
                widget::open_folder(&folder);
                Task::none()
            }
            Message::FolderChosen(Some(folder)) => {
                let folder = folder.to_string_lossy().into_owned();
                let previous_folder = std::mem::replace(&mut self.config.target_directory, folder);
                save(
                    &self.config,
                    &self.paths.config_file,
                    &format!("AddOns folder set to {}", self.config.target_directory),
                );

                self.dialog = Some(Dialog::ScanPrompt {
                    previous_folder: Some(previous_folder),
                });
                Task::none()
            }
            Message::FolderChosen(None) => Task::none(),
            Message::ScanRequested => {
                self.menu = None;
                self.dialog = Some(Dialog::ScanPrompt {
                    previous_folder: None,
                });
                Task::none()
            }
            Message::FolderRestored => {
                if let Some(Dialog::ScanPrompt {
                    previous_folder: Some(previous_folder),
                }) = self.dialog.take()
                {
                    self.config.target_directory = previous_folder;
                    save(
                        &self.config,
                        &self.paths.config_file,
                        &format!("AddOns folder restored to {}", self.config.target_directory),
                    );
                }
                Task::none()
            }
            Message::DialogClosed => {
                self.close_dialog();
                if self.pending_update && self.activity.is_none() {
                    self.pending_update = false;
                    self.dialog = Some(Dialog::UpdateAvailable);
                }
                Task::none()
            }
            Message::MenuToggled(menu) => {
                self.menu = (self.menu != Some(menu)).then_some(menu);
                Task::none()
            }
            Message::MenuHovered(menu) => {
                if self.menu.is_some() {
                    self.menu = Some(menu);
                }
                Task::none()
            }
            Message::MenuClosed => {
                self.menu = None;
                Task::none()
            }
            Message::MenuMeasured(menu, size) => {
                self.menu_widths.insert(menu, size.width);
                Task::none()
            }
            Message::MenuListMeasured(menu, column, size) => {
                self.menu_list_widths.insert((menu, column), size);
                Task::none()
            }
            Message::MenuTotalMeasured(menu, size) => {
                self.menu_total_widths.insert(menu, size.width);
                Task::none()
            }
            Message::ControlsMeasured(size) => {
                self.controls_width = size.width;
                Task::none()
            }
            Message::CloseRequested => {
                self.close_dialog();
                iced::exit()
            }
            Message::ScanConfirmed => {
                tracing::info!("scan requested");
                self.dialog = None;
                let Screen::Catalogue(screen) = &self.screen else {
                    return Task::none();
                };
                let Some(catalogue) = screen.catalogue() else {
                    return Task::none();
                };
                let catalogue = catalogue.clone();
                let client = screen.client().clone();
                self.activity = Some(Activity::Scan);
                let target = PathBuf::from(&self.config.target_directory);
                let custom_dir = self.paths.custom_dir.clone();
                let cache = self.paths.cache_dir.join(cache::DIRECTORY);
                let config = self.config.clone();
                let index = self.index.clone();

                Task::perform(
                    async move {
                        let handle = tokio::runtime::Handle::current();
                        tokio::task::spawn_blocking(move || {
                            let folders = scan::folders(&target).map_err(|e| e.to_string())?;
                            let custom = scan::custom_folders(&custom_dir);

                            let mut manifests: BTreeMap<String, Option<Vec<String>>> =
                                BTreeMap::new();
                            let found =
                                scan::new_addons(&folders, &catalogue, &config, &index, |addon| {
                                    if let Some(cached) = manifests.get(&addon.uid) {
                                        return cached.clone();
                                    }
                                    let path = cache::archive_path(&cache, addon);
                                    let provides =
                                        if cache::is_valid(&path, addon, index.get(&addon.uid)) {
                                            deps::read_archive(&path).ok().map(|m| m.provides)
                                        } else {
                                            let version =
                                                catalogue.origin_of(&addon.uid).unwrap_or_default();
                                            match handle.block_on(cache::fetch(
                                                &client, version, &addon.uid, &path,
                                            )) {
                                                Ok(download) => {
                                                    let _ = cache::store(&path, &download);
                                                    deps::read_archive(&path)
                                                        .ok()
                                                        .map(|m| m.provides)
                                                }
                                                Err(error) => {
                                                    tracing::debug!(
                                                        "cannot inspect {} for scan: {error}",
                                                        addon.uid
                                                    );
                                                    None
                                                }
                                            }
                                        };
                                    manifests.insert(addon.uid.clone(), provides.clone());
                                    provides
                                });

                            let addons =
                                found.addons.iter().map(|addon| addon.uid.clone()).collect();
                            let choices = found
                                .choices
                                .iter()
                                .map(|(folder, candidates)| {
                                    let uids =
                                        candidates.iter().map(|addon| addon.uid.clone()).collect();
                                    (folder.clone(), uids)
                                })
                                .collect();

                            Ok::<Scanned, String>(Scanned {
                                folders,
                                custom,
                                addons,
                                choices,
                            })
                        })
                        .await
                        .map_err(|error| error.to_string())?
                    },
                    Message::FoldersScanned,
                )
            }
            Message::FoldersScanned(result) => {
                self.activity = None;
                self.dialog = Some(match result {
                    Ok(scanned) if scanned.choices.is_empty() => self.complete_scan(&scanned, &[]),
                    Ok(scanned) => {
                        let choices = scanned.choices.clone();
                        self.choose_providers(&choices, AfterChoice::Scan { scanned })
                    }
                    Err(error) => {
                        tracing::error!("cannot scan the AddOns folder: {error}");
                        Dialog::Report {
                            title: fl!("scan-failed"),
                            message: error,
                        }
                    }
                });
                Task::none()
            }
            Message::SyncRequested => {
                tracing::info!("sync requested");
                self.start_sync(None, true, false)
            }
            Message::LanguageSelected(language) => {
                self.menu = None;
                self.menu_list_widths.clear();
                self.menu_total_widths.clear();
                self.config.language = language.code.to_owned();
                save(
                    &self.config,
                    &self.paths.config_file,
                    &format!("language changed to {}", language.code),
                );
                self.language = i18n::select(language.code);
                Task::none()
            }
            Message::LogToFileToggled => {
                let enabled = !self.config.log_to_file;
                self.config.log_to_file = enabled;
                if enabled {
                    self.file_log.set_enabled(true, &self.log.text());
                    save(
                        &self.config,
                        &self.paths.config_file,
                        "file logging enabled",
                    );
                } else {
                    save(
                        &self.config,
                        &self.paths.config_file,
                        "file logging disabled",
                    );
                    self.file_log.set_enabled(false, "");
                }
                Task::none()
            }
            Message::CheckOnLaunchToggled => {
                self.config.check_for_updates = !self.config.check_for_updates;
                let reason = if self.config.check_for_updates {
                    "update check on launch enabled"
                } else {
                    "update check on launch disabled"
                };
                save(&self.config, &self.paths.config_file, reason);
                Task::none()
            }
            Message::NotifyPrereleasesToggled => {
                self.config.notify_prereleases = !self.config.notify_prereleases;
                let reason = if self.config.notify_prereleases {
                    "prerelease notifications enabled"
                } else {
                    "prerelease notifications disabled"
                };
                save(&self.config, &self.paths.config_file, reason);
                Task::none()
            }
            Message::CheckUpdatesNow => {
                self.menu = None;
                tracing::info!("checking for updates");
                let beta = self.config.notify_prereleases;
                Task::perform(check_updates(beta), |result| Message::UpdateChecked {
                    result,
                    manual: true,
                })
            }
            Message::UpdateChecked { result, manual } => {
                match result {
                    Ok(Some(available)) => {
                        tracing::info!("update available: {}", available.version);
                        self.update_page = Some(available.page);
                        if self.sync_on_launch_pending
                            || self.activity.is_some()
                            || self.dialog.is_some()
                        {
                            self.pending_update = true;
                        } else {
                            self.dialog = Some(Dialog::UpdateAvailable);
                        }
                    }
                    Ok(None) => {
                        tracing::info!("no update available");
                        if manual && self.dialog.is_none() {
                            self.dialog = Some(Dialog::Report {
                                title: fl!("update-title"),
                                message: fl!("update-up-to-date"),
                            });
                        }
                    }
                    Err(error) => {
                        tracing::warn!("update check failed: {error}");
                        if manual && self.dialog.is_none() {
                            self.dialog = Some(Dialog::Report {
                                title: fl!("update-title"),
                                message: fl!("update-check-failed"),
                            });
                        }
                    }
                }
                Task::none()
            }
            Message::UpdateOpenPage => {
                self.dialog = None;
                self.confirm_open = self
                    .update_page
                    .clone()
                    .or_else(|| Some(update::stable_page()));
                Task::none()
            }
            Message::UpdateDisable => {
                self.dialog = None;
                self.config.check_for_updates = false;
                save(
                    &self.config,
                    &self.paths.config_file,
                    "update check disabled from dialog",
                );
                Task::none()
            }
            Message::UpdatePricesToggled => {
                self.config.update_prices = !self.config.update_prices;
                let reason = if self.config.update_prices {
                    "TTC price updates enabled"
                } else {
                    "TTC price updates disabled"
                };
                save(&self.config, &self.paths.config_file, reason);
                self.exclusions.refresh(
                    &installed_uids(&self.config, &self.index),
                    self.config.update_prices,
                );
                Task::none()
            }
            Message::PriceRegionSelected(region) => {
                self.config.price_region = region;
                save(
                    &self.config,
                    &self.paths.config_file,
                    "TTC price region changed",
                );
                Task::none()
            }
            Message::DiscontinuedToggled => {
                self.menu = None;
                self.config.include_outdated = !self.config.include_outdated;
                save(
                    &self.config,
                    &self.paths.config_file,
                    &format!(
                        "outdated add-ons {}",
                        if self.config.include_outdated {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    ),
                );

                match api::Client::new() {
                    Ok(client) => {
                        let (catalogue, task) = catalogue::Catalogue::new(
                            client,
                            api::sources(self.config.include_outdated),
                            theme(),
                        );
                        self.screen = Screen::Catalogue(catalogue);
                        task.map(Message::Catalogue)
                    }
                    Err(error) => {
                        tracing::error!("cannot create HTTP client: {error}");
                        self.screen = Screen::Unavailable(error.to_string());
                        Task::none()
                    }
                }
            }
            Message::OpenLogsFolder => {
                self.menu = None;
                let folder = &self.paths.log_dir;
                let _ = std::fs::create_dir_all(folder);
                widget::open_folder(folder);
                Task::none()
            }
            Message::OpenConfigFolder => {
                self.menu = None;
                let folder = self
                    .paths
                    .config_file
                    .parent()
                    .unwrap_or(&self.paths.config_file);
                let _ = std::fs::create_dir_all(folder);
                widget::open_folder(folder);
                Task::none()
            }
            Message::OpenCacheFolder => {
                self.menu = None;
                let folder = &self.paths.cache_dir;
                let _ = std::fs::create_dir_all(folder);
                widget::open_folder(folder);
                Task::none()
            }
            Message::OpenCustomFolder => {
                self.menu = None;
                let folder = &self.paths.custom_dir;
                let _ = std::fs::create_dir_all(folder);
                widget::open_folder(folder);
                Task::none()
            }
            Message::ClearLogsRequested => {
                self.menu = None;
                self.dialog = Some(Dialog::ClearLogs {
                    choice: ClearChoice::default(),
                    writing: self.config.log_to_file,
                });
                Task::none()
            }
            Message::ClearLogsChoice(choice) => {
                if let Some(Dialog::ClearLogs {
                    choice: current, ..
                }) = &mut self.dialog
                {
                    *current = choice;
                }
                Task::none()
            }
            Message::ClearLogsConfirmed => {
                if let Some(Dialog::ClearLogs { choice, .. }) = &self.dialog {
                    let choice = *choice;
                    self.dialog = None;
                    match choice {
                        ClearChoice::AllDisable => {
                            self.file_log.clear_all_and_disable();
                            self.config.log_to_file = false;
                            save(
                                &self.config,
                                &self.paths.config_file,
                                "log files deleted, file logging disabled",
                            );
                        }
                        ClearChoice::Others => {
                            self.file_log.clear_others();
                            tracing::info!("old log files deleted");
                        }
                    }
                }
                Task::none()
            }
            Message::SyncOnLaunchToggled(enabled) => {
                self.config.sync_on_launch = enabled;
                save(
                    &self.config,
                    &self.paths.config_file,
                    &format!("sync on launch set to {enabled}"),
                );
                Task::none()
            }
            Message::SectionToggled(index) => {
                if let Some(
                    Dialog::Details { expanded, .. } | Dialog::ConfirmRemoval { expanded, .. },
                ) = &mut self.dialog
                    && let Some(expanded) = expanded.get_mut(index)
                {
                    *expanded = !*expanded;
                }
                Task::none()
            }
            Message::TabSelected(tab) => {
                self.tab = tab;
                self.exclusions.refresh(
                    &installed_uids(&self.config, &self.index),
                    self.config.update_prices,
                );
                Task::none()
            }
            Message::Libraries(libraries::Message::Selected { directory, uid }) => {
                let reason = format!("choice of {directory} selected: {uid}");
                self.config.selected_libraries.insert(directory, uid);
                save(&self.config, &self.paths.config_file, &reason);
                Task::none()
            }
            Message::Libraries(libraries::Message::Reset(directory)) => {
                let reason = format!("choice of {directory} reset");
                self.config.selected_libraries.remove(&directory);
                save(&self.config, &self.paths.config_file, &reason);
                Task::none()
            }
            Message::Libraries(libraries::Message::OpenPage(url)) | Message::OpenPage(url) => {
                self.confirm_open = Some(url);
                Task::none()
            }
            Message::OpenConfirmed => {
                if let Some(url) = self.confirm_open.take() {
                    open_url(&url);
                }
                Task::none()
            }
            Message::OpenCancelled => {
                self.confirm_open = None;
                Task::none()
            }
            Message::Exclusions(message) => {
                if let exclusions::Action::Save(patterns) = self.exclusions.update(message) {
                    let reason = format!("exclusions updated: {} patterns", patterns.len());
                    self.config.exclusions = patterns;
                    save(&self.config, &self.paths.config_file, &reason);
                }
                Task::none()
            }
            Message::ProviderSelected(position, index) => {
                if let Some(Dialog::Choose { choices, .. }) = &mut self.dialog
                    && let Some(choice) = choices.get_mut(position)
                {
                    choice.select(index);
                }
                Task::none()
            }
            Message::ProvidersConfirmed => {
                let Some(Dialog::Choose { choices, then }) = self.dialog.take() else {
                    return Task::none();
                };
                if !choice::is_complete(&choices) {
                    self.dialog = Some(Dialog::Choose { choices, then });
                    return Task::none();
                }

                let chosen: Vec<(String, String)> = choices
                    .iter()
                    .filter_map(|choice| Some((choice.directory.clone(), choice.uid()?.to_owned())))
                    .collect();
                for (directory, uid) in &chosen {
                    tracing::info!("provider of {directory} chosen: {uid}");
                }

                match then {
                    AfterChoice::Sync { toggle } => {
                        let reason = format!("{} provider choices saved", chosen.len());
                        self.config.selected_libraries.extend(chosen);
                        save(&self.config, &self.paths.config_file, &reason);
                        self.start_sync(toggle, false, false)
                    }
                    AfterChoice::Scan { scanned } => {
                        let chosen: Vec<String> = chosen.into_iter().map(|(_, uid)| uid).collect();
                        self.dialog = Some(self.complete_scan(&scanned, &chosen));
                        Task::none()
                    }
                }
            }
            Message::RemovalConfirmed => {
                let Some(Dialog::ConfirmRemoval {
                    plan,
                    toggle,
                    on_launch,
                    ..
                }) = self.dialog.take()
                else {
                    return Task::none();
                };
                tracing::info!(
                    "removal of {} add-on(s) not in the catalogue confirmed",
                    plan.unknown.len()
                );
                self.activity = Some(Activity::Sync { toggle, on_launch });
                self.sync_since = Some(Instant::now());
                sync::apply_plan(*plan, self.config.clone(), self.paths.clone()).map(Message::Sync)
            }
            Message::ReportProblemRequested => {
                self.menu = None;
                self.dialog = Some(Dialog::ReportProblem {
                    selection: support::Selection {
                        config: true,
                        config_backups: false,
                        log: true,
                        log_backups: false,
                    },
                    description: text_editor::Content::new(),
                    contact: String::new(),
                    sending: false,
                });
                Task::none()
            }
            Message::ReportConfigToggled(value) => {
                if let Some(Dialog::ReportProblem { selection, .. }) = &mut self.dialog {
                    selection.config = value;
                }
                Task::none()
            }
            Message::ReportConfigBackupsToggled(value) => {
                if let Some(Dialog::ReportProblem { selection, .. }) = &mut self.dialog {
                    selection.config_backups = value;
                }
                Task::none()
            }
            Message::ReportLogsToggled(value) => {
                if let Some(Dialog::ReportProblem { selection, .. }) = &mut self.dialog {
                    selection.log = value;
                }
                Task::none()
            }
            Message::ReportLogsBackupsToggled(value) => {
                if let Some(Dialog::ReportProblem { selection, .. }) = &mut self.dialog {
                    selection.log_backups = value;
                }
                Task::none()
            }
            Message::ReportContactChanged(value) => {
                if let Some(Dialog::ReportProblem { contact, .. }) = &mut self.dialog {
                    *contact = value;
                }
                Task::none()
            }
            Message::ReportDescriptionAction(action) => {
                if let Some(Dialog::ReportProblem { description, .. }) = &mut self.dialog {
                    description.perform(action);
                }
                Task::none()
            }
            Message::ReportSend => {
                let Some(Dialog::ReportProblem {
                    selection,
                    description,
                    contact,
                    sending,
                }) = &mut self.dialog
                else {
                    return Task::none();
                };
                if *sending {
                    return Task::none();
                }
                *sending = true;
                let selection = *selection;
                let description = description.text();
                let contact = contact.clone();
                let log = self.log.text();
                let paths = self.paths.clone();
                tracing::info!("sending a problem report");
                Task::perform(
                    support::send(paths, selection, description, contact, log),
                    Message::ReportSent,
                )
            }
            Message::ReportSave => {
                let Some(Dialog::ReportProblem {
                    selection,
                    description,
                    contact,
                    sending,
                }) = &mut self.dialog
                else {
                    return Task::none();
                };
                if *sending {
                    return Task::none();
                }
                *sending = true;
                let selection = *selection;
                let description = description.text();
                let contact = contact.clone();
                let log = self.log.text();
                let paths = self.paths.clone();
                tracing::info!("saving a problem report");
                Task::perform(
                    support::bundle(paths, selection, description, contact, log),
                    Message::ReportBundled,
                )
            }
            Message::ReportBundled(bundle) => {
                self.report_fallback = Some((bundle, false));
                self.dialog = None;
                save_report_dialog()
            }
            Message::ReportSent(outcome) => match outcome {
                support::Outcome::Sent => {
                    self.dialog = Some(Dialog::Report {
                        title: fl!("report-sent-title"),
                        message: fl!("report-sent-message"),
                    });
                    Task::none()
                }
                support::Outcome::Failed { error, bundle } => {
                    tracing::warn!("problem report failed: {error}");
                    if bundle.is_empty() {
                        self.dialog = Some(Dialog::Report {
                            title: fl!("report-failed-title"),
                            message: fl!("report-failed", error = error),
                        });
                        Task::none()
                    } else {
                        self.report_fallback = Some((bundle, true));
                        self.dialog = None;
                        save_report_dialog()
                    }
                }
            },
            Message::ReportSaveChosen(path) => {
                // Cancelling the save dialog is a deliberate "never mind" — dismiss silently (the dialog
                // was already cleared when the picker opened). Only a real write error is surfaced.
                if let (Some(path), Some((bundle, after_failed_send))) =
                    (path, self.report_fallback.take())
                {
                    self.dialog = Some(match std::fs::write(&path, bundle) {
                        Ok(()) => {
                            tracing::info!("problem report saved to {}", path.display());
                            Dialog::ReportSaved {
                                path,
                                after_failed_send,
                            }
                        }
                        Err(error) => {
                            tracing::warn!("could not save the problem report: {error}");
                            Dialog::Report {
                                title: fl!("report-save-failed-title"),
                                message: fl!("report-save-failed", error = error.to_string()),
                            }
                        }
                    });
                }
                Task::none()
            }
            Message::OpenReportFolder => {
                if let Some(Dialog::ReportSaved { path, .. }) = &self.dialog
                    && let Some(folder) = path.parent()
                {
                    widget::open_folder(folder);
                }
                Task::none()
            }
            Message::MigrateChoice(keep_old) => {
                if let Some(Dialog::Migrate(conflict)) = self.dialog.take() {
                    migrate::resolve(&conflict, keep_old, &self.paths);
                    tracing::info!(
                        "migration conflict resolved: kept {}",
                        if keep_old {
                            "the old folder"
                        } else {
                            "the profile"
                        }
                    );
                }
                match Config::load(&self.paths.config_file) {
                    Ok(config) => {
                        self.language = i18n::select(&config.language);
                        self.exclusions = exclusions::Exclusions::new(
                            &config,
                            &installed_uids(&config, &self.index),
                            config.update_prices,
                        );
                        self.synced_addons = config.addons.keys().cloned().collect();
                        self.config = config;
                    }
                    Err(error) => {
                        tracing::error!("cannot reload settings after migration: {error}");
                    }
                }
                Task::none()
            }
            Message::MigrateOpenFolders => {
                if let Some(Dialog::Migrate(conflict)) = &self.dialog {
                    for folder in &conflict.folders {
                        widget::open_folder(folder);
                    }
                }
                Task::none()
            }
            Message::Logged(records) => {
                self.log.push(records);
                Task::none()
            }
            Message::Log(message) => {
                self.log.update(message);
                Task::none()
            }
            Message::PageResized(size) => {
                self.page_height = size.height;
                self.page_width = size.width;
                self.menu = None;
                self.menu_list_widths.clear();
                self.menu_total_widths.clear();
                Task::none()
            }
            Message::Frame(frame) => {
                self.frame = frame;
                Task::none()
            }
            Message::WindowEvent(id, window::Event::Opened { .. }) => {
                window::scale_factor(id).map(Message::ScaleChanged)
            }
            Message::WindowEvent(_, window::Event::Rescaled(scale)) => {
                Task::done(Message::ScaleChanged(scale))
            }
            Message::WindowEvent(_, _) => Task::none(),
            Message::ScaleChanged(scale) => {
                if (scale - self.scale).abs() < f32::EPSILON {
                    return Task::none();
                }

                tracing::info!("display scale factor changed to {scale}");
                self.scale = scale;

                if let Screen::Catalogue(screen) = &mut self.screen {
                    return screen.set_scale(scale).map(Message::Catalogue);
                }

                Task::none()
            }
            Message::Sync(sync::Update::Refreshed(catalogue)) => {
                tracing::info!("catalogue refreshed: {} addons", catalogue.len());
                if let Screen::Catalogue(screen) = &mut self.screen {
                    return screen
                        .replace(catalogue, &self.config, &self.index)
                        .map(Message::Catalogue);
                }
                Task::none()
            }
            Message::Sync(sync::Update::Progress(event)) => {
                tracing::debug!("sync: {event:?}");
                Task::none()
            }
            Message::Sync(sync::Update::Finished(result)) => {
                let (toggle, on_launch) = match self.activity.take() {
                    Some(Activity::Sync { toggle, on_launch }) => (toggle, on_launch),
                    _ => (None, false),
                };
                match result {
                    Ok(outcome) => {
                        let sync::Outcome { plan, applied } = *outcome;
                        if plan.failed.is_empty() && !plan.choices.is_empty() {
                            let _ = self.finish_sync(&plan);
                            self.dialog = Some(
                                self.choose_providers(&plan.choices, AfterChoice::Sync { toggle }),
                            );
                        } else if !applied && !plan.unknown.is_empty() {
                            self.dialog = Some(self.confirm_removal(plan, toggle, on_launch));
                        } else {
                            let report = self.finish_sync(&plan);
                            self.dialog = Some(details(fl!("sync-complete"), report));
                        }
                    }
                    Err(error) => {
                        tracing::error!("sync failed: {error}");
                        if !on_launch {
                            self.dialog = Some(Dialog::Report {
                                title: fl!("sync-failed"),
                                message: error,
                            });
                        }
                    }
                }
                if let Some(since) = self.sync_since.take() {
                    tracing::info!("sync run took {:?}", since.elapsed());
                }
                Task::none()
            }
        }
    }

    fn start_sync(
        &mut self,
        toggle: Option<Toggle>,
        refresh: bool,
        on_launch: bool,
    ) -> Task<Message> {
        if self.config.target_directory.trim().is_empty() {
            tracing::warn!("sync skipped: no AddOns folder selected");
            return Task::none();
        }
        let Screen::Catalogue(screen) = &self.screen else {
            return Task::none();
        };
        let Some(catalogue) = screen.catalogue() else {
            return Task::none();
        };

        self.activity = Some(Activity::Sync { toggle, on_launch });
        self.sync_since = Some(Instant::now());
        sync::start(
            screen.client().clone(),
            screen.versions().to_vec(),
            catalogue.clone(),
            self.config.clone(),
            self.index.clone(),
            self.paths.clone(),
            refresh,
        )
        .map(Message::Sync)
    }

    fn close_dialog(&mut self) {
        match self.dialog.take() {
            Some(Dialog::Choose {
                then:
                    AfterChoice::Sync {
                        toggle: Some(toggle),
                    },
                ..
            }) => self.undo(toggle),
            Some(Dialog::ConfirmRemoval { plan, .. }) => {
                tracing::info!(
                    "sync cancelled: kept {} add-on(s) no longer in the catalogue",
                    plan.unknown.len()
                );
            }
            _ => {}
        }
    }

    fn undo(&mut self, toggle: Toggle) {
        tracing::info!(
            "provider choice cancelled, undoing the change of {}",
            toggle.uid
        );
        match toggle.previous {
            Some(name) => self.config.addons.insert(toggle.uid, name),
            None => self.config.addons.remove(&toggle.uid),
        };
        save(
            &self.config,
            &self.paths.config_file,
            "toggle undone after cancelled provider choice",
        );
        if let Screen::Catalogue(screen) = &mut self.screen {
            screen.refresh(&self.config, &self.index);
        }
        self.exclusions.refresh(
            &installed_uids(&self.config, &self.index),
            self.config.update_prices,
        );
    }

    fn confirm_removal(
        &self,
        plan: core_sync::Plan,
        toggle: Option<Toggle>,
        on_launch: bool,
    ) -> Dialog {
        let name_of = |uid: &str| {
            self.config
                .addons
                .get(uid)
                .filter(|name| !name.is_empty())
                .cloned()
                .unwrap_or_else(|| uid.to_owned())
        };
        // Log the list now, before the sync — so a user who cancels can still send a log that names
        // what would have been removed.
        tracing::info!(
            "{} installed add-on(s) are no longer in the catalogue; awaiting confirmation:",
            plan.unknown.len()
        );
        for uid in &plan.unknown {
            tracing::warn!("not in the catalogue: {} (UID: {uid})", name_of(uid));
        }
        let lines: Vec<Line> = plan
            .unknown
            .iter()
            .map(|uid| Line {
                name: name_of(uid),
                detail: fl!("remove-missing-uid", uid = uid.clone()),
            })
            .collect();
        let mut report = Report::default();
        report.push_section(fl!("remove-missing-section"), lines);
        let expanded = report.expanded();
        Dialog::ConfirmRemoval {
            report,
            expanded,
            plan: Box::new(plan),
            toggle,
            on_launch,
        }
    }

    fn finish_sync(&mut self, plan: &core_sync::Plan) -> Report {
        self.recommended = plan.recommended.clone();
        self.dependents = plan
            .dependents
            .iter()
            .map(|(uid, dependers)| (uid.clone(), dependers.iter().cloned().collect()))
            .collect();
        let Screen::Catalogue(screen) = &mut self.screen else {
            return Report::default();
        };
        let Some(catalogue) = screen.catalogue() else {
            return Report::default();
        };

        let summary = sync::summarise(
            plan,
            &self.config,
            &self.index,
            catalogue,
            &self.synced_addons,
        );
        sync::log(plan, &summary);
        let report = sync::report(&summary);

        core_sync::record(catalogue, &mut self.config);
        core_sync::record_archives(plan, &mut self.index);
        if plan.is_applicable() {
            let pruned = core_sync::clean_config(plan, catalogue, &mut self.config);
            if !pruned.is_empty() {
                backup::before_auto_removal(&self.paths.config_file, &pruned);
            }
            core_sync::clean_archives(plan, &mut self.index);
            self.synced_addons = self.config.addons.keys().cloned().collect();
        }
        save(
            &self.config,
            &self.paths.config_file,
            "config reconciled after sync",
        );
        let addons = self.paths.cache_dir.join(cache::DIRECTORY);
        if let Err(error) = self.index.save(&addons) {
            tracing::warn!("cannot save the cache index: {error}");
        }
        screen.refresh(&self.config, &self.index);
        self.exclusions.refresh(
            &installed_uids(&self.config, &self.index),
            self.config.update_prices,
        );

        report
    }

    fn complete_scan(&mut self, scanned: &Scanned, chosen: &[String]) -> Dialog {
        let failed = |message: String| Dialog::Report {
            title: fl!("scan-failed"),
            message,
        };

        let Screen::Catalogue(catalogue) = &mut self.screen else {
            return failed(fl!("catalogue-unavailable"));
        };
        let Some(addons) = catalogue.catalogue() else {
            return failed(fl!("catalogue-not-loaded"));
        };

        let mut seen = BTreeSet::new();
        let found: Vec<(String, String, String)> = scanned
            .addons
            .iter()
            .chain(chosen.iter())
            .filter_map(|uid| addons.position(uid).and_then(|index| addons.addon(index)))
            .filter(|addon| seen.insert(addon.uid.clone()))
            .map(|addon| (addon.uid.clone(), addon.name.clone(), addon.version.clone()))
            .collect();
        let unrecognised: Vec<Line> = scan::unrecognised(&scanned.folders, addons, &scanned.custom)
            .into_iter()
            .map(|folder| Line {
                name: folder.to_owned(),
                detail: String::new(),
            })
            .collect();

        for (_, name, version) in &found {
            tracing::info!("scan found {name} {version}");
        }
        for line in &unrecognised {
            tracing::info!("scan found an unrecognised folder {}", line.name);
        }
        tracing::info!("scan finished: {} addons added", found.len());

        let mut report = Report::default();
        report.push_section(
            fl!("scan-added"),
            found
                .iter()
                .map(|(_, name, version)| Line {
                    name: name.clone(),
                    detail: version.clone(),
                })
                .collect(),
        );
        report.push_section(fl!("scan-unrecognised"), unrecognised);
        if found.is_empty() {
            report.note = Some(fl!("scan-nothing-new"));
        }

        if !found.is_empty() {
            let reason = format!("{} addons added from scan", found.len());
            self.config
                .addons
                .extend(found.into_iter().map(|(uid, name, _)| (uid, name)));
            save(&self.config, &self.paths.config_file, &reason);
            catalogue.refresh(&self.config, &self.index);
            self.exclusions.refresh(
                &installed_uids(&self.config, &self.index),
                self.config.update_prices,
            );
        }

        details(fl!("scan-complete"), report)
    }

    fn choose_providers(
        &self,
        choices: &BTreeMap<String, Vec<String>>,
        then: AfterChoice,
    ) -> Dialog {
        let catalogue = match &self.screen {
            Screen::Catalogue(screen) => screen.catalogue(),
            Screen::Unavailable(_) => None,
        };
        let choices = choices
            .iter()
            .map(|(directory, uids)| {
                tracing::info!(
                    "{directory} has several possible providers: {}",
                    uids.join(", ")
                );
                let candidates: Vec<_> = uids
                    .iter()
                    .filter_map(|uid| {
                        let catalogue = catalogue?;
                        catalogue.addon(catalogue.position(uid)?)
                    })
                    .collect();
                let recommended = self
                    .recommended
                    .get(directory)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                choice::Choice::new(directory.clone(), &candidates, recommended)
            })
            .collect();

        Dialog::Choose { choices, then }
    }

    fn subscription(&self) -> Subscription<Message> {
        let screen = match &self.screen {
            Screen::Catalogue(catalogue) => catalogue.subscription().map(Message::Catalogue),
            Screen::Unavailable(_) => Subscription::none(),
        };
        let frames = if self.activity.is_some() {
            window::frames().map(Message::Frame)
        } else {
            Subscription::none()
        };

        let menu = if self.menu.is_some() {
            keyboard::listen().filter_map(|event| match event {
                keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(keyboard::key::Named::Escape),
                    ..
                } => Some(Message::MenuClosed),
                _ => None,
            })
        } else {
            Subscription::none()
        };

        Subscription::batch([
            screen,
            frames,
            menu,
            window::close_requests().map(|_| Message::CloseRequested),
            iced::event::listen_with(|event, _status, id| match event {
                iced::Event::Window(
                    event @ (window::Event::Opened { .. } | window::Event::Rescaled(_)),
                ) => Some(Message::WindowEvent(id, event)),
                _ => None,
            }),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let page: Element<'_, Message> = match &self.screen {
            Screen::Catalogue(catalogue) => column![
                self.tabs(),
                container(self.tab_content(catalogue)).height(Fill),
                rule::horizontal(1),
                self.folder_bar(catalogue.catalogue().is_some()),
            ]
            .into(),
            Screen::Unavailable(error) => center(text(error)).into(),
        };

        let mut layers: Vec<Element<'_, Message>> =
            vec![sensor(page).on_resize(Message::PageResized).into()];

        if let (Screen::Catalogue(catalogue), Some(menu)) = (&self.screen, self.menu) {
            layers.push(self.menu_overlay(menu, catalogue.catalogue().is_some()));
        }

        if let Screen::Catalogue(catalogue) = &self.screen
            && let Some(overlay) = catalogue.overlay()
        {
            layers.push(overlay.map(Message::Catalogue));
        }

        if let Some(dialog) = &self.dialog {
            let backdrop_area =
                mouse_area(center(opaque(dialog_view(dialog, self.page_height))).style(backdrop));
            layers.push(opaque(match dialog {
                Dialog::Migrate(_) => backdrop_area,
                _ => backdrop_area.on_press(Message::DialogClosed),
            }));
        }

        if let Some(url) = &self.confirm_open {
            layers.push(opaque(
                mouse_area(center(opaque(confirm_open_view(url))).style(backdrop))
                    .on_press(Message::OpenCancelled),
            ));
        }

        stack(layers).into()
    }

    fn menu_left(&self) -> f32 {
        let titles = MENUS
            .iter()
            .map(|menu| self.menu_widths.get(menu).copied().unwrap_or(0.0))
            .sum::<f32>()
            + MENU_SPACING * (MENUS.len() - 1) as f32;
        let menu = titles + 2.0 * BAR_OVERLAP_GAP;
        let content = self.page_width - 2.0 * SURFACE_PADDING;
        let centred = (content - menu) / 2.0;
        let beside_controls = content - self.controls_width - menu;

        centred.min(beside_controls).max(0.0)
    }

    fn menu_titles(&self) -> Element<'_, Message> {
        let busy = matches!(self.activity, Some(Activity::Scan));
        let mark: Element<'_, Message> = if busy {
            spinner(self.spinner_angle(), MENU_SPINNER_SIZE).into()
        } else {
            icon::menu().size(MENU_ICON_SIZE).into()
        };
        let content = row![mark, single_line(fl!("menu-title"))]
            .spacing(SPACING_S_NUDGE)
            .align_y(Center);

        let titles = MENUS.into_iter().zip([content]).map(|(menu, content)| {
            menu::title(
                content,
                self.menu == Some(menu),
                Message::MenuToggled(menu),
                Message::MenuHovered(menu),
                move |size| Message::MenuMeasured(menu, size),
            )
        });

        row(titles).spacing(MENU_SPACING).into()
    }

    fn menu_overlay(&self, menu: Menu, catalogue_loaded: bool) -> Element<'_, Message> {
        let idle = self.activity.is_none();
        let has_target = !self.config.target_directory.trim().is_empty();
        let columns = match menu {
            Menu::Settings => {
                let mut first = Vec::new();
                first.extend([
                    menu::Entry::Header(fl!("menu-ttc-prices")),
                    menu::Entry::Item(menu::Item {
                        label: fl!("menu-update-prices"),
                        on_press: Some(Message::UpdatePricesToggled),
                        checked: self.config.update_prices,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: fl!("region-eu"),
                        on_press: Some(Message::PriceRegionSelected(ttc::Region::Eu)),
                        checked: self.config.price_region == ttc::Region::Eu,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: fl!("region-na"),
                        on_press: Some(Message::PriceRegionSelected(ttc::Region::Na)),
                        checked: self.config.price_region == ttc::Region::Na,
                    }),
                    menu::Entry::Separator,
                    menu::Entry::Header(fl!("menu-language")),
                ]);
                first.extend(i18n::LANGUAGES.iter().map(|&language| {
                    menu::Entry::Item(menu::Item {
                        label: language.to_string(),
                        on_press: Some(Message::LanguageSelected(language)),
                        checked: language == self.language,
                    })
                }));

                let second = vec![
                    menu::Entry::Header(fl!("addons-folder")),
                    menu::Entry::Item(menu::Item {
                        label: fl!("menu-open-folder"),
                        on_press: has_target.then_some(Message::OpenFolder),
                        checked: false,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: if has_target {
                            fl!("menu-choose-folder")
                        } else {
                            fl!("menu-select-folder")
                        },
                        on_press: idle.then_some(Message::ChangeFolder),
                        checked: false,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: fl!("scan-folder"),
                        on_press: (idle && catalogue_loaded && has_target)
                            .then_some(Message::ScanRequested),
                        checked: false,
                    }),
                    menu::Entry::Separator,
                    menu::Entry::Header(fl!("menu-app-folders")),
                    menu::Entry::Item(menu::Item {
                        label: fl!("menu-configuration"),
                        on_press: Some(Message::OpenConfigFolder),
                        checked: false,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: fl!("menu-custom"),
                        on_press: Some(Message::OpenCustomFolder),
                        checked: false,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: fl!("menu-cache"),
                        on_press: Some(Message::OpenCacheFolder),
                        checked: false,
                    }),
                    menu::Entry::Separator,
                    menu::Entry::Header(fl!("tab-log")),
                    menu::Entry::Item(menu::Item {
                        label: fl!("log-to-file"),
                        on_press: Some(Message::LogToFileToggled),
                        checked: self.config.log_to_file,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: fl!("log-open-folder"),
                        on_press: Some(Message::OpenLogsFolder),
                        checked: false,
                    }),
                    menu::Entry::Item(menu::Item {
                        label: fl!("log-clear"),
                        on_press: self
                            .file_log
                            .has_logs()
                            .then_some(Message::ClearLogsRequested),
                        checked: false,
                    }),
                ];

                let mut third = Vec::new();
                // The whole update section is present only on a direct launch; under `--no-update-check`
                // (updater-launched) it is gone, not greyed.
                if self.update_ui {
                    third.extend([
                        menu::Entry::Header(fl!("menu-updates")),
                        menu::Entry::Item(menu::Item {
                            label: fl!("menu-check-on-launch"),
                            on_press: Some(Message::CheckOnLaunchToggled),
                            checked: self.config.check_for_updates,
                        }),
                        menu::Entry::Item(menu::Item {
                            label: fl!("menu-notify-prereleases"),
                            on_press: Some(Message::NotifyPrereleasesToggled),
                            checked: self.config.notify_prereleases,
                        }),
                        menu::Entry::Item(menu::Item {
                            label: fl!("menu-check-now"),
                            on_press: Some(Message::CheckUpdatesNow),
                            checked: false,
                        }),
                        menu::Entry::Separator,
                    ]);
                }
                third.extend([
                    menu::Entry::Header(fl!("menu-support")),
                    menu::Entry::Item(menu::Item {
                        label: fl!("menu-report-problem"),
                        on_press: Some(Message::ReportProblemRequested),
                        checked: false,
                    }),
                    menu::Entry::Separator,
                ]);
                third.extend([
                    menu::Entry::Header(fl!("menu-experimental")),
                    menu::Entry::Item(menu::Item {
                        label: fl!("menu-discontinued"),
                        on_press: idle.then_some(Message::DiscontinuedToggled),
                        checked: self.config.include_outdated,
                    }),
                ]);

                vec![first, second, third]
            }
        };
        let widths: Vec<Option<f32>> = (0..columns.len())
            .map(|column| {
                self.menu_list_widths
                    .get(&(menu, column))
                    .map(|size| size.width)
            })
            .collect();
        let height = (0..columns.len())
            .filter_map(|column| {
                self.menu_list_widths
                    .get(&(menu, column))
                    .map(|size| size.height)
            })
            .fold(0.0_f32, f32::max);
        let height = (height > 0.0).then_some(height);
        let list = menu::list(columns, &widths, height, move |column, size| {
            Message::MenuListMeasured(menu, column, size)
        });

        let title_left = SURFACE_PADDING
            + self.menu_left()
            + BAR_OVERLAP_GAP
            + MENUS
                .iter()
                .take_while(|other| **other != menu)
                .map(|other| self.menu_widths.get(other).copied().unwrap_or(0.0) + MENU_SPACING)
                .sum::<f32>();
        let title_centre = title_left + self.menu_widths.get(&menu).copied().unwrap_or(0.0) / 2.0;
        let total = self.menu_total_widths.get(&menu).copied().unwrap_or(0.0);
        let list_left = (title_centre - total / 2.0).max(0.0);

        let list = sensor(list).on_resize(move |size| Message::MenuTotalMeasured(menu, size));

        let bar_height = BAR_CONTENT_HEIGHT + 2.0 * BAR_VERTICAL_PADDING;
        stack![
            column![
                mouse_area(space().width(Fill).height(Fill))
                    .interaction(mouse::Interaction::Idle)
                    .on_press(Message::MenuClosed),
                space().height(bar_height),
            ],
            column![
                space().height(Fill),
                row![space().width(list_left), list],
                space().height(bar_height),
            ],
        ]
        .into()
    }

    fn tabs(&self) -> Element<'_, Message> {
        let tab = |label: String, tab: Tab| widget::Tab {
            label,
            active: self.tab == tab,
            on_press: Some(Message::TabSelected(tab)),
        };

        widget::tab_bar(
            [
                tab(fl!("tab-catalogue"), Tab::Catalogue),
                tab(fl!("tab-libraries"), Tab::Libraries),
                tab(fl!("tab-exclusions"), Tab::Exclusions),
                tab(fl!("tab-log"), Tab::Log),
            ],
            Fill,
            widget::TAB_PADDING,
        )
    }

    fn tab_content<'a>(&'a self, catalogue: &'a catalogue::Catalogue) -> Element<'a, Message> {
        match self.tab {
            Tab::Catalogue => catalogue
                .view(
                    &self.config,
                    &self.index,
                    &self.dependents,
                    catalogue::Activity {
                        busy: self.activity.is_some(),
                        can_install: !self.config.target_directory.trim().is_empty(),
                        installing: match &self.activity {
                            Some(Activity::Sync { toggle, .. }) => {
                                toggle.as_ref().map(|toggle| toggle.uid.as_str())
                            }
                            _ => None,
                        },
                        spinner_angle: self.spinner_angle(),
                    },
                )
                .map(Message::Catalogue),
            Tab::Libraries => match catalogue.catalogue() {
                Some(addons) => {
                    libraries::view(addons, &self.config, &self.recommended).map(Message::Libraries)
                }
                None => center(text(fl!("loading-catalogue"))).into(),
            },
            Tab::Exclusions => self.exclusions.view(&self.config).map(Message::Exclusions),
            Tab::Log => self.log.view().map(Message::Log),
        }
    }

    fn spinner_angle(&self) -> f32 {
        self.frame
            .saturating_duration_since(self.started)
            .as_secs_f32()
            .fract()
            * TAU
    }

    fn folder_bar(&self, catalogue_loaded: bool) -> Element<'_, Message> {
        let idle = self.activity.is_none();
        let has_target = !self.config.target_directory.trim().is_empty();
        let ready = catalogue_loaded && idle && has_target;
        let busy_spinner = || -> Element<'_, Message> {
            container(spinner(self.spinner_angle(), BAR_SPINNER_SIZE))
                .padding(BAR_BUTTON_PADDING)
                .into()
        };

        let sync: Element<'_, Message> = match self.activity {
            Some(Activity::Sync { .. }) => busy_spinner(),
            _ => button(single_line(fl!("sync")))
                .style(button_style)
                .padding(BAR_BUTTON_PADDING)
                .on_press_maybe(ready.then_some(Message::SyncRequested))
                .into(),
        };

        let path: Element<'_, Message> = if has_target {
            let full_path = std::path::absolute(&self.config.target_directory).map_or_else(
                |_| self.config.target_directory.clone(),
                |path| path.display().to_string(),
            );
            tooltip(
                text(&self.config.target_directory).wrapping(text::Wrapping::None),
                text(full_path),
                tooltip::Position::Top,
            )
            .style(container::rounded_box)
            .into()
        } else {
            secondary(fl!("addons-folder-none")).into()
        };
        let folder = row![secondary(fl!("addons-folder")), path]
            .spacing(SPACING_M_NUDGE)
            .align_y(Center);
        let controls = row![
            checkbox(self.config.sync_on_launch)
                .label(fl!("sync-on-launch"))
                .text_wrapping(text::Wrapping::None)
                .on_toggle(Message::SyncOnLaunchToggled),
            sync,
        ]
        .spacing(SPACING_M_NUDGE)
        .align_y(Center);

        let overlay = row![
            space().width(self.menu_left()),
            container(self.menu_titles())
                .padding([0.0, BAR_OVERLAP_GAP])
                .style(bar_fill),
            space::horizontal(),
            sensor(
                container(controls)
                    .padding(Padding::ZERO.left(BAR_OVERLAP_GAP))
                    .style(bar_fill),
            )
            .on_resize(Message::ControlsMeasured),
        ]
        .align_y(Center)
        .height(BAR_CONTENT_HEIGHT);

        let bar = surface(
            stack![overlay].push_under(
                container(folder)
                    .width(Fill)
                    .height(Fill)
                    .align_y(Center)
                    .clip(true),
            ),
        )
        .padding([BAR_VERTICAL_PADDING, SURFACE_PADDING]);

        mouse_area(bar).on_press(Message::MenuClosed).into()
    }
}

fn confirm_open_view(url: &str) -> Element<'_, Message> {
    dialog_frame(
        fl!("open-link-question"),
        secondary(url),
        vec![
            dialog_button(fl!("cancel"), Message::OpenCancelled),
            dialog_button(fl!("open-link-confirm"), Message::OpenConfirmed),
        ],
    )
}

fn eso_addons_example() -> &'static str {
    if cfg!(target_os = "windows") {
        r"Documents\Elder Scrolls Online\live\AddOns"
    } else if cfg!(target_os = "macos") {
        "Documents/Elder Scrolls Online/live/AddOns"
    } else {
        "~/.local/share/Steam/steamapps/compatdata/306130/pfx/drive_c/users/steamuser/Documents/Elder Scrolls Online/live/AddOns"
    }
}

fn dialog_view(dialog: &Dialog, page_height: f32) -> Element<'_, Message> {
    match dialog {
        Dialog::ScanPrompt { previous_folder } => {
            let mut buttons = Vec::new();
            if previous_folder.is_some() {
                buttons.push(dialog_button(
                    fl!("scan-keep-folder"),
                    Message::FolderRestored,
                ));
            }
            buttons.push(dialog_button(fl!("scan-skip"), Message::DialogClosed));
            buttons.push(dialog_button(fl!("scan-confirm"), Message::ScanConfirmed));

            dialog_frame(
                fl!("scan-prompt-title"),
                text(format!(
                    "{}\n\n{}",
                    fl!("scan-prompt-question"),
                    fl!("scan-prompt-warning")
                )),
                buttons,
            )
        }
        Dialog::Report { title, message } => dialog_frame(
            title.as_str(),
            text(message),
            vec![dialog_button(fl!("ok"), Message::DialogClosed)],
        ),
        Dialog::Details {
            title,
            report,
            expanded,
        } => dialog_frame(
            title.as_str(),
            container(
                scrollable(report::view(
                    report,
                    expanded,
                    Message::SectionToggled,
                    false,
                ))
                .spacing(DIALOG_SPACING),
            )
            .max_height(page_height * REPORT_HEIGHT_RATIO),
            vec![dialog_button(fl!("ok"), Message::DialogClosed)],
        ),
        Dialog::Choose { choices, .. } => dialog_frame(
            fl!("providers-title"),
            column![
                text(fl!("providers-note")),
                container(
                    scrollable(choice::view(
                        choices,
                        Message::ProviderSelected,
                        Message::OpenPage
                    ))
                    .spacing(DIALOG_SPACING)
                )
                .max_height(page_height * REPORT_HEIGHT_RATIO),
            ]
            .spacing(DIALOG_SPACING),
            vec![
                dialog_button(fl!("cancel"), Message::DialogClosed),
                dialog_button(
                    fl!("continue"),
                    choice::is_complete(choices).then_some(Message::ProvidersConfirmed),
                ),
            ],
        ),
        Dialog::ConfirmRemoval {
            report, expanded, ..
        } => dialog_frame(
            fl!("remove-missing-title"),
            column![
                text(fl!("remove-missing-question")),
                container(
                    scrollable(report::view(
                        report,
                        expanded,
                        Message::SectionToggled,
                        true
                    ))
                    .spacing(DIALOG_SPACING),
                )
                .max_height(page_height * REPORT_HEIGHT_RATIO),
            ]
            .spacing(DIALOG_SPACING),
            vec![
                dialog_button(fl!("cancel"), Message::DialogClosed),
                dialog_button(fl!("continue"), Message::RemovalConfirmed),
            ],
        ),
        Dialog::ReportProblem {
            selection,
            description,
            contact,
            sending,
        } => {
            let can_send = !*sending
                && (selection.config
                    || selection.config_backups
                    || selection.log
                    || selection.log_backups
                    || !description.text().trim().is_empty());
            let (action_label, action) = if support::is_enabled() {
                (fl!("report-send"), Message::ReportSend)
            } else {
                (fl!("report-save"), Message::ReportSave)
            };
            dialog_frame(
                fl!("report-title"),
                column![
                    text(fl!("report-intro")),
                    checkbox(selection.config)
                        .label(fl!("report-include-config"))
                        .on_toggle(Message::ReportConfigToggled),
                    checkbox(selection.config_backups)
                        .label(fl!("report-include-config-backups"))
                        .on_toggle(Message::ReportConfigBackupsToggled),
                    checkbox(selection.log)
                        .label(fl!("report-include-logs"))
                        .on_toggle(Message::ReportLogsToggled),
                    checkbox(selection.log_backups)
                        .label(fl!("report-include-log-files"))
                        .on_toggle(Message::ReportLogsBackupsToggled),
                    secondary(fl!("report-description-label")),
                    text_editor(description)
                        .placeholder(fl!("report-description-placeholder"))
                        .height(96.0)
                        .on_action(Message::ReportDescriptionAction),
                    secondary(fl!("report-contact-label")),
                    text_input(&fl!("report-contact-placeholder"), contact)
                        .on_input(Message::ReportContactChanged),
                ]
                .spacing(DIALOG_SPACING),
                vec![
                    dialog_button(fl!("cancel"), Message::DialogClosed),
                    dialog_button(action_label, can_send.then_some(action)),
                ],
            )
        }
        Dialog::ReportSaved {
            path,
            after_failed_send,
        } => dialog_frame(
            fl!("report-saved-title"),
            text(if *after_failed_send {
                fl!(
                    "report-saved-sent-failed",
                    path = path.display().to_string()
                )
            } else {
                fl!("report-saved-message", path = path.display().to_string())
            }),
            vec![
                dialog_button(fl!("report-open-folder"), Message::OpenReportFolder),
                dialog_button(fl!("ok"), Message::DialogClosed),
            ],
        ),
        Dialog::ClearLogs { choice, writing } => dialog_frame(
            fl!("log-clear-title"),
            column![
                radio(
                    fl!("log-clear-others"),
                    ClearChoice::Others,
                    Some(*choice),
                    Message::ClearLogsChoice,
                ),
                radio(
                    if *writing {
                        fl!("log-clear-all-disable")
                    } else {
                        fl!("log-clear-all")
                    },
                    ClearChoice::AllDisable,
                    Some(*choice),
                    Message::ClearLogsChoice,
                ),
            ]
            .spacing(DIALOG_SPACING),
            vec![
                dialog_button(fl!("cancel"), Message::DialogClosed),
                dialog_button(fl!("ok"), Message::ClearLogsConfirmed),
            ],
        ),
        Dialog::Migrate(_) => dialog_frame(
            fl!("migrate-title"),
            text(fl!("migrate-question")),
            vec![
                dialog_button(fl!("migrate-open-folders"), Message::MigrateOpenFolders),
                dialog_button(fl!("migrate-keep-profile"), Message::MigrateChoice(false)),
                dialog_button(fl!("migrate-keep-old"), Message::MigrateChoice(true)),
            ],
        ),
        Dialog::ChooseFolder { configured } => dialog_frame(
            if *configured {
                fl!("change-folder-title")
            } else {
                fl!("no-folder-title")
            },
            text(format!(
                "{}\n\n{}",
                fl!("no-folder-question", example = eso_addons_example()),
                fl!("no-folder-warning")
            )),
            vec![
                dialog_button(fl!("cancel"), Message::DialogClosed),
                dialog_button(fl!("no-folder-choose"), Message::ChooseFolder),
            ],
        ),
        Dialog::UpdateAvailable => dialog_frame(
            fl!("update-title"),
            text(fl!("update-available")),
            vec![
                dialog_button(fl!("cancel"), Message::DialogClosed),
                dialog_button(fl!("update-dont-ask"), Message::UpdateDisable),
                dialog_button(fl!("update-open-page"), Message::UpdateOpenPage),
            ],
        ),
    }
}

fn dialog_frame<'a>(
    title: impl text::IntoFragment<'a>,
    body: impl Into<Element<'a, Message>>,
    buttons: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            column![
                text(title).font(Font {
                    weight: font::Weight::Semibold,
                    ..Font::DEFAULT
                }),
                body.into(),
            ]
            .spacing(DIALOG_SPACING)
            .width(Fill),
            row(buttons).spacing(SPACING_M_NUDGE),
        ]
        .spacing(DIALOG_SPACING)
        .align_x(Right),
    )
    .max_width(DIALOG_MAX_WIDTH)
    .padding(SPACING_XL)
    .style(dialog_style)
    .into()
}

fn details(title: String, report: Report) -> Dialog {
    let expanded = report.expanded();

    Dialog::Details {
        title,
        report,
        expanded,
    }
}

fn dialog_button<'a>(label: String, message: impl Into<Option<Message>>) -> Element<'a, Message> {
    button(single_line(label))
        .style(button_style)
        .on_press_maybe(message.into())
        .into()
}

fn bar_fill(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.weakest.color.into()),
        ..container::Style::default()
    }
}

fn save(config: &Config, path: &Path, reason: &str) {
    match config.save(path) {
        Ok(()) => tracing::info!("config updated: {reason}"),
        Err(error) => tracing::error!("cannot save settings ({reason}): {error}"),
    }
}

fn save_report_dialog() -> Task<Message> {
    let name = support::bundle_filename();
    Task::perform(
        async move {
            rfd::AsyncFileDialog::new()
                .set_file_name(name)
                .save_file()
                .await
                .map(|handle| handle.path().to_path_buf())
        },
        Message::ReportSaveChosen,
    )
}

fn installed_uids(config: &Config, index: &cache::Index) -> BTreeSet<String> {
    config.addons.keys().chain(index.keys()).cloned().collect()
}

enum Instance {
    // The lock is held (or could not be set up); the file keeps the lock for the process lifetime.
    Primary(Option<std::fs::File>),
    // Another instance already holds the lock.
    Busy,
}

fn acquire_instance_lock(paths: &paths::Paths) -> Instance {
    let path = &paths.lock_file;

    let file = match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
    {
        Ok(file) => file,
        Err(error) => {
            tracing::warn!("cannot open the instance lock {}: {error}", path.display());
            return Instance::Primary(None);
        }
    };

    match file.try_lock() {
        Ok(()) => Instance::Primary(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Instance::Busy,
        Err(std::fs::TryLockError::Error(error)) => {
            tracing::warn!("cannot lock the instance file {}: {error}", path.display());
            Instance::Primary(None)
        }
    }
}

async fn check_updates(beta: bool) -> Result<Option<update::Available>, String> {
    let client = api::Client::new().map_err(|error| error.to_string())?;

    if beta {
        // Beta mode: the GitHub API, picking the newest release by semver (pre-releases included).
        let body = client
            .text(update::releases_api())
            .await
            .map_err(|error| error.to_string())?;
        let releases: Vec<update::Release> =
            serde_json::from_str(&body).map_err(|error| error.to_string())?;
        let Some(tag) = update::newest(&releases) else {
            return Ok(None);
        };
        let version = tag.strip_prefix('v').unwrap_or(tag);
        Ok(update::is_newer(version).then(|| update::Available {
            version: version.to_owned(),
            page: update::tag_page(tag),
        }))
    } else {
        // Stable mode: the GitHub API `releases/latest` — the newest non-draft, non-prerelease.
        let body = client
            .text(&update::stable_latest_api())
            .await
            .map_err(|error| error.to_string())?;
        let release: update::Release =
            serde_json::from_str(&body).map_err(|error| error.to_string())?;
        let tag = release.tag_name.as_str();
        let version = tag.strip_prefix('v').unwrap_or(tag);
        Ok(update::is_newer(version).then(|| update::Available {
            version: version.to_owned(),
            page: update::stable_page(),
        }))
    }
}

// Whether the settings file still carries the in-app update keys (left from a past direct launch).
fn update_keys_present(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .is_ok_and(|text| text.contains("CheckForUpdates") || text.contains("NotifyPrereleases"))
}
