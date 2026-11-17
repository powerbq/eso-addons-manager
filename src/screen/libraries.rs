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

pub fn view<'a>(catalogue: &'a Catalogue, config: &'a Config) -> Element<'a, Message> {
    let providers = catalogue.providers();

    let mut groups: Vec<(bool, Element<'a, Message>)> = config
        .selected_libraries
        .iter()
        .map(|(directory, selected)| {
            let shared = providers
                .get(directory.as_str())
                .filter(|candidates| candidates.len() > 1);
            let candidates: Vec<&'a Addon> = match shared {
                Some(candidates) => candidates.clone(),
                None => catalogue
                    .position(selected)
                    .and_then(|index| catalogue.addon(index))
                    .into_iter()
                    .collect(),
            };

            let chosen = candidates.iter().position(|addon| &addon.uid == selected);
            let options: Element<'a, Message> = if candidates.is_empty() {
                secondary(format!("UID {selected}")).into()
            } else {
                column(candidates.iter().enumerate().map(|(index, addon)| {
                    let option =
                        radio(choice::label(addon), index, chosen, |_| Message::Selected {
                            directory: directory.clone(),
                            uid: addon.uid.clone(),
                        });
                    choice::candidate(
                        option,
                        choice::folders(addon),
                        Message::OpenPage(addon.info_url.clone()),
                    )
                }))
                .spacing(SPACING_S)
                .into()
            };

            let header = row![
                text(directory).font(Font {
                    weight: font::Weight::Semibold,
                    ..Font::DEFAULT
                }),
                space::horizontal(),
                button(single_line(fl!("reset")))
                    .style(button_style)
                    .on_press(Message::Reset(directory.clone())),
            ]
            .spacing(SPACING_M_NUDGE)
            .align_y(Center);

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
