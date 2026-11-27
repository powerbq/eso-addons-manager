use iced::theme::palette;
use iced::widget::{
    Container, Svg, Text, button, container, row, svg, text, text_editor, text_input, tooltip,
};
use iced::{Border, Center, Color, Element, Fill, Length, Padding, Radians, Theme};

use crate::i18n::fl;
use crate::icon;

pub const CAPTION_TEXT_SIZE: f32 = 12.0;
pub const BODY_TEXT_SIZE: f32 = 14.0;
pub const SUBTITLE_TEXT_SIZE: f32 = 20.0;
pub const SPACING_XXS: f32 = 2.0;
pub const SPACING_XS: f32 = 4.0;
pub const SPACING_S_NUDGE: f32 = 6.0;
pub const SPACING_S: f32 = 8.0;
pub const SPACING_M_NUDGE: f32 = 10.0;
pub const SPACING_M: f32 = 12.0;
pub const SPACING_L: f32 = 16.0;
pub const SPACING_XL: f32 = 20.0;
pub const SPACING_XXXL: f32 = 32.0;
pub const SURFACE_PADDING: f32 = SPACING_M_NUDGE;
// Fixed height of a two-column panel header, sized from the standard scale to hold a button
// (so the custom and automatic headers always match without measuring either).
pub const HEADER_HEIGHT: f32 = SPACING_XXXL + SPACING_L;

pub const BAR_BUTTON_PADDING: Padding = Padding {
    top: SPACING_S,
    right: SPACING_M_NUDGE,
    bottom: SPACING_S,
    left: SPACING_M_NUDGE,
};

const CHIP_RADIUS: f32 = 4.0;
const BUTTON_RADIUS: f32 = 2.0;
const DISABLED_TEXT_OPACITY: f32 = 0.5;
const FIELD_BACKGROUND_MIX: f32 = 0.2;
const PLACEHOLDER_MIX: f32 = 0.2;
pub const TAB_PADDING: f32 = SPACING_M_NUDGE;
const TAB_RECESS: f32 = SPACING_XXS;
const TAB_OUTLINE: f32 = 1.0;
const TAB_OUTLINE_SHADE: f32 = 0.8;

pub struct Tab<Message> {
    pub label: String,
    pub active: bool,
    pub on_press: Option<Message>,
}

pub fn tab_bar<'a, Message: Clone + 'a>(
    tabs: impl IntoIterator<Item = Tab<Message>>,
    width: Length,
    padding: f32,
) -> Element<'a, Message> {
    let tab = |tab: Tab<Message>| -> Element<'a, Message> {
        let active = tab.active;
        let recess = if active { 0.0 } else { TAB_RECESS };

        let button = button(single_line(tab.label).width(width).align_x(Center))
            .width(width)
            .padding([padding - recess / 2.0, TAB_PADDING])
            .style(move |theme, status| tab_style(theme, status, active))
            .on_press_maybe(tab.on_press);

        container(
            container(button)
                .padding(Padding::ZERO.top(TAB_OUTLINE))
                .style(tab_outline),
        )
        .width(width)
        .padding(Padding::ZERO.top(recess))
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.base.color.into()),
            ..container::Style::default()
        })
        .into()
    };

    container(row(tabs.into_iter().map(tab)).spacing(TAB_OUTLINE))
        .padding(
            Padding::ZERO
                .bottom(TAB_OUTLINE)
                .left(TAB_OUTLINE)
                .right(TAB_OUTLINE),
        )
        .style(tab_outline)
        .into()
}

fn tab_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let shades = theme.extended_palette().background;
    let shade = if active { shades.weak } else { shades.weaker };
    let text_color = match status {
        button::Status::Disabled => shade.text.scale_alpha(DISABLED_TEXT_OPACITY),
        _ => shade.text,
    };

    button::Style {
        background: Some(shade.color.into()),
        text_color,
        ..button::Style::default()
    }
}

fn tab_outline(theme: &Theme) -> container::Style {
    let base = theme.extended_palette().background.base.color;

    container::Style {
        background: Some(
            Color {
                r: base.r * TAB_OUTLINE_SHADE,
                g: base.g * TAB_OUTLINE_SHADE,
                b: base.b * TAB_OUTLINE_SHADE,
                a: base.a,
            }
            .into(),
        ),
        ..container::Style::default()
    }
}

const SPINNER: &[u8] = include_bytes!("spinner.svg");

pub fn open_page<'a, Message: Clone + 'a>(on_press: Message) -> Element<'a, Message> {
    tooltip(
        button(icon::open_page())
            .style(button::text)
            .padding(0)
            .on_press(on_press),
        text(fl!("open-page")),
        tooltip::Position::Left,
    )
    .style(container::rounded_box)
    .into()
}

