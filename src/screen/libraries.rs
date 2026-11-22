use std::collections::{BTreeMap, BTreeSet};

use eso_addons_manager_core::{Addon, Catalogue, Config};
use iced::widget::{button, center, column, radio, row, rule, scrollable, space, text};
use iced::{Center, Element, Fill, Font, font};

use crate::choice;
use crate::i18n::fl;
use crate::widget::{
    SPACING_M_NUDGE, SPACING_S, SPACING_XL, button_style, secondary, single_line, surface,
};

#[derive(Debug, Clone)]
pub enum Message {
    Selected { directory: String, uid: String },
    OpenPage(String),
    Reset(String),
}

pub fn view<'a>(
    catalogue: &'a Catalogue,
    config: &'a Config,
    recommended: &'a BTreeMap<String, Vec<String>>,
) -> Element<'a, Message> {
    let providers = catalogue.providers();

    // Every shared-folder library in play: those the user chose manually, plus those resolved
    // automatically from a description link (so an automatic pick can still be changed).
    let directories: BTreeSet<&'a String> = config
        .selected_libraries
        .keys()
        .chain(recommended.keys())
        .collect();

    let mut groups: Vec<(bool, Element<'a, Message>)> = directories
        .into_iter()
        .map(|directory| {
            let selected = config.selected_libraries.get(directory);
            let recommended = recommended.get(directory);

            let shared = providers
                .get(directory.as_str())
                .filter(|candidates| candidates.len() > 1);
            let candidates: Vec<&'a Addon> = match shared {
                Some(candidates) => candidates.clone(),
                None => selected
                    .and_then(|uid| catalogue.position(uid))
                    .and_then(|index| catalogue.addon(index))
                    .into_iter()
                    .collect(),
            };

            // the current pick: the manual choice, else the automatic one (a lone recommendation).
            let chosen_uid: Option<&str> = selected.map(String::as_str).or_else(|| {
                recommended
                    .filter(|uids| uids.len() == 1)
                    .map(|uids| uids[0].as_str())
            });
            let chosen =
                chosen_uid.and_then(|uid| candidates.iter().position(|addon| addon.uid == uid));

            let options: Element<'a, Message> = if candidates.is_empty() {
                let uid = selected.map(String::as_str).unwrap_or_default();
                secondary(format!("UID {uid}")).into()
            } else {
                column(candidates.iter().enumerate().map(|(index, addon)| {
                    let option =
                        radio(choice::label(addon), index, chosen, |_| Message::Selected {
                            directory: directory.clone(),
                            uid: addon.uid.clone(),
                        });
                    let is_recommended = recommended.is_some_and(|uids| uids.contains(&addon.uid));
                    choice::candidate(
                        option,
                        choice::folders(addon),
                        Message::OpenPage(addon.info_url.clone()),
                        is_recommended,
                    )
                }))
                .spacing(SPACING_S)
                .into()
            };

            let mut header = row![
                text(directory).font(Font {
                    weight: font::Weight::Semibold,
                    ..Font::DEFAULT
                }),
                space::horizontal(),
            ]
            .spacing(SPACING_M_NUDGE)
            .align_y(Center);
            // Reset only makes sense for a manual choice — it returns the folder to automatic.
            if selected.is_some() {
                header = header.push(
                    button(single_line(fl!("reset")))
                        .style(button_style)
                        .on_press(Message::Reset(directory.clone())),
                );
            }

            let group = column![
                surface(header),
                column![options].padding([SPACING_M_NUDGE, SPACING_XL]),
                rule::horizontal(1),
            ];

            (shared.is_none(), group.into())
        })
        .collect();
    groups.sort_by_key(|(unexplained, _)| *unexplained);

    if groups.is_empty() {
        return center(text(fl!("libraries-none"))).into();
    }

    scrollable(column(groups.into_iter().map(|(_, group)| group)))
        .height(Fill)
        .into()
}
