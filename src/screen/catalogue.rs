mod description;
mod gallery;

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::time::Instant;

use eso_addons_manager_core::{Addon, Config, Filter, Query, SortOrder, api, bbcode, cache, store};
use iced::futures::future;
use iced::widget::operation::scroll_to;
use iced::widget::{
    button, center, center_x, column, container, image, pane_grid, row, rule, scrollable, sensor,
    space, stack, text, text_input, tooltip,
};
use iced::{
    Bottom, Center, Element, Fill, Font, Length, Padding, Shrink, Size, Subscription, Task, Theme,
    font,
};

use self::description::Description;
use self::gallery::Gallery;
use crate::i18n::fl;
use crate::widget::{
    CAPTION_TEXT_SIZE, Chip, SPACING_L, SPACING_M, SPACING_M_NUDGE, SPACING_S, SPACING_S_NUDGE,
    SPACING_XL, SPACING_XS, SPACING_XXS, SPACING_XXXL, SUBTITLE_TEXT_SIZE, TAB_PADDING, Tab,
    button_style, chip_style, inverted_input, open_page, secondary, spinner, star, surface,
    tab_bar,
};
use crate::{format, html, icon};

const ICON_SIZE: f32 = 45.0;
const DATE_COLUMN_WIDTH: f32 = 3.0 * SPACING_XXXL;
const NUMBER_COLUMN_WIDTH: f32 = 2.0 * SPACING_XXXL;
const SORT_ARROW_SLOT: f32 = SPACING_L;
const METRIC_ICON_SPACING: f32 = SPACING_XS;
const DETAILS_SPACING: f32 = SPACING_M_NUDGE;
const STAR_SIZE: f32 = SPACING_XL;
const SEARCH_PADDING: Padding = Padding {
    top: SPACING_S,
    right: SPACING_M,
    bottom: SPACING_S,
    left: SPACING_M,
};
const INSTALL_ICON_SIZE: f32 = SPACING_XL;
const LIST_OVERSCAN: usize = 5;
const LIST_RATIO: f32 = 0.3;
const LIST_WIDTH: f32 = 560.0;
const PANEL_MIN_SIZE: f32 = 300.0;
const SPLIT_LEEWAY: f32 = 10.0;

static LIST_ID: LazyLock<iced::widget::Id> =
    LazyLock::new(|| iced::widget::Id::new("catalogue-list"));

pub struct Catalogue {
    client: api::Client,
    api_dir: Arc<Path>,
    theme: Theme,
    state: State,
    width: Option<f32>,
    scale: f32,
    loading_since: Instant,
}

enum State {
    Loading,
    Failed(String),
    Loaded(Box<Loaded>),
}

struct Loaded {
    catalogue: eso_addons_manager_core::Catalogue,
    query: Query,
    results: Vec<usize>,
    icons: HashMap<String, image::Handle>,
    selected: Option<usize>,
    details: Details,
    section: Section,
    description: Description,
    gallery: Gallery,
    panels: pane_grid::State<Panel>,
    list: ListViewport,
    sort_row_width: Option<f32>,
    split_pending: bool,
}

#[derive(Default)]
struct ListViewport {
    offset: f32,
    height: f32,
    row_height: Option<f32>,
}

enum Panel {
    List,
    Details,
}

enum Details {
    Loading,
    Ready(Sections),
    Failed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Description,
    Changelog,
}

#[derive(Debug, Clone)]
pub struct Sections {
    description: String,
    changelog: String,
}

impl Sections {
    fn body(&self, section: Section) -> &str {
        match section {
            Section::Description => &self.description,
            Section::Changelog => &self.changelog,
        }
    }

