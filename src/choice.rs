use eso_addons_manager_core::Addon;
use iced::widget::{Column, Radio, column, container, radio, row, text};
use iced::{Center, Element, Font, Padding, font};

use crate::i18n::fl;
use crate::widget::{
    SPACING_L, SPACING_M_NUDGE, SPACING_S, SPACING_XL, SPACING_XXS, open_page, secondary, surface,
};

const SPACING: f32 = SPACING_S;

#[derive(Debug)]
pub struct Choice {
    pub directory: String,
    candidates: Vec<Candidate>,
    selected: Option<usize>,
}

#[derive(Debug)]
struct Candidate {
    uid: String,
    label: String,
    folders: String,
    url: String,
}

impl Choice {
    pub fn new(directory: String, candidates: &[&Addon]) -> Self {
        Self {
            directory,
            candidates: candidates
                .iter()
                .map(|addon| Candidate {
                    uid: addon.uid.clone(),
                    label: label(addon),
                    folders: folders(addon),
                    url: addon.info_url.clone(),
                })
                .collect(),
            selected: None,
        }
    }

    pub fn select(&mut self, index: usize) {
        if index < self.candidates.len() {
            self.selected = Some(index);
        }
    }

    pub fn uid(&self) -> Option<&str> {
        self.selected
            .and_then(|index| self.candidates.get(index))
            .map(|candidate| candidate.uid.as_str())
    }
}

pub fn label(addon: &Addon) -> String {
    format!("{} · UID {}", addon.name, addon.uid)
}

pub fn folders(addon: &Addon) -> String {
    fl!("candidate-folders", folders = addon.directories.join(", "))
}

pub fn candidate<'a, Message: Clone + 'a>(
    option: Radio<'a, Message>,
    folders: impl text::IntoFragment<'a>,
    open: Message,
) -> Element<'a, Message> {
    let indent = Radio::<Message>::DEFAULT_SIZE + Radio::<Message>::DEFAULT_SPACING;

    column![
        row![option, open_page(open)]
            .spacing(SPACING_M_NUDGE)
            .align_y(Center),
        container(secondary(folders).wrapping(text::Wrapping::Word))
            .padding(Padding::ZERO.left(indent)),
    ]
    .spacing(SPACING_XXS)
    .into()
}

pub fn is_complete(choices: &[Choice]) -> bool {
    choices.iter().all(|choice| choice.uid().is_some())
}

pub fn view<'a, Message: Clone + 'a>(
    choices: &'a [Choice],
    on_select: fn(usize, usize) -> Message,
    on_open: fn(String) -> Message,
) -> Column<'a, Message> {
    let bold = Font {
        weight: font::Weight::Semibold,
        ..Font::DEFAULT
    };

    column(choices.iter().enumerate().map(|(position, choice)| {
        let options = column(choice.candidates.iter().enumerate().map(|(index, entry)| {
            let option = radio(entry.label.as_str(), index, choice.selected, move |index| {
                on_select(position, index)
            });
            candidate(option, entry.folders.as_str(), on_open(entry.url.clone()))
        }))
        .spacing(SPACING)
        .padding([0.0, SPACING_XL]);

        column![surface(text(&choice.directory).font(bold)), options]
            .spacing(SPACING)
            .into()
    }))
    .spacing(SPACING_L)
}
