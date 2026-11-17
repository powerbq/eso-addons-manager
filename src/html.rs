mod container;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

use iced::futures::channel::oneshot;
use iced::widget::image;
use iced::{Color, Theme};
use litehtml::pixbuf::PixbufContainer;
use litehtml::{Document, DrawContext, Position};

use self::container::Shared;
use crate::widget::BODY_TEXT_SIZE;

const MASTER_CSS: &str = include_str!("html/master.css");
const THEME_CSS: &str = include_str!("html/theme.css");

#[derive(Debug, Clone)]
pub struct Engine {
    commands: mpsc::Sender<Command>,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub generation: u64,
    pub height: u32,
    pub images: Vec<String>,
}

enum Command {
    Load {
        html: String,
        width: u32,
        scale: f32,
        reply: oneshot::Sender<Option<Layout>>,
    },
    Resize {
        width: u32,
        reply: oneshot::Sender<Option<Layout>>,
    },
    AddImages {
        images: Vec<(String, Vec<u8>)>,
        reply: oneshot::Sender<Option<Layout>>,
    },
    Draw {
        generation: u64,
        top: u32,
        height: u32,
        scale: f32,
        reply: oneshot::Sender<Option<image::Handle>>,
    },
    Click {
        generation: u64,
        x: f32,
        y: f32,
        reply: oneshot::Sender<Option<String>>,
    },
    Hover {
        generation: u64,
        x: f32,
        y: f32,
        reply: oneshot::Sender<bool>,
    },
}

impl Engine {
    pub fn spawn(theme: &Theme) -> Self {
        let (commands, receiver) = mpsc::channel();
        let css = theme_css(theme);

        thread::Builder::new()
            .name("html".to_owned())
            .spawn(move || run(receiver, &css))
            .expect("cannot start the HTML rendering thread");

        Self { commands }
    }

    pub async fn load(self, html: String, width: u32, scale: f32) -> Option<Layout> {
        self.request(|reply| Command::Load {
            html,
            width,
            scale,
            reply,
        })
        .await
        .flatten()
    }

    pub async fn resize(self, width: u32) -> Option<Layout> {
        self.request(|reply| Command::Resize { width, reply })
            .await
            .flatten()
    }

    pub async fn add_images(self, images: Vec<(String, Vec<u8>)>) -> Option<Layout> {
        self.request(|reply| Command::AddImages { images, reply })
            .await
            .flatten()
    }

    pub async fn draw(
        self,
        generation: u64,
        top: u32,
        height: u32,
        scale: f32,
    ) -> Option<image::Handle> {
        self.request(|reply| Command::Draw {
            generation,
            top,
            height,
            scale,
            reply,
        })
        .await
        .flatten()
    }

    pub async fn click(self, generation: u64, x: f32, y: f32) -> Option<String> {
        self.request(|reply| Command::Click {
            generation,
            x,
            y,
            reply,
        })
        .await
        .flatten()
    }

    pub async fn hover(self, generation: u64, x: f32, y: f32) -> bool {
        self.request(|reply| Command::Hover {
            generation,
            x,
            y,
            reply,
        })
        .await
        .unwrap_or(false)
    }

    async fn request<T>(&self, command: impl FnOnce(oneshot::Sender<T>) -> Command) -> Option<T> {
        let (reply, response) = oneshot::channel();
        self.commands.send(command(reply)).ok()?;
        response.await.ok()
    }
}