    fn page(&self, section: Section) -> String {
        format!("<html><body>{}</body></html>", self.body(section))
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<(Vec<api::FileListEntry>, Vec<api::CategoryEntry>), api::Error>),
    Retry,
    IconLoaded(String, Result<image::Handle, api::Error>),
    SearchChanged(String),
    InstalledFilterChanged(Filter),
    FavouritesFilterChanged(Filter),
    FavouriteToggled(usize),
    InstallToggled(usize),
    SortSelected(SortOption),
    AddonSelected(usize),
    DetailsLoaded(String, Result<Sections, api::Error>),
    SectionSelected(Section),
    OpenPage,
    PanelResized(pane_grid::ResizeEvent),
    Description(description::Message),
    Gallery(gallery::Message),
    ListScrolled(scrollable::Viewport),
    ListResized(Size),
    RowMeasured(Size),
    SortRowMeasured(Size),
    Resized(Size),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Loaded(Task<Message>),
    ToggleFavourite {
        uid: String,
        name: String,
    },
    ToggleInstalled {
        uid: String,
        name: String,
    },
    OpenUrl(String),
    QueryChanged {
        sort: SortOrder,
        reversed: bool,
        installed: Filter,
        favourites: Filter,
    },
}

#[derive(Clone, Copy)]
pub struct Activity<'a> {
    pub busy: bool,
    pub can_install: bool,
    pub installing: Option<&'a str>,
    pub spinner_angle: f32,
}

#[derive(Clone, Copy)]
struct Context<'a> {
    config: &'a Config,
    index: &'a cache::Index,
    dependents: &'a BTreeMap<String, Vec<String>>,
    activity: Activity<'a>,
}

impl Catalogue {
    pub fn new(client: api::Client, api_dir: Arc<Path>, theme: Theme) -> (Self, Task<Message>) {
        let task = load(client.clone(), api_dir.clone());

        (
            Self {
                client,
                api_dir,
                theme,
                state: State::Loading,
                width: None,
                scale: 1.0,
                loading_since: Instant::now(),
            },
            task,
        )
    }

    pub fn set_scale(&mut self, scale: f32) -> Task<Message> {
        self.scale = scale;

        if let State::Loaded(loaded) = &mut self.state {
            loaded
                .description
                .set_scale(scale)
                .map(Message::Description)
        } else {
            Task::none()
        }
    }

    pub fn update(&mut self, message: Message, config: &Config) -> Action {
        match message {
            Message::Loaded(Ok((entries, categories))) => {
                let catalogue = eso_addons_manager_core::Catalogue::new(entries, categories);
                let query = Query {
                    text: String::new(),
                    sort: config.sort,
                    reversed: config.sort_reversed,
                    installed: config.filter_installed,
                    favourites: config.filter_favourites,
                };
                let results = catalogue.search(&query, config);
                let icons_task = Task::batch(
                    catalogue
                        .categories()
                        .iter()
                        .filter(|category| !category.icon_url.is_empty())
                        .map(|category| {
                            load_icon(
                                self.client.clone(),
                                category.id.clone(),
                                category.icon_url.clone(),
                            )
                        }),
                );

                tracing::info!(
                    "catalogue loaded: {} addons in {:?}",
                    catalogue.len(),
                    self.loading_since.elapsed()
                );

                self.state = State::Loaded(Box::new(Loaded {
                    catalogue,
                    query,
                    results,
                    icons: HashMap::new(),
                    selected: None,
                    details: Details::Loading,
                    section: Section::Description,
                    description: Description::new(html::Engine::spawn(&self.theme), self.scale),
                    gallery: Gallery::default(),
                    list: ListViewport::default(),
                    sort_row_width: None,
                    split_pending: self.width.is_none(),
                    panels: pane_grid::State::with_configuration(pane_grid::Configuration::Split {
                        axis: pane_grid::Axis::Vertical,
                        ratio: self.width.map_or(LIST_RATIO, list_ratio),
                        a: Box::new(pane_grid::Configuration::Pane(Panel::List)),
                        b: Box::new(pane_grid::Configuration::Pane(Panel::Details)),
                    }),
                }));

                Action::Loaded(icons_task)
            }
            Message::Loaded(Err(error)) => {
                tracing::error!("cannot load catalogue: {error}");
                self.state = State::Failed(error.to_string());
                Action::None
            }
            Message::Retry => {
                self.state = State::Loading;
                Action::Run(load(self.client.clone(), self.api_dir.clone()))
            }
            Message::Resized(size) => {
                self.width = Some(size.width);

                if let State::Loaded(loaded) = &mut self.state
                    && loaded.split_pending
                    && let pane_grid::Node::Split { id, .. } = loaded.panels.layout()
                {
                    let split = *id;
                    loaded.split_pending = false;
                    loaded.panels.resize(split, list_ratio(size.width));
                }
                Action::None
            }
            message => {
                let State::Loaded(loaded) = &mut self.state else {
                    return Action::None;
                };

                loaded.update(&self.client, &self.api_dir, message, config)
            }
        }
    }

