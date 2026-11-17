use std::collections::HashMap;

use eso_addons_manager_core::{Screenshot, api};

use iced::keyboard::{self, key};
use iced::widget::{
    button, center, center_x, column, container, image, mouse_area, opaque, responsive, row,
    scrollable, sensor, space, text,
};
use iced::{Border, Center, Element, Fill, Padding, Shrink, Size, Subscription, Task, Theme};

use crate::i18n::fl;
use crate::modal::backdrop;
use crate::widget::{SPACING_M, SPACING_M_NUDGE, SPACING_XL, SPACING_XXXL};

const THUMBNAIL_BOX: f32 = 100.0;
const THUMBNAIL_MARGIN: f32 = SPACING_M_NUDGE;
const ANTICIPATE: f32 = 200.0;
const OVERLAY_MARGIN: f32 = SPACING_XXXL;
const FRAME_WIDTH: f32 = 4.0;
const FRAME_RADIUS: f32 = 8.0;
const VIEWER_SPACING: f32 = SPACING_M_NUDGE;
const NAVIGATION_PADDING: [f32; 2] = [SPACING_M, SPACING_XL];
const COUNTER_PADDING: f32 = SPACING_M;
const COUNTER_HEIGHT: f32 = 30.0;
const COUNTER_OPACITY: f32 = 0.8;
const DISABLED_OPACITY: f32 = 0.5;

#[derive(Default)]
pub struct Gallery {
    thumbnails: HashMap<String, Picture<image::Handle>>,
    screenshots: HashMap<String, Picture<Full>>,
    viewer: Option<Viewer>,
    navigation: Size,
}

enum Picture<T> {
    Loading,
    Ready(T),
    Failed,
}

#[derive(Debug, Clone)]
pub struct Full {
    handle: image::Handle,
    size: Size,
}

struct Viewer {
    screenshots: Vec<Screenshot>,
    position: usize,
}

#[derive(Debug, Clone)]
pub enum Message {
    ThumbnailShown(String),
    ThumbnailLoaded(String, Result<image::Handle, String>),
    ScreenshotLoaded(String, Result<Full, String>),
    NavigationMeasured(Size),
    Open(Vec<Screenshot>, usize),
    Previous,
    Next,
    Close,
}

impl Gallery {
    pub fn update(&mut self, client: &api::Client, message: Message) -> Task<Message> {
        match message {
            Message::ThumbnailShown(url) => {
                if self.thumbnails.contains_key(&url) {
                    return Task::none();
                }

                self.thumbnails.insert(url.clone(), Picture::Loading);
                download(
                    client.clone(),
                    url,
                    decode_thumbnail,
                    Message::ThumbnailLoaded,
                )
            }
            Message::ThumbnailLoaded(url, result) => {
                self.thumbnails.insert(url.clone(), picture(&url, result));
                Task::none()
            }
            Message::ScreenshotLoaded(url, result) => {
                self.screenshots.insert(url.clone(), picture(&url, result));
                Task::none()
            }
            Message::Open(screenshots, position) => {
                self.viewer = Some(Viewer {
                    screenshots,
                    position,
                });
                self.load_current(client)
            }
            Message::Previous => {
                if let Some(viewer) = &mut self.viewer {
                    viewer.position = viewer.position.saturating_sub(1);
                }
                self.load_current(client)
            }
            Message::Next => {
                if let Some(viewer) = &mut self.viewer {
                    viewer.position = (viewer.position + 1).min(viewer.screenshots.len() - 1);
                }
                self.load_current(client)
            }
            Message::Close => {
                self.viewer = None;
                Task::none()
            }
            Message::NavigationMeasured(size) => {
                self.navigation = size;
                Task::none()
            }
        }
    }

    pub fn thumbnails<'a>(&'a self, screenshots: &'a [Screenshot]) -> Option<Element<'a, Message>> {
        if screenshots.is_empty() {
            return None;
        }

        let thumbnails = column(
            screenshots
                .iter()
                .enumerate()
                .map(|(position, screenshot)| {
                    let url = &screenshot.thumbnail;

                    let content: Element<'_, Message> = match self.thumbnails.get(url) {
                        Some(Picture::Ready(thumbnail)) => image(thumbnail.clone()).into(),
                        Some(Picture::Failed) => placeholder("×"),
                        Some(Picture::Loading) => placeholder("…"),
                        None => sensor(placeholder("…"))
                            .key(url.clone())
                            .anticipate(ANTICIPATE)
                            .on_show(move |_| Message::ThumbnailShown(url.clone()))
                            .into(),
                    };

                    button(content)
                        .padding(0)
                        .style(button::text)
                        .on_press(Message::Open(screenshots.to_vec(), position))
                        .into()
                }),
        )
        .spacing(SPACING_M_NUDGE)
        .padding(Padding::new(THUMBNAIL_MARGIN).left(0))
        .width(THUMBNAIL_BOX + THUMBNAIL_MARGIN)
        .align_x(Center);