fn run(receiver: mpsc::Receiver<Command>, theme_css: &str) {
    let canvas = Rc::new(RefCell::new(PixbufContainer::new(1, 1)));
    let mut originals: HashMap<String, Vec<u8>> = HashMap::new();
    let mut image_scale = 1.0;
    let mut generation = 0;
    let mut next = None;

    loop {
        let command = match next.take() {
            Some(command) => command,
            None => match receiver.recv() {
                Ok(command) => command,
                Err(_) => return,
            },
        };

        let Command::Load {
            html,
            mut width,
            scale,
            reply,
        } = command
        else {
            reject(command);
            continue;
        };

        canvas.borrow_mut().clear_pending_images();
        canvas
            .borrow_mut()
            .resize_with_scale(width.max(1), 1, scale);

        if (scale - image_scale).abs() > f32::EPSILON {
            image_scale = scale;
            for (url, bytes) in &originals {
                let prepared = prepare_image(bytes, scale);
                canvas
                    .borrow_mut()
                    .load_image_data(url, prepared.as_deref().unwrap_or(bytes));
            }
        }

        let mut container = Shared::new(canvas.clone());
        let mut document =
            match Document::from_html(&html, &mut container, Some(MASTER_CSS), Some(theme_css)) {
                Ok(document) => document,
                Err(error) => {
                    tracing::warn!("cannot parse description HTML: {error:?}");
                    let _ = reply.send(None);
                    continue;
                }
            };

        generation += 1;
        let _ = reply.send(Some(layout(&mut document, &canvas, width, generation)));

        while let Ok(command) = receiver.recv() {
            match command {
                Command::Load { .. } => {
                    next = Some(command);
                    break;
                }
                Command::Resize {
                    width: new_width,
                    reply,
                } => {
                    width = new_width;
                    canvas.borrow_mut().resize(width.max(1), 1);
                    generation += 1;
                    let _ = reply.send(Some(layout(&mut document, &canvas, width, generation)));
                }
                Command::AddImages { images, reply } => {
                    for (url, bytes) in images {
                        let prepared = prepare_image(&bytes, image_scale);
                        canvas
                            .borrow_mut()
                            .load_image_data(&url, prepared.as_deref().unwrap_or(&bytes));
                        originals.insert(url, bytes);
                    }
                    generation += 1;
                    let _ = reply.send(Some(layout(&mut document, &canvas, width, generation)));
                }
                Command::Draw {
                    generation: requested,
                    top,
                    height,
                    scale,
                    reply,
                } => {
                    if requested != generation || width == 0 || height == 0 {
                        let _ = reply.send(None);
                        continue;
                    }

                    canvas.borrow_mut().resize_with_scale(width, height, scale);
                    document.draw(
                        DrawContext::default(),
                        0.0,
                        -(top as f32),
                        Some(Position {
                            x: 0.0,
                            y: 0.0,
                            width: width as f32,
                            height: height as f32,
                        }),
                    );

                    let canvas = canvas.borrow();
                    let _ = reply.send(Some(image::Handle::from_rgba(
                        canvas.width(),
                        canvas.height(),
                        canvas.pixels().to_vec(),
                    )));
                }
                Command::Click {
                    generation: requested,
                    x,
                    y,
                    reply,
                } => {
                    if requested != generation {
                        let _ = reply.send(None);
                        continue;
                    }

                    let _ = document.on_lbutton_down(x, y, x, y);
                    let _ = document.on_lbutton_up(x, y, x, y);
                    let _ = reply.send(canvas.borrow_mut().take_anchor_click());
                }
                Command::Hover {
                    generation: requested,
                    x,
                    y,
                    reply,
                } => {
                    if requested != generation {
                        let _ = reply.send(false);
                        continue;
                    }

                    let _ = document.on_mouse_over(x, y, x, y);
                    let _ = reply.send(canvas.borrow().cursor() == "pointer");
                }
            }
        }
    }
}

fn layout(
    document: &mut Document<'_>,
    canvas: &Rc<RefCell<PixbufContainer>>,
    width: u32,
    generation: u64,
) -> Layout {
    let _ = document.render(width as f32);
    let height = document.height().ceil().max(1.0) as u32;
    let images = canvas
        .borrow_mut()
        .take_pending_images()
        .into_iter()
        .map(|(url, _)| url)
        .collect();

    Layout {
        generation,
        height,
        images,
    }
}

fn prepare_image(data: &[u8], scale: f32) -> Option<Vec<u8>> {
    if (scale - 1.0).abs() < f32::EPSILON {
        return None;
    }

    let image = ::image::load_from_memory(data).ok()?;
    let width = ((image.width() as f32) * scale).round().max(1.0) as u32;
    let height = ((image.height() as f32) * scale).round().max(1.0) as u32;
    let resized = image.resize_exact(width, height, ::image::imageops::FilterType::Lanczos3);

    let mut buffer = Vec::new();
    resized
        .write_to(&mut Cursor::new(&mut buffer), ::image::ImageFormat::Png)
        .ok()?;
    Some(buffer)
}

fn reject(command: Command) {
    match command {
        Command::Load { reply, .. }
        | Command::Resize { reply, .. }
        | Command::AddImages { reply, .. } => {
            let _ = reply.send(None);
        }
        Command::Draw { reply, .. } => {
            let _ = reply.send(None);
        }
        Command::Click { reply, .. } => {
            let _ = reply.send(None);
        }
        Command::Hover { reply, .. } => {
            let _ = reply.send(false);
        }
    }
}

fn theme_css(theme: &Theme) -> String {
    let palette = theme.extended_palette();

    [
        ("{background}", palette.background.base.color),
        ("{text}", palette.background.base.text),
        ("{link}", palette.primary.strong.color),
        ("{code}", palette.background.weakest.color),
        ("{line}", palette.background.strong.color),
    ]
    .into_iter()
    .fold(THEME_CSS.to_owned(), |css, (name, color)| {
        css.replace(name, &css_color(color))
    })
    .replace("{size}", &BODY_TEXT_SIZE.to_string())
}

fn css_color(color: Color) -> String {
    let [red, green, blue, _] = color.into_rgba8();
    format!("#{red:02x}{green:02x}{blue:02x}")
}
