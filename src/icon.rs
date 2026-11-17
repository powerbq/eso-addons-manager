mod glyphs;

use iced::Font;
use iced::widget::{Text, text};
use lucide_icons::Icon;

pub const FONT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lucide.ttf"));

const FONT_NAME: Font = Font::with_name("lucide");

pub fn name<'a>() -> Text<'a> {
    icon(glyphs::NAME)
}

pub fn author<'a>() -> Text<'a> {
    icon(glyphs::AUTHOR)
}

pub fn updated<'a>() -> Text<'a> {
    icon(glyphs::UPDATED)
}

pub fn downloads<'a>() -> Text<'a> {
    icon(glyphs::DOWNLOADS)
}

pub fn monthly_downloads<'a>() -> Text<'a> {
    icon(glyphs::MONTHLY_DOWNLOADS)
}

pub fn favourites<'a>() -> Text<'a> {
    icon(glyphs::FAVOURITES)
}

pub fn ascending<'a>() -> Text<'a> {
    icon(glyphs::ASCENDING)
}

pub fn descending<'a>() -> Text<'a> {
    icon(glyphs::DESCENDING)
}

pub fn install<'a>() -> Text<'a> {
    icon(glyphs::INSTALL)
}

pub fn remove<'a>() -> Text<'a> {
    icon(glyphs::REMOVE)
}

pub fn collapsed<'a>() -> Text<'a> {
    icon(glyphs::COLLAPSED)
}

pub fn expanded<'a>() -> Text<'a> {
    icon(glyphs::EXPANDED)
}

pub fn open_page<'a>() -> Text<'a> {
    icon(glyphs::OPEN_PAGE)
}

pub fn checked<'a>() -> Text<'a> {
    icon(glyphs::CHECKED)
}

pub fn menu<'a>() -> Text<'a> {
    icon(glyphs::MENU)
}

fn icon<'a>(glyph: Icon) -> Text<'a> {
    text(char::from(glyph).to_string())
        .font(FONT_NAME)
        .line_height(1.0)
}