        Some(
            scrollable(thumbnails)
                .spacing(SPACING_M_NUDGE)
                .height(Fill)
                .into(),
        )
    }

    pub fn overlay(&self) -> Option<Element<'_, Message>> {
        let viewer = self.viewer.as_ref()?;
        let screenshot = viewer.screenshots.get(viewer.position)?;
        let position = viewer.position;
        let count = viewer.screenshots.len();

        let content: Element<'_, Message> = match self.screenshots.get(&screenshot.full) {
            Some(Picture::Ready(full)) => {
                let full = full.clone();
                let navigation = self.navigation;

                responsive(move |available| {
                    viewer_layout(&full, available, navigation, position, count)
                })
                .into()
            }
            Some(Picture::Failed) => center(text(fl!("screenshot-failed"))).into(),
            _ => center(text(fl!("loading"))).into(),
        };

        Some(opaque(
            mouse_area(
                container(content)
                    .width(Fill)
                    .height(Fill)
                    .padding(OVERLAY_MARGIN)
                    .style(backdrop),
            )
            .on_press(Message::Close),
        ))
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.viewer.is_none() {
            return Subscription::none();
        }

        keyboard::listen().filter_map(|event| match event {
            keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(named),
                ..
            } => match named {
                key::Named::Escape => Some(Message::Close),
                key::Named::ArrowLeft => Some(Message::Previous),
                key::Named::ArrowRight => Some(Message::Next),
                _ => None,
            },
            _ => None,
        })
    }

    fn load_current(&mut self, client: &api::Client) -> Task<Message> {
        let Some(url) = self
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.screenshots.get(viewer.position))
            .map(|screenshot| screenshot.full.clone())
        else {
            return Task::none();
        };

        if self.screenshots.contains_key(&url) {
            return Task::none();
        }

        self.screenshots.insert(url.clone(), Picture::Loading);
        download(
            client.clone(),
            url,
            decode_screenshot,
            Message::ScreenshotLoaded,
        )
    }
}

fn viewer_layout<'a>(
    full: &Full,
    available: Size,
    navigation: Size,
    position: usize,
    count: usize,
) -> Element<'a, Message> {
    let frame = 2.0 * FRAME_WIDTH;
    let space_for_picture = Size::new(
        available.width - 2.0 * (navigation.width + VIEWER_SPACING) - frame,
        available.height - COUNTER_HEIGHT - VIEWER_SPACING - frame,
    );
    let scale = (space_for_picture.width / full.size.width)
        .min(space_for_picture.height / full.size.height)
        .clamp(0.0, 1.0);

    let picture = container(
        image(full.handle.clone())
            .width(full.size.width * scale)
            .height(full.size.height * scale),
    )
    .padding(FRAME_WIDTH)
    .style(|theme: &Theme| container::Style {
        border: Border {
            width: FRAME_WIDTH,
            radius: FRAME_RADIUS.into(),
            color: theme.extended_palette().background.weak.color,
        },
        ..container::Style::default()
    });

    let previous = navigation_button(fl!("previous"), (position > 0).then_some(Message::Previous));
    let next = navigation_button(fl!("next"), (position + 1 < count).then_some(Message::Next));

    let pictures = row![
        sensor(opaque(previous)).on_resize(Message::NavigationMeasured),
        opaque(picture),
        opaque(next),
    ]
    .spacing(VIEWER_SPACING)
    .align_y(Center);

    let footer_row = row![
        space().width(navigation.width),
        center_x(
            container(text(format!("{} / {count}", position + 1)))
                .height(COUNTER_HEIGHT)
                .align_y(Center)
                .padding([0.0, COUNTER_PADDING])
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();

                    container::Style {
                        background: Some(
                            palette
                                .background
                                .base
                                .color
                                .scale_alpha(COUNTER_OPACITY)
                                .into(),
                        ),
                        text_color: Some(palette.background.base.text),
                        border: Border::default().rounded(FRAME_RADIUS),
                        ..container::Style::default()
                    }
                })
        ),
        space().width(navigation.width),
    ]
    .spacing(VIEWER_SPACING);

    center(
        column![pictures, footer_row]
            .spacing(VIEWER_SPACING)
            .width(Shrink),
    )
    .into()
}

fn navigation_button<'a>(label: String, message: Option<Message>) -> button::Button<'a, Message> {
    button(text(label))
        .padding(NAVIGATION_PADDING)
        .style(navigation_style)
        .on_press_maybe(message)
}

fn navigation_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let shade = palette.background.weaker;
    let background = match status {
        button::Status::Active => shade.color.scale_alpha(COUNTER_OPACITY),
        button::Status::Hovered => shade.color,
        button::Status::Pressed => palette.background.neutral.color,
        button::Status::Disabled => shade.color.scale_alpha(DISABLED_OPACITY),
    };
    let text_color = match status {
        button::Status::Disabled => shade.text.scale_alpha(DISABLED_OPACITY),
        _ => shade.text,
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border::default().rounded(FRAME_RADIUS),
        ..button::Style::default()
    }
}

fn placeholder(label: &str) -> Element<'_, Message> {
    center(text(label))
        .width(THUMBNAIL_BOX)
        .height(THUMBNAIL_BOX)
        .into()
}

fn picture<T>(url: &str, result: Result<T, String>) -> Picture<T> {
    match result {
        Ok(handle) => Picture::Ready(handle),
        Err(error) => {
            tracing::warn!("cannot load screenshot {url}: {error}");
            Picture::Failed
        }
    }
}

fn download<T: Send + 'static>(
    client: api::Client,
    url: String,
    decode: fn(Vec<u8>) -> Result<T, String>,
    on_loaded: fn(String, Result<T, String>) -> Message,
) -> Task<Message> {
    Task::perform(
        {
            let url = url.clone();
            async move {
                let bytes = client
                    .download(&url)
                    .await
                    .map_err(|error| error.to_string())?;
                decode(bytes)
            }
        },
        move |result| on_loaded(url.clone(), result),
    )
}

fn decode_thumbnail(bytes: Vec<u8>) -> Result<image::Handle, String> {
    Ok(image::Handle::from_bytes(bytes))
}

fn decode_screenshot(bytes: Vec<u8>) -> Result<Full, String> {
    let picture = ::image::load_from_memory(&bytes)
        .map_err(|error| error.to_string())?
        .to_rgba8();
    let (width, height) = picture.dimensions();

    Ok(Full {
        handle: image::Handle::from_rgba(width, height, picture.into_raw()),
        size: Size::new(width as f32, height as f32),
    })
}