    pub fn refresh(&mut self, config: &Config) {
        if let State::Loaded(loaded) = &mut self.state {
            loaded.refresh(config);
        }
    }

    pub fn replace(
        &mut self,
        catalogue: eso_addons_manager_core::Catalogue,
        config: &Config,
    ) -> Task<Message> {
        if let State::Loaded(loaded) = &mut self.state {
            loaded.replace(catalogue, config);
            return scroll_to(
                LIST_ID.clone(),
                scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: loaded.list.offset,
                },
            );
        }
        Task::none()
    }

    pub fn overlay(&self) -> Option<Element<'_, Message>> {
        match &self.state {
            State::Loaded(loaded) => loaded
                .gallery
                .overlay()
                .map(|overlay| overlay.map(Message::Gallery)),
            _ => None,
        }
    }

    pub fn client(&self) -> &api::Client {
        &self.client
    }

    pub fn catalogue(&self) -> Option<&eso_addons_manager_core::Catalogue> {
        match &self.state {
            State::Loaded(loaded) => Some(&loaded.catalogue),
            _ => None,
        }
    }

    pub fn view<'a>(
        &'a self,
        config: &'a Config,
        index: &'a cache::Index,
        dependents: &'a BTreeMap<String, Vec<String>>,
        activity: Activity<'a>,
    ) -> Element<'a, Message> {
        let context = Context {
            config,
            index,
            dependents,
            activity,
        };

        let content: Element<'a, Message> = match &self.state {
            State::Loading => center(text(fl!("loading-catalogue"))).into(),
            State::Failed(error) => center(
                column![
                    text(fl!("catalogue-failed", error = error.clone())),
                    button(text(fl!("retry")))
                        .style(button_style)
                        .on_press(Message::Retry),
                ]
                .spacing(SPACING_M_NUDGE)
                .align_x(Center),
            )
            .into(),
            State::Loaded(loaded) => loaded.view(context),
        };

        sensor(content).on_resize(Message::Resized).into()
    }
}

impl Catalogue {
    pub fn subscription(&self) -> Subscription<Message> {
        match &self.state {
            State::Loaded(loaded) => loaded.gallery.subscription().map(Message::Gallery),
            _ => Subscription::none(),
        }
    }
}

