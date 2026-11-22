use std::collections::{HashMap, HashSet};

use eso_addons_manager_core::api;
use iced::futures::future;
use iced::widget::{column, image, mouse_area, sensor, space, text};
use iced::{ContentFit, Element, Fill, Point, Size, Task, mouse};

use crate::html;
use crate::i18n::fl;

const TILE_HEIGHT: u32 = 1024;
const ANTICIPATE: f32 = 512.0;

pub struct Description {
    engine: html::Engine,
    width: u32,
    html: Option<String>,
    pending: Option<String>,
    layout: Option<html::Layout>,
    laid_out_width: u32,
    busy: bool,
    failed: bool,
    tiles: HashMap<u32, image::Handle>,
    requested: HashSet<u32>,
    images: HashSet<String>,
    cursor: Option<Point>,
    pointer: bool,
    scale: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    Resized(Size),
    LaidOut(u32, Option<html::Layout>),
    TileShown(u64, u32),
    TileDrawn(u64, u32, Option<image::Handle>),
    ImagesLoaded(Vec<(String, Vec<u8>)>),
    Moved(Point),
    Pressed,
    Clicked(Option<String>),
    Hovered(u64, bool),
}

pub enum Action {
    None,
    Run(Task<Message>),
    OpenUrl(String),
}

impl Description {
    pub fn new(engine: html::Engine, scale: f32) -> Self {
        Self {
            engine,
            width: 0,
            html: None,
            pending: None,
            layout: None,
            laid_out_width: 0,
            busy: false,
            failed: false,
            tiles: HashMap::new(),
            requested: HashSet::new(),
            images: HashSet::new(),
            cursor: None,
            pointer: false,
            scale,
        }
    }

    pub fn set_scale(&mut self, scale: f32) -> Task<Message> {
        if (scale - self.scale).abs() < f32::EPSILON {
            return Task::none();
        }

        self.scale = scale;
        self.tiles.clear();
        self.requested.clear();

        if self.html.is_some() {
            self.pending = self.html.clone();
            self.start_layout()
        } else {
            Task::none()
        }
    }

    pub fn show(&mut self, html: String) -> Task<Message> {
        self.layout = None;
        self.tiles.clear();
        self.requested.clear();

        self.replace(html)
    }

    pub fn replace(&mut self, html: String) -> Task<Message> {
        self.pointer = false;
        self.failed = false;
        self.html = Some(html.clone());
        self.pending = Some(html);

        self.start_layout()
    }

