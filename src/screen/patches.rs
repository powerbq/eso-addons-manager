use std::collections::BTreeSet;

use eso_addons_manager_core::config::Patch;
use eso_addons_manager_core::{Catalogue, Config, patches};
use iced::widget::{
    button, checkbox, column, container, row, rule, scrollable, sensor, space, text, text_editor,
    text_input,
};
use iced::{Center, Element, Fill, FillPortion, Font, Length, Shrink, Size};

use crate::i18n::fl;
use crate::widget::{
    SPACING_M, SPACING_M_NUDGE, SPACING_S, SPACING_XS, SPACING_XXXL, button_style, inverted_input,
    plain_editor, secondary, single_line, surface,
};

const PADDING: f32 = SPACING_M_NUDGE;
const LABEL_WIDTH: f32 = SPACING_XXXL + SPACING_M;
// Fixed height of a card's top strip — the custom cards' control row and the automatic cards' name
// strip — sized from the standard scale to hold the control row so both columns line up row for row.
const CARD_HEADER_HEIGHT: f32 = SPACING_XXXL;

pub struct Patches {
    rules: Vec<Patch>,
    // Each automatic rule keeps the UID of the addon it comes from, so the card can be labelled;
    // rules of the same addon stay consecutive (the engine groups them by addon).
    automatic: Vec<(String, [text_editor::Content; 3])>,
    automatic_src: Vec<(String, Patch)>,
    error: Option<String>,
    header_height: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum Message {
    PathEdited(usize, String),
    FromEdited(usize, String),
    ToEdited(usize, String),
    EnabledToggled(usize),
    MoveUp(usize),
    MoveDown(usize),
    Remove(usize),
    Add,
    Browsed(usize, usize, text_editor::Action),
    Clear,
    Save,
    HeaderMeasured(Size),
}

pub enum Action {
    None,
    Save(Vec<Patch>),
}

impl Patches {
    pub fn new(config: &Config, installed: &BTreeSet<String>) -> Self {
        let automatic_src = patches::automatic(installed);
        Self {
            rules: config.patches.clone(),
            automatic: automatic_src.iter().map(contents_of).collect(),
            automatic_src,
            error: None,
            header_height: None,
        }
    }

