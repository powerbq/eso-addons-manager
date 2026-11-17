use iced::widget::{button, column, container, mouse_area, row, rule, sensor, text};
use iced::{Border, Center, Element, Fill, Length, Shrink, Size, Theme};

use crate::icon;
use crate::modal::dialog_style;
use crate::widget::{
    BAR_BUTTON_PADDING, CAPTION_TEXT_SIZE, SPACING_L, SPACING_M_NUDGE, SPACING_S, SPACING_S_NUDGE,
    SPACING_XS, secondary,
};

const MARK_WIDTH: f32 = SPACING_L;
const DISABLED_TEXT_OPACITY: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Menu {
    Settings,
}

pub const MENUS: [Menu; 1] = [Menu::Settings];

pub struct Item<Message> {
    pub label: String,
    pub on_press: Option<Message>,
    pub checked: bool,
}

pub enum Entry<Message> {
    Header(String),
    Separator,
    Item(Item<Message>),
}

pub fn title<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    open: bool,
    on_press: Message,
    on_enter: Message,
    on_resize: impl Fn(Size) -> Message + 'a,
) -> Element<'a, Message> {
    let title = button(content)
        .padding(BAR_BUTTON_PADDING)
        .style(move |theme, status| title_style(theme, status, open))
        .on_press(on_press);

    sensor(mouse_area(title).on_enter(on_enter))
        .on_resize(on_resize)
        .into()
}

pub fn list<'a, Message: Clone + 'a>(
    entries: Vec<Entry<Message>>,
    width: Option<f32>,
    on_resize: impl Fn(Size) -> Message + 'a,
) -> Element<'a, Message> {
    let checkable = entries
        .iter()
        .any(|entry| matches!(entry, Entry::Item(item) if item.checked));

    let rows = entries
        .into_iter()
        .map(move |entry| -> Element<'a, Message> {
            match entry {
                Entry::Header(label) => container(secondary(label).size(CAPTION_TEXT_SIZE))
                    .padding([SPACING_S_NUDGE, SPACING_M_NUDGE])
                    .into(),
                Entry::Separator => container(rule::horizontal(1))
                    .padding([SPACING_XS, SPACING_M_NUDGE])
                    .into(),
                Entry::Item(item) => {
                    let label = text(item.label).wrapping(text::Wrapping::None);
                    let content: Element<'a, Message> = if checkable {
                        let mark: Element<'a, Message> = if item.checked {
                            icon::checked().into()
                        } else {
                            text("").into()
                        };
                        row![
                            container(mark).width(MARK_WIDTH),
                            label,
                            container(text("")).width(MARK_WIDTH),
                        ]
                        .spacing(SPACING_S)
                        .align_y(Center)
                        .into()
                    } else {
                        label.into()
                    };

                    button(content)
                        .width(if width.is_some() { Fill } else { Shrink })
                        .padding([SPACING_S_NUDGE, SPACING_M_NUDGE])
                        .style(item_style)
                        .on_press_maybe(item.on_press)
                        .into()
                }
            }
        });

    let list = container(column(rows))
        .width(width.map_or(Shrink, Length::Fixed))
        .padding(SPACING_XS)
        .style(dialog_style);

    sensor(list).on_resize(on_resize).into()
}

fn title_style(theme: &Theme, status: button::Status, open: bool) -> button::Style {
    let shades = theme.extended_palette().background;
    let background = match (open, status) {
        (true, _) | (false, button::Status::Hovered | button::Status::Pressed) => {
            Some(shades.weak.color.into())
        }
        _ => None,
    };

    button::Style {
        background,
        text_color: shades.weakest.text,
        border: Border::default().rounded(2),
        ..button::Style::default()
    }
}

fn item_style(theme: &Theme, status: button::Status) -> button::Style {
    let shades = theme.extended_palette().background;
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => {
            (Some(shades.weak.color.into()), shades.weak.text)
        }
        button::Status::Disabled => (None, shades.weakest.text.scale_alpha(DISABLED_TEXT_OPACITY)),
        button::Status::Active => (None, shades.weakest.text),
    };

    button::Style {
        background,
        text_color,
        border: Border::default().rounded(2),
        ..button::Style::default()
    }
}