    pub fn update(&mut self, client: &api::Client, message: Message) -> Action {
        match message {
            Message::Resized(size) => {
                let width = size.width.floor().max(1.0) as u32;
                if width == self.width {
                    return Action::None;
                }

                self.width = width;
                Action::Run(self.start_layout())
            }
            Message::LaidOut(width, layout) => {
                self.busy = false;

                if self.pending.is_some() {
                    return Action::Run(self.start_layout());
                }

                let Some(layout) = layout else {
                    self.failed = self.layout.is_none();
                    return Action::Run(self.start_layout());
                };

                self.laid_out_width = width;
                self.tiles.clear();
                self.requested.clear();

                let images: Vec<String> = layout
                    .images
                    .iter()
                    .filter(|url| self.images.insert((*url).clone()))
                    .cloned()
                    .collect();
                self.layout = Some(layout);

                let downloads = if images.is_empty() {
                    Task::none()
                } else {
                    download(client.clone(), images)
                };

                Action::Run(Task::batch([downloads, self.start_layout()]))
            }
            Message::TileShown(generation, index) => {
                let Some(layout) = &self.layout else {
                    return Action::None;
                };
                if layout.generation != generation
                    || self.tiles.contains_key(&index)
                    || !self.requested.insert(index)
                {
                    return Action::None;
                }

                let top = index * TILE_HEIGHT;
                let height = TILE_HEIGHT.min(layout.height.saturating_sub(top));

                Action::Run(Task::perform(
                    self.engine
                        .clone()
                        .draw(generation, top, height, self.scale),
                    move |tile| Message::TileDrawn(generation, index, tile),
                ))
            }
            Message::TileDrawn(generation, index, tile) => {
                if self.current_generation() == Some(generation) {
                    match tile {
                        Some(tile) => {
                            self.tiles.insert(index, tile);
                        }
                        None => {
                            self.requested.remove(&index);
                        }
                    }
                }
                Action::None
            }
            Message::ImagesLoaded(images) => {
                if images.is_empty() || self.layout.is_none() {
                    return Action::None;
                }

                self.busy = true;
                let width = self.width;

                Action::Run(Task::perform(
                    self.engine.clone().add_images(images),
                    move |layout| Message::LaidOut(width, layout),
                ))
            }
            Message::Moved(position) => {
                self.cursor = Some(position);

                let Some(generation) = self.current_generation() else {
                    return Action::None;
                };

                Action::Run(Task::perform(
                    self.engine
                        .clone()
                        .hover(generation, position.x, position.y),
                    move |pointer| Message::Hovered(generation, pointer),
                ))
            }
            Message::Hovered(generation, pointer) => {
                if self.current_generation() == Some(generation) {
                    self.pointer = pointer;
                }
                Action::None
            }
            Message::Pressed => {
                let (Some(generation), Some(position)) = (self.current_generation(), self.cursor)
                else {
                    return Action::None;
                };

                Action::Run(Task::perform(
                    self.engine
                        .clone()
                        .click(generation, position.x, position.y),
                    Message::Clicked,
                ))
            }
            Message::Clicked(Some(url)) => Action::OpenUrl(url),
            Message::Clicked(None) => Action::None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match &self.layout {
            Some(layout) => {
                let generation = layout.generation;
                let count = layout.height.div_ceil(TILE_HEIGHT);

                let tiles = column((0..count).map(|index| {
                    let height = TILE_HEIGHT.min(layout.height - index * TILE_HEIGHT) as f32;

                    match self.tiles.get(&index) {
                        Some(tile) => {
                            let native = (self.scale - 1.0).abs() < f32::EPSILON;
                            image(tile.clone())
                                .width(self.laid_out_width as f32)
                                .height(height)
                                .content_fit(if native {
                                    ContentFit::None
                                } else {
                                    ContentFit::Fill
                                })
                                .filter_method(if native {
                                    image::FilterMethod::Nearest
                                } else {
                                    image::FilterMethod::Linear
                                })
                                .into()
                        }
                        None => sensor(space().width(Fill).height(height))
                            .key((generation, index))
                            .anticipate(ANTICIPATE)
                            .on_show(move |_| Message::TileShown(generation, index))
                            .into(),
                    }
                }))
                .width(Fill);

                mouse_area(tiles)
                    .on_move(Message::Moved)
                    .on_press(Message::Pressed)
                    .interaction(if self.pointer {
                        mouse::Interaction::Pointer
                    } else {
                        mouse::Interaction::Idle
                    })
                    .into()
            }
            None if self.failed => text(fl!("description-render-failed")).width(Fill).into(),
            None => text(fl!("loading")).width(Fill).into(),
        };

        sensor(content).on_resize(Message::Resized).into()
    }

    fn current_generation(&self) -> Option<u64> {
        self.layout.as_ref().map(|layout| layout.generation)
    }

    fn start_layout(&mut self) -> Task<Message> {
        if self.busy || self.width == 0 {
            return Task::none();
        }

        let width = self.width;
        let engine = self.engine.clone();

        if let Some(html) = self.pending.take() {
            self.busy = true;
            let scale = self.scale;
            return Task::perform(engine.load(html, width, scale), move |layout| {
                Message::LaidOut(width, layout)
            });
        }

        if self.layout.is_some() && self.laid_out_width != width {
            self.busy = true;
            return Task::perform(engine.resize(width), move |layout| {
                Message::LaidOut(width, layout)
            });
        }

        Task::none()
    }
}

fn download(client: api::Client, urls: Vec<String>) -> Task<Message> {
    Task::perform(
        async move {
            let downloads = urls.into_iter().map(|url| {
                let client = client.clone();
                async move {
                    match client.download(&url).await {
                        Ok(bytes) => Some((url, bytes)),
                        Err(error) => {
                            tracing::warn!("cannot load picture {url}: {error}");
                            None
                        }
                    }
                }
            });

            future::join_all(downloads)
                .await
                .into_iter()
                .flatten()
                .collect()
        },
        Message::ImagesLoaded,
    )
}