impl Loaded {
    fn update(
        &mut self,
        client: &api::Client,
        api: &Arc<Path>,
        message: Message,
        config: &Config,
    ) -> Action {
        match message {
            Message::IconLoaded(category_id, Ok(handle)) => {
                self.icons.insert(category_id, handle);
                Action::None
            }
            Message::IconLoaded(category_id, Err(error)) => {
                tracing::warn!("cannot load icon of category {category_id}: {error}");
                Action::None
            }
            Message::SearchChanged(text) => {
                self.query.text = text;
                self.refresh(config);
                Action::None
            }
            Message::InstalledFilterChanged(installed) => {
                self.query.installed = installed;
                self.refresh(config);
                self.persist_query()
            }
            Message::FavouritesFilterChanged(favourites) => {
                self.query.favourites = favourites;
                self.refresh(config);
                self.persist_query()
            }
            Message::FavouriteToggled(index) => match self.catalogue.addon(index) {
                Some(addon) => Action::ToggleFavourite {
                    uid: addon.uid.clone(),
                    name: addon.name.clone(),
                },
                None => Action::None,
            },
            Message::InstallToggled(index) => match self.catalogue.addon(index) {
                Some(addon) => Action::ToggleInstalled {
                    uid: addon.uid.clone(),
                    name: addon.name.clone(),
                },
                None => Action::None,
            },
            Message::SortSelected(SortOption(sort)) => {
                if self.query.sort == sort {
                    self.query.reversed = !self.query.reversed;
                } else {
                    self.query.sort = sort;
                    self.query.reversed = false;
                }
                self.refresh(config);
                self.persist_query()
            }
            Message::AddonSelected(index) => {
                let Some(addon) = self.catalogue.addon(index) else {
                    return Action::None;
                };

                self.selected = Some(index);
                self.details = Details::Loading;
                self.section = Section::Description;

                Action::Run(load_details(client.clone(), api.clone(), addon.uid.clone()))
            }
            Message::DetailsLoaded(uid, result) => {
                if self.selected_addon().is_none_or(|addon| addon.uid != uid) {
                    return Action::None;
                }

                match result {
                    Ok(sections) => {
                        if sections.body(self.section).is_empty()
                            && !sections.body(Section::Changelog).is_empty()
                        {
                            self.section = Section::Changelog;
                        }
                        let page = sections.page(self.section);
                        self.details = Details::Ready(sections);
                        Action::Run(self.description.show(page).map(Message::Description))
                    }
                    Err(error) => {
                        tracing::warn!("cannot load addon details: {error}");
                        self.details = Details::Failed(error.to_string());
                        Action::None
                    }
                }
            }
            Message::SectionSelected(section) => {
                let Details::Ready(sections) = &self.details else {
                    return Action::None;
                };
                if section == self.section {
                    return Action::None;
                }

                self.section = section;
                let page = sections.page(section);
                Action::Run(self.description.replace(page).map(Message::Description))
            }
            Message::OpenPage => match self.selected_addon() {
                Some(addon) => Action::OpenUrl(addon.info_url.clone()),
                None => Action::None,
            },
            Message::Gallery(message) => {
                Action::Run(self.gallery.update(client, message).map(Message::Gallery))
            }
            Message::Description(message) => match self.description.update(client, message) {
                description::Action::None => Action::None,
                description::Action::Run(task) => Action::Run(task.map(Message::Description)),
                description::Action::OpenUrl(url) => Action::OpenUrl(url),
            },
            Message::ListScrolled(viewport) => {
                self.list.offset = viewport.absolute_offset().y;
                self.list.height = viewport.bounds().height;
                Action::None
            }
            Message::ListResized(size) => {
                self.list.height = size.height;
                Action::None
            }
            Message::SortRowMeasured(size) => {
                self.sort_row_width = Some(size.width);
                Action::None
            }
            Message::RowMeasured(size) => {
                if size.height > 0.0 {
                    self.list.row_height = Some(size.height);
                }
                Action::None
            }
            Message::PanelResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panels.resize(split, ratio);
                Action::None
            }
            Message::Loaded(_) | Message::Retry | Message::Resized(_) => Action::None,
        }
    }

    fn refresh(&mut self, config: &Config) {
        self.results = self.catalogue.search(&self.query, config);
    }

    fn persist_query(&self) -> Action {
        Action::QueryChanged {
            sort: self.query.sort,
            reversed: self.query.reversed,
            installed: self.query.installed,
            favourites: self.query.favourites,
        }
    }

    fn replace(&mut self, catalogue: eso_addons_manager_core::Catalogue, config: &Config) {
        let selected = self.selected_addon().map(|addon| addon.uid.clone());

        self.catalogue = catalogue;
        self.selected = selected.and_then(|uid| self.catalogue.position(&uid));
        self.refresh(config);
    }

    fn selected_addon(&self) -> Option<&Addon> {
        self.selected.and_then(|index| self.catalogue.addon(index))
    }

    fn view<'a>(&'a self, context: Context<'a>) -> Element<'a, Message> {
        let panels = pane_grid(&self.panels, |_, panel, _| {
            pane_grid::Content::new(match panel {
                Panel::List => row![self.list(context), rule::vertical(1)],
                Panel::Details => row![self.details()],
            })
        })
        .min_size(PANEL_MIN_SIZE)
        .on_resize(SPLIT_LEEWAY, Message::PanelResized);

        panels.into()
    }

    fn list<'a>(&'a self, context: Context<'a>) -> iced::widget::Column<'a, Message> {
        let search = row![
            text_input(&fl!("search-placeholder"), &self.query.text)
                .on_input(Message::SearchChanged)
                .padding(SEARCH_PADDING)
                .style(inverted_input),
            text(fl!("addon-count", count = self.results.len())),
        ]
        .spacing(SPACING_M_NUDGE)
        .align_y(Center);

        let filters = row![
            filter_button(
                fl!("filter-installed"),
                self.query.installed,
                Message::InstalledFilterChanged
            ),
            filter_button(
                fl!("filter-favourites"),
                self.query.favourites,
                Message::FavouritesFilterChanged
            ),
        ]
        .spacing(SPACING_XS)
        .width(self.sort_row_width.map_or(Shrink, Length::Fixed));

        let controls = column![
            labelled(fl!("filter"), filters),
            labelled(
                fl!("sort-by"),
                sensor(self.sort_buttons()).on_resize(Message::SortRowMeasured),
            ),
        ]
        .spacing(SPACING_M_NUDGE);

        let rows: Vec<Element<'_, Message>> = match self.list.row_height {
            None => self
                .results
                .first()
                .map(|&index| {
                    sensor(self.addon_item(index, context))
                        .on_resize(Message::RowMeasured)
                        .into()
                })
                .into_iter()
                .collect(),
            Some(row_height) => {
                let total = self.results.len();
                let visible = (self.list.height / row_height).ceil() as usize + 2 * LIST_OVERSCAN;
                let first = ((self.list.offset / row_height) as usize)
                    .saturating_sub(LIST_OVERSCAN)
                    .min(total.saturating_sub(visible));
                let last = (first + visible).min(total);

                std::iter::once(space().height(first as f32 * row_height).into())
                    .chain(
                        self.results[first..last]
                            .iter()
                            .map(|&index| self.addon_item(index, context)),
                    )
                    .chain(std::iter::once(
                        space().height((total - last) as f32 * row_height).into(),
                    ))
                    .collect()
            }
        };

        let list = sensor(
            scrollable(column(rows))
                .id(LIST_ID.clone())
                .height(Fill)
                .on_scroll(Message::ListScrolled),
        )
        .on_resize(Message::ListResized);

        column![
            container(search).padding(SPACING_M_NUDGE),
            surface(controls),
            list
        ]
    }

    fn sort_buttons(&self) -> Element<'_, Message> {
        row(SortOption::ALL.iter().map(|&option| {
            let arrow: Element<'_, Message> = if option.0 == self.query.sort {
                let ascending = option.0.is_ascending() != self.query.reversed;
                let arrow = if ascending {
                    icon::ascending()
                } else {
                    icon::descending()
                };
                arrow.width(SORT_ARROW_SLOT).into()
            } else {
                space().width(SORT_ARROW_SLOT).into()
            };
            let label = row![option.icon(), arrow]
                .spacing(SPACING_XXS)
                .align_y(Center);

            tooltip(
                button(label)
                    .style(button::text)
                    .padding(button::DEFAULT_PADDING.top(0).bottom(0))
                    .on_press(Message::SortSelected(option)),
                text(option.to_string()),
                tooltip::Position::Top,
            )
            .style(container::rounded_box)
            .into()
        }))
        .spacing(SPACING_XS)
        .into()
    }

    fn addon_item<'a>(&'a self, index: usize, context: Context<'a>) -> Element<'a, Message> {
        column![self.addon_row(index, context), rule::horizontal(1)].into()
    }

    fn installed_auto_tip(&self, uid: &str, context: Context<'_>) -> String {
        let base = fl!("install-directly");
        let Some(dependers) = context.dependents.get(uid) else {
            return base;
        };
        let names: Vec<&str> = dependers
            .iter()
            .filter_map(|uid| {
                self.catalogue
                    .position(uid)
                    .and_then(|index| self.catalogue.addon(index))
                    .map(|addon| addon.name.as_str())
            })
            .collect();
        if names.is_empty() {
            base
        } else {
            let list: String = names.iter().map(|name| format!("\n  {name}")).collect();
            format!("{base}\n{}{list}", fl!("required-by"))
        }
    }

    fn addon_row<'a>(&'a self, index: usize, context: Context<'a>) -> Element<'a, Message> {
        let Some(addon) = self.catalogue.addon(index) else {
            return space().into();
        };

        let icon: Element<'_, Message> = match self.icons.get(&addon.category_id) {
            Some(handle) => image(handle.clone())
                .width(ICON_SIZE)
                .height(ICON_SIZE)
                .into(),
            None => space().width(ICON_SIZE).height(ICON_SIZE).into(),
        };
        let icon: Element<'_, Message> = match self.catalogue.category(&addon.category_id) {
            Some(category) => tooltip(icon, text(&category.title), tooltip::Position::Top)
                .style(container::rounded_box)
                .into(),
            None => icon,
        };

        let favourite = context.config.favourites.contains_key(&addon.uid);
        let favourite_star = tooltip(
            button(star(favourite, STAR_SIZE))
                .style(button::text)
                .padding(0)
                .on_press(Message::FavouriteToggled(index)),
            text(if favourite {
                fl!("remove-favourite")
            } else {
                fl!("add-favourite")
            }),
            tooltip::Position::Top,
        )
        .style(container::rounded_box);

        let author: Element<'a, Message> = if addon.author.is_empty() {
            secondary("").into()
        } else {
            with_icon(icon::author(), addon.author.clone()).into()
        };
        let info = column![addon_name(&addon.name)]
            .push(author)
            .push(metrics(addon));

        let selected = self.selected == Some(index);

        let directly = context.config.addons.contains_key(&addon.uid);
        let automatic = !directly && context.index.contains_key(&addon.uid);
        let install: Element<'a, Message> =
            if context.activity.installing == Some(addon.uid.as_str()) {
                container(spinner(context.activity.spinner_angle, INSTALL_ICON_SIZE))
                    .padding(button::DEFAULT_PADDING.top(0).bottom(0))
                    .into()
            } else {
                let (glyph, tip) = if directly {
                    (icon::remove(), fl!("remove"))
                } else if automatic {
                    (
                        icon::installed_auto(),
                        self.installed_auto_tip(&addon.uid, context),
                    )
                } else {
                    (icon::install(), fl!("install"))
                };
                tooltip(
                    button(glyph.size(INSTALL_ICON_SIZE))
                        .style(button::text)
                        .padding(button::DEFAULT_PADDING.top(0).bottom(0))
                        .on_press_maybe(
                            (!context.activity.busy && context.activity.can_install)
                                .then_some(Message::InstallToggled(index)),
                        ),
                    text(tip),
                    tooltip::Position::Left,
                )
                .style(container::rounded_box)
                .into()
            };

        button(stack![
            container(
                row![favourite_star, icon, info]
                    .spacing(SPACING_M_NUDGE)
                    .align_y(Center)
            )
            .width(Fill)
            .clip(true),
            container(install).align_right(Fill).center_y(Fill),
        ])
        .width(Fill)
        .style(move |theme, status| row_style(theme, status, selected))
        .on_press(Message::AddonSelected(index))
        .into()
    }

    fn details(&self) -> Element<'_, Message> {
        let Some(addon) = self.selected_addon() else {
            return center(text(fl!("select-addon"))).into();
        };

        let description: Element<'_, Message> = match &self.details {
            Details::Loading => text(fl!("loading")).into(),
            Details::Failed(error) => text(fl!("description-failed", error = error.clone())).into(),
            Details::Ready(_) => self.description.view().map(Message::Description),
        };

        let body: Element<'_, Message> = scrollable(
            column![
                column![self.addon_info(addon), rule::horizontal(1)],
                description
            ]
            .spacing(DETAILS_SPACING)
            .padding(Padding::new(DETAILS_SPACING).right(0)),
        )
        .spacing(SPACING_M_NUDGE)
        .height(Fill)
        .into();

        let body: Element<'_, Message> = match self.gallery.thumbnails(&addon.screenshots) {
            Some(thumbnails) => row![body, rule::vertical(1), thumbnails.map(Message::Gallery)]
                .spacing(SPACING_M_NUDGE)
                .into(),
            None => body,
        };

        body
    }

    fn section_tabs(&self) -> Option<Element<'_, Message>> {
        let Details::Ready(sections) = &self.details else {
            return None;
        };

        let tab = |label: String, section: Section| Tab {
            label,
            active: self.section == section,
            on_press: (!sections.body(section).is_empty())
                .then_some(Message::SectionSelected(section)),
        };

        Some(tab_bar(
            [
                tab(fl!("tab-description"), Section::Description),
                tab(fl!("tab-changelog"), Section::Changelog),
            ],
            Shrink,
            TAB_PADDING,
        ))
    }

    fn addon_info<'a>(&'a self, addon: &'a Addon) -> Element<'a, Message> {
        let title = text(&addon.name).size(SUBTITLE_TEXT_SIZE).font(Font {
            weight: font::Weight::Semibold,
            ..Font::DEFAULT
        });

        let category = self
            .catalogue
            .category(&addon.category_id)
            .map(|category| category.title.as_str());
        let facts: Vec<String> = [
            Some(format!("UID {}", addon.uid)),
            (!addon.version.is_empty())
                .then(|| fl!("addon-version", version = addon.version.clone())),
            category.map(str::to_owned),
        ]
        .into_iter()
        .flatten()
        .collect();

        let facts = row![open_page(Message::OpenPage), secondary(facts.join(" · "))]
            .spacing(SPACING_S_NUDGE)
            .align_y(Center);

        row![
            column![title, facts]
                .spacing(SPACING_S_NUDGE)
                .width(Fill)
                .padding(Padding::ZERO.bottom(DETAILS_SPACING)),
        ]
        .push(self.section_tabs())
        .spacing(SPACING_M_NUDGE)
        .align_y(Bottom)
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortOption(SortOrder);

impl SortOption {
    const ALL: [SortOption; 6] = [
        SortOption(SortOrder::Name),
        SortOption(SortOrder::Author),
        SortOption(SortOrder::Updated),
        SortOption(SortOrder::Downloads),
        SortOption(SortOrder::MonthlyDownloads),
        SortOption(SortOrder::Favourites),
    ];
}

impl SortOption {
    fn icon(self) -> iced::widget::Text<'static> {
        match self.0 {
            SortOrder::Name => icon::name(),
            SortOrder::Author => icon::author(),
            SortOrder::Updated => icon::updated(),
            SortOrder::Downloads => icon::downloads(),
            SortOrder::MonthlyDownloads => icon::monthly_downloads(),
            SortOrder::Favourites => icon::favourites(),
        }
    }
}

