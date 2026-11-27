use std::collections::BTreeSet;

use eso_addons_manager_core::{Config, exclusions};
use iced::widget::{button, column, container, row, rule, space, text, text_editor};
use iced::{Center, Element, Fill, FillPortion, Font, Length};

use crate::i18n::fl;
use crate::widget::{
    HEADER_HEIGHT, SPACING_M_NUDGE, button_style, inverted_editor, plain_editor, secondary,
    single_line, surface,
};

const PADDING: f32 = SPACING_M_NUDGE;

pub struct Exclusions {
    content: text_editor::Content,
    automatic: text_editor::Content,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Edited(text_editor::Action),
    Browsed(text_editor::Action),
    Clear,
    Save,
}

pub enum Action {
    None,
    Save(Vec<String>),
}

impl Exclusions {
    pub fn new(config: &Config, installed: &BTreeSet<String>, ttc_managed: bool) -> Self {
        Self {
            content: text_editor::Content::with_text(&config.exclusions.join("\n")),
            automatic: text_editor::Content::with_text(&automatic_text(installed, ttc_managed)),
            error: None,
        }
    }

    pub fn refresh(&mut self, installed: &BTreeSet<String>, ttc_managed: bool) {
        let automatic = automatic_text(installed, ttc_managed);
        if self.automatic.text() != automatic {
            self.automatic = text_editor::Content::with_text(&automatic);
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Edited(action) => {
                self.content.perform(action);
                self.error = None;
                Action::None
            }
            Message::Browsed(action) => {
                if !action.is_edit() {
                    self.automatic.perform(action);
                }
                Action::None
            }
            Message::Clear => {
                self.content = text_editor::Content::new();
                self.error = None;
                Action::None
            }
            Message::Save => {
                let patterns = self.patterns();
                match exclusions::validate(&patterns) {
                    Ok(()) => Action::Save(patterns),
                    Err(error) => {
                        self.error = Some(error.to_string());
                        Action::None
                    }
                }
            }
        }
    }

    pub fn view<'a>(&'a self, config: &'a Config) -> Element<'a, Message> {
        let changed = self.patterns() != config.exclusions;

        let custom_header = row![
            secondary(fl!("exclusions-custom")),
            space::horizontal(),
            button(single_line(fl!("clear")))
                .style(button_style)
                .on_press_maybe((!self.content.is_empty()).then_some(Message::Clear)),
            button(single_line(fl!("save")))
                .style(button_style)
                .on_press_maybe(changed.then_some(Message::Save)),
        ]
        .spacing(SPACING_M_NUDGE)
        .align_y(Center);

        let custom_header = surface(custom_header)
            .height(Length::Fixed(HEADER_HEIGHT))
            .align_y(Center);
        let automatic_header = surface(secondary(fl!("exclusions-automatic")))
            .height(Length::Fixed(HEADER_HEIGHT))
            .align_y(Center);

        let mut custom = column![
            text_editor(&self.content)
                .on_action(Message::Edited)
                .font(Font::MONOSPACE)
                .style(inverted_editor)
                .height(Fill),
        ]
        .spacing(SPACING_M_NUDGE);
        if let Some(error) = &self.error {
            custom = custom.push(text(error).style(text::danger));
        }

        let automatic: Element<'a, Message> = if self.automatic.is_empty() {
            secondary(fl!("exclusions-automatic-none")).into()
        } else {
            text_editor(&self.automatic)
                .on_action(Message::Browsed)
                .font(Font::MONOSPACE)
                .height(Fill)
                .style(plain_editor)
                .into()
        };

        row![
            column![
                custom_header,
                rule::horizontal(1),
                container(custom).padding(PADDING).height(Fill),
            ]
            .width(FillPortion(1)),
            rule::vertical(1),
            column![
                automatic_header,
                rule::horizontal(1),
                container(automatic).padding(PADDING).height(Fill),
            ]
            .width(FillPortion(1)),
        ]
        .into()
    }

    fn patterns(&self) -> Vec<String> {
        self.content
            .text()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

fn automatic_text(installed: &BTreeSet<String>, ttc_managed: bool) -> String {
    exclusions::automatic(installed, ttc_managed).join("\n")
}
