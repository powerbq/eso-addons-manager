use iced::widget::container;
use iced::{Border, Color, Theme};

const BACKDROP_OPACITY: f32 = 0.8;
const DIALOG_RADIUS: f32 = 4.0;

pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(
            Color {
                a: BACKDROP_OPACITY,
                ..Color::BLACK
            }
            .into(),
        ),
        ..container::Style::default()
    }
}

pub fn dialog_style(theme: &Theme) -> container::Style {
    let shades = theme.extended_palette().background;

    container::Style {
        background: Some(shades.weakest.color.into()),
        text_color: Some(shades.weakest.text),
        border: Border::default()
            .width(1)
            .color(shades.weak.color)
            .rounded(DIALOG_RADIUS),
        ..container::Style::default()
    }
}