impl fmt::Display for SortOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self.0 {
            SortOrder::Name => fl!("sort-name"),
            SortOrder::Author => fl!("sort-author"),
            SortOrder::Downloads => fl!("sort-downloads"),
            SortOrder::MonthlyDownloads => fl!("sort-monthly-downloads"),
            SortOrder::Favourites => fl!("sort-favourites"),
            SortOrder::Updated => fl!("sort-updated"),
        })
    }
}

fn list_ratio(width: f32) -> f32 {
    if width <= 2.0 * PANEL_MIN_SIZE {
        return 0.5;
    }

    LIST_WIDTH.clamp(PANEL_MIN_SIZE, width - PANEL_MIN_SIZE) / width
}

fn row_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let palette = theme.extended_palette();
    let background = match (selected, status) {
        (true, _) => Some(palette.background.weaker.color.into()),
        (false, button::Status::Hovered | button::Status::Pressed) => {
            Some(palette.background.weakest.color.into())
        }
        (false, _) => None,
    };

    button::Style {
        background,
        text_color: palette.background.base.text,
        ..button::Style::default()
    }
}

fn addon_name(name: &str) -> iced::widget::Text<'_> {
    text(name)
        .font(Font {
            weight: font::Weight::Semibold,
            ..Font::DEFAULT
        })
        .wrapping(text::Wrapping::None)
}