    pub fn refresh(&mut self, installed: &BTreeSet<String>) {
        let automatic = patches::automatic(installed);
        if automatic != self.automatic_src {
            self.automatic = automatic.iter().map(contents_of).collect();
            self.automatic_src = automatic;
        }
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::PathEdited(index, value) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.path = value;
                }
                self.error = None;
                Action::None
            }
            Message::FromEdited(index, value) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.from = value;
                }
                self.error = None;
                Action::None
            }
            Message::ToEdited(index, value) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.to = value;
                }
                self.error = None;
                Action::None
            }
            Message::EnabledToggled(index) => {
                if let Some(rule) = self.rules.get_mut(index) {
                    rule.enabled = !rule.enabled;
                }
                self.error = None;
                Action::None
            }
            Message::MoveUp(index) => {
                if index > 0 && index < self.rules.len() {
                    self.rules.swap(index - 1, index);
                }
                self.error = None;
                Action::None
            }
            Message::MoveDown(index) => {
                if index + 1 < self.rules.len() {
                    self.rules.swap(index, index + 1);
                }
                self.error = None;
                Action::None
            }
            Message::Remove(index) => {
                if index < self.rules.len() {
                    self.rules.remove(index);
                }
                self.error = None;
                Action::None
            }
            Message::Add => {
                self.rules.push(Patch::default());
                self.error = None;
                Action::None
            }
            Message::Browsed(rule, field, action) => {
                // Read-only: ignore edits, allow cursor/selection so the text can be copied.
                if !action.is_edit()
                    && let Some((_, fields)) = self.automatic.get_mut(rule)
                    && let Some(content) = fields.get_mut(field)
                {
                    content.perform(action);
                }
                Action::None
            }
            Message::HeaderMeasured(size) => {
                self.header_height = Some(size.height);
                Action::None
            }
            Message::Clear => {
                self.rules.clear();
                self.error = None;
                Action::None
            }
            Message::Save => {
                // An empty Path or From would match every file, so refuse to save one.
                if self
                    .rules
                    .iter()
                    .any(|rule| rule.path.is_empty() || rule.from.is_empty())
                {
                    self.error = Some(fl!("patches-error-empty"));
                    return Action::None;
                }
                match patches::validate(&self.rules) {
                    Ok(()) => Action::Save(self.rules.clone()),
                    Err(error) => {
                        self.error = Some(error.to_string());
                        Action::None
                    }
                }
            }
        }
    }

    pub fn view<'a>(
        &'a self,
        config: &'a Config,
        catalogue: Option<&'a Catalogue>,
    ) -> Element<'a, Message> {
        let changed = self.rules != config.patches;

        let custom_header = row![
            secondary(fl!("patches-custom")),
            space::horizontal(),
            button(single_line(fl!("clear")))
                .style(button_style)
                .on_press_maybe((!self.rules.is_empty()).then_some(Message::Clear)),
            button(single_line(fl!("save")))
                .style(button_style)
                .on_press_maybe(changed.then_some(Message::Save)),
        ]
        .spacing(SPACING_M_NUDGE)
        .align_y(Center);

        let custom_header = sensor(surface(custom_header)).on_resize(Message::HeaderMeasured);
        let automatic_header = surface(secondary(fl!("patches-automatic")))
            .height(self.header_height.map_or(Shrink, Length::Fixed))
            .align_y(Center);

        let last = self.rules.len().saturating_sub(1);
        let mut list = column![].spacing(SPACING_M_NUDGE);
        for (index, patch) in self.rules.iter().enumerate() {
            let controls = row![
                checkbox(patch.enabled)
                    .label(fl!("patches-enabled"))
                    .on_toggle(move |_| Message::EnabledToggled(index)),
                space::horizontal(),
                button(single_line("↑"))
                    .style(button_style)
                    .on_press_maybe((index > 0).then_some(Message::MoveUp(index))),
                button(single_line("↓"))
                    .style(button_style)
                    .on_press_maybe((index < last).then_some(Message::MoveDown(index))),
                button(single_line("✕"))
                    .style(button_style)
                    .on_press(Message::Remove(index)),
            ]
            .spacing(SPACING_XS)
            .align_y(Center);

            // Fixed height so this strip matches the automatic cards' name strip without measuring.
            let controls = container(controls)
                .height(Length::Fixed(CARD_HEADER_HEIGHT))
                .align_y(Center);

            let card = surface(
                column![
                    controls,
                    edit_field(fl!("patches-path"), &patch.path, move |value| {
                        Message::PathEdited(index, value)
                    }),
                    edit_field(fl!("patches-from"), &patch.from, move |value| {
                        Message::FromEdited(index, value)
                    }),
                    edit_field(fl!("patches-to"), &patch.to, move |value| {
                        Message::ToEdited(index, value)
                    }),
                ]
                .spacing(SPACING_S),
            );
            list = list.push(card);
        }

        let mut custom =
            column![scrollable(list).spacing(SPACING_S).height(Fill)].spacing(SPACING_M_NUDGE);
        if let Some(error) = &self.error {
            custom = custom.push(text(error).style(text::danger));
        }
        custom = custom.push(
            row![
                button(single_line(fl!("patches-add")))
                    .style(button_style)
                    .on_press(Message::Add),
                space::horizontal(),
                secondary(fl!("patches-hint")),
            ]
            .align_y(Center),
        );

        let automatic: Element<'a, Message> = if self.automatic.is_empty() {
            secondary(fl!("patches-automatic-none")).into()
        } else {
            let mut list = column![].spacing(SPACING_M_NUDGE);
            for (index, (uid, fields)) in self.automatic.iter().enumerate() {
                // The catalogue holds the name of every addon, including ones pulled in only as a
                // dependency; fall back to the UID if it is not loaded yet or has no such addon.
                let name = catalogue
                    .and_then(|catalogue| catalogue.addons().iter().find(|addon| &addon.uid == uid))
                    .map_or(uid.as_str(), |addon| addon.name.as_str());
                // Same fixed strip as the custom cards, with the name where the checkbox label sits.
                let header = container(text(name))
                    .height(Length::Fixed(CARD_HEADER_HEIGHT))
                    .align_y(Center);
                list = list.push(surface(
                    column![
                        header,
                        read_field(fl!("patches-path"), &fields[0], index, 0),
                        read_field(fl!("patches-from"), &fields[1], index, 1),
                        read_field(fl!("patches-to"), &fields[2], index, 2),
                    ]
                    .spacing(SPACING_S),
                ));
            }
            scrollable(list).spacing(SPACING_S).height(Fill).into()
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
}

fn edit_field<'a>(
    label: String,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        secondary(label).width(Length::Fixed(LABEL_WIDTH)),
        text_input("", value)
            .on_input(on_input)
            .font(Font::MONOSPACE)
            .style(inverted_input),
    ]
    .spacing(SPACING_S)
    .align_y(Center)
    .into()
}

fn read_field<'a>(
    label: String,
    content: &'a text_editor::Content,
    rule: usize,
    field: usize,
) -> Element<'a, Message> {
    row![
        secondary(label).width(Length::Fixed(LABEL_WIDTH)),
        // `text_editor` has no horizontal scroll, so a long value wraps within the card rather than
        // spilling off-screen; it stays fully readable and selectable, the card just grows taller.
        text_editor(content)
            .on_action(move |action| Message::Browsed(rule, field, action))
            .font(Font::MONOSPACE)
            .style(plain_editor),
    ]
    .spacing(SPACING_S)
    .align_y(Center)
    .into()
}

fn contents_of((uid, patch): &(String, Patch)) -> (String, [text_editor::Content; 3]) {
    (
        uid.clone(),
        [
            text_editor::Content::with_text(&patch.path),
            text_editor::Content::with_text(&patch.from),
            text_editor::Content::with_text(&patch.to),
        ],
    )
}