pub fn open_url(url: &str) {
    tracing::debug!("opening {url}");
    if let Err(error) = open::that_detached(url) {
        tracing::warn!("cannot open {url}: {error}");
    }
}

pub fn open_folder(path: &std::path::Path) {
    tracing::debug!("opening folder {}", path.display());
    if let Err(error) = open::that_detached(path) {
        tracing::warn!("cannot open folder {}: {error}", path.display());
    }
}

pub fn surface<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message> {
    container(content)
        .width(Fill)
        .padding(SURFACE_PADDING)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.weakest.color.into()),
            ..container::Style::default()
        })
}

pub fn secondary<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content)
        .size(CAPTION_TEXT_SIZE)
        .style(text::secondary)
        .wrapping(text::Wrapping::None)
}

pub fn spinner<'a>(angle: f32, size: f32) -> Svg<'a> {
    svg(svg::Handle::from_memory(SPINNER))
        .width(size)
        .height(size)
        .rotation(Radians(angle))
        .style(|theme: &Theme, _status| svg::Style {
            color: Some(theme.extended_palette().secondary.base.color),
        })
}

const STAR_FILLED: &[u8] = include_bytes!("star-filled.svg");
const STAR_OUTLINE: &[u8] = include_bytes!("star-outline.svg");

pub fn star<'a>(favourite: bool, size: f32) -> Svg<'a> {
    let handle = if favourite { STAR_FILLED } else { STAR_OUTLINE };

    svg(svg::Handle::from_memory(handle))
        .width(size)
        .height(size)
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(if favourite {
                theme.palette().warning
            } else {
                theme.extended_palette().secondary.base.color
            }),
        })
}

pub fn single_line<'a>(label: impl text::IntoFragment<'a>) -> Text<'a> {
    text(label).wrapping(text::Wrapping::None)
}

pub fn button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let shades = palette.background;
    let background = match status {
        button::Status::Hovered => shades.neutral.color,
        button::Status::Pressed => shades.weaker.color,
        button::Status::Active | button::Status::Disabled => shades.weak.color,
    };
    let text_color = match status {
        button::Status::Disabled => shades.weak.text.scale_alpha(DISABLED_TEXT_OPACITY),
        _ => shades.weak.text,
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border::default()
            .width(1)
            .color(shades.base.color)
            .rounded(BUTTON_RADIUS),
        ..button::Style::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chip {
    Off,
    Included,
    Excluded,
}

pub fn chip_style(theme: &Theme, _status: button::Status, chip: Chip) -> button::Style {
    let palette = theme.extended_palette();
    let (background, border, text_color) = match chip {
        Chip::Included => (
            Some(palette.primary.base.color.into()),
            palette.primary.base.color,
            palette.primary.base.text,
        ),
        Chip::Excluded => (
            Some(palette.danger.base.color.into()),
            palette.danger.base.color,
            palette.danger.base.text,
        ),
        Chip::Off => (
            None,
            palette.background.strongest.color,
            palette.secondary.strong.color,
        ),
    };

    button::Style {
        background,
        text_color,
        border: Border::default()
            .width(1)
            .color(border)
            .rounded(CHIP_RADIUS),
        ..button::Style::default()
    }
}

pub fn inverted_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = theme.extended_palette().background.base;
    let background = palette::mix(base.text, base.color, FIELD_BACKGROUND_MIX);
    let default = text_input::default(theme, status);

    text_input::Style {
        background: background.into(),
        border: default.border.color(background),
        icon: base.color,
        placeholder: palette::mix(base.color, background, PLACEHOLDER_MIX),
        value: base.color,
        ..default
    }
}

pub fn inverted_editor(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let base = theme.extended_palette().background.base;
    let background = palette::mix(base.text, base.color, FIELD_BACKGROUND_MIX);
    let default = text_editor::default(theme, status);

    text_editor::Style {
        background: background.into(),
        border: default.border.color(background),
        placeholder: palette::mix(base.color, background, PLACEHOLDER_MIX),
        value: base.color,
        ..default
    }
}

pub fn plain_editor(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let default = text_editor::default(theme, status);

    text_editor::Style {
        border: default.border.color(Color::TRANSPARENT),
        ..default
    }
}

// A container filled like the read-only log field — the same default text-editor background with a
// transparent border, matching `plain_editor` — for read-only regions that should read as an inert
// field rather than plain surface.
pub fn field_fill(theme: &Theme) -> container::Style {
    let editor = text_editor::default(theme, text_editor::Status::Active);

    container::Style {
        background: Some(editor.background),
        border: editor.border.color(Color::TRANSPARENT),
        ..container::Style::default()
    }
}