fn metrics(addon: &Addon) -> Element<'_, Message> {
    row![
        metric(
            icon::updated(),
            format::date(addon.updated_ms),
            DATE_COLUMN_WIDTH
        ),
        metric(
            icon::downloads(),
            format::compact_number(addon.downloads),
            NUMBER_COLUMN_WIDTH,
        ),
        metric(
            icon::monthly_downloads(),
            format::compact_number(addon.monthly_downloads),
            NUMBER_COLUMN_WIDTH,
        ),
        metric(
            icon::favourites(),
            format::compact_number(addon.favourites),
            NUMBER_COLUMN_WIDTH,
        ),
    ]
    .into()
}

fn metric<'a>(icon: iced::widget::Text<'a>, value: String, width: f32) -> Element<'a, Message> {
    with_icon(icon, value).width(width).into()
}

fn with_icon<'a>(icon: iced::widget::Text<'a>, value: String) -> iced::widget::Row<'a, Message> {
    row![
        icon.size(CAPTION_TEXT_SIZE).style(text::secondary),
        secondary(value)
    ]
    .spacing(METRIC_ICON_SPACING)
    .align_y(Center)
}

fn filter_button<'a>(
    label: String,
    filter: Filter,
    on_change: fn(Filter) -> Message,
) -> Element<'a, Message> {
    let chip = match filter {
        Filter::Any => Chip::Off,
        Filter::Only => Chip::Included,
        Filter::Except => Chip::Excluded,
    };

    button(text(label).line_height(1.0).width(Fill).align_x(Center))
        .width(Fill)
        .style(move |theme, status| chip_style(theme, status, chip))
        .on_press(on_change(filter.next()))
        .into()
}

fn labelled<'a>(label: String, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    stack![
        center_x(control),
        container(secondary(label).line_height(1.0))
            .height(Fill)
            .align_y(Center),
    ]
    .width(Fill)
    .into()
}

fn load(client: api::Client, api: Arc<Path>) -> Task<Message> {
    Task::perform(
        async move {
            let (entries, categories) = future::join(
                store::file_list(&client, &api),
                store::categories(&client, &api),
            )
            .await;
            let categories = categories.unwrap_or_else(|error| {
                tracing::warn!("cannot load categories: {error}");
                Vec::new()
            });

            entries.map(|entries| (entries, categories))
        },
        Message::Loaded,
    )
}

fn load_icon(client: api::Client, category_id: String, url: String) -> Task<Message> {
    Task::perform(
        async move { client.download(&url).await.map(image::Handle::from_bytes) },
        move |result| Message::IconLoaded(category_id.clone(), result),
    )
}

fn load_details(client: api::Client, api: Arc<Path>, uid: String) -> Task<Message> {
    let request = uid.clone();

    Task::perform(
        async move {
            let details = store::file_details(&client, &api, &request).await?;
            let convert = |source: &str| {
                if source.trim().is_empty() {
                    String::new()
                } else {
                    bbcode::to_html(source)
                }
            };

            Ok(Sections {
                description: convert(&details.description),
                changelog: convert(&details.changelog),
            })
        },
        move |result| Message::DetailsLoaded(uid.clone(), result),
    )
}
