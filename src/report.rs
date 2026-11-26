use iced::widget::{Column, button, column, row, text};
use iced::{Center, Fill, Font, font};

use crate::i18n::fl;
use crate::icon;
use crate::widget::{
    BODY_TEXT_SIZE, SPACING_M, SPACING_M_NUDGE, SPACING_S_NUDGE, SPACING_XS, secondary,
};

const EXPANDED_LINES: usize = 10;
const SPACING: f32 = SPACING_M;
const HEADER_SPACING: f32 = SPACING_S_NUDGE;
const LINE_INDENT: f32 = BODY_TEXT_SIZE + HEADER_SPACING;

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub note: Option<String>,
    pub sections: Vec<Section>,
    pub footer: Option<String>,
    pub problems: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub title: String,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub name: String,
    pub detail: String,
}

impl Report {
    pub fn push_section(&mut self, title: String, mut lines: Vec<Line>) {
        if lines.is_empty() {
            return;
        }

        lines.sort_by_cached_key(|line| line.name.to_lowercase());
        self.sections.push(Section { title, lines });
    }

    pub fn expanded(&self) -> Vec<bool> {
        let mut flags: Vec<bool> = self
            .sections
            .iter()
            .map(|section| section.lines.len() <= EXPANDED_LINES)
            .collect();
        if !self.problems.is_empty() {
            flags.push(true);
        }
        flags
    }
}

pub fn view<'a, Message: Clone + 'a>(
    report: &'a Report,
    expanded: &'a [bool],
    on_toggle: impl Fn(usize) -> Message,
    inline: bool,
) -> Column<'a, Message> {
    let bold = Font {
        weight: font::Weight::Semibold,
        ..Font::DEFAULT
    };
    let mut body = column![].spacing(SPACING).width(Fill);

    if let Some(note) = &report.note {
        body = body.push(text(note));
    }

    for (index, section) in report.sections.iter().enumerate() {
        let open = expanded.get(index).copied().unwrap_or(true);
        let header = button(
            row![
                if open {
                    icon::expanded()
                } else {
                    icon::collapsed()
                },
                text(format!("{} ({})", section.title, section.lines.len())).font(bold),
            ]
            .spacing(HEADER_SPACING)
            .align_y(Center),
        )
        .style(button::text)
        .padding(0)
        .on_press(on_toggle(index));

        let mut section_column = column![header].spacing(SPACING_XS);
        if open {
            for line in &section.lines {
                // inline keeps the detail next to the name (grey); otherwise the name fills and the
                // detail is pushed to the right as its own column (the sync report's version column).
                let name = text(&line.name);
                let name = if inline { name } else { name.width(Fill) };
                // inline: grey detail at the name's own size, so the two line up cleanly next to each
                // other; otherwise the smaller `secondary` caption in its own right-hand column.
                let detail = if inline {
                    text(line.detail.as_str()).style(text::secondary)
                } else {
                    secondary(line.detail.as_str())
                };
                section_column = section_column.push(
                    row![name, detail]
                        .spacing(SPACING_M_NUDGE)
                        .padding([0.0, LINE_INDENT])
                        .align_y(Center),
                );
            }
        }
        body = body.push(section_column);
    }

    if let Some(footer) = &report.footer {
        body = body.push(text(footer));
    }

    if !report.problems.is_empty() {
        // The problems block is a collapsible section like the others (its toggle slot sits after the
        // real sections in `expanded`), but it opens expanded by default (see `expanded`).
        let index = report.sections.len();
        let open = expanded.get(index).copied().unwrap_or(true);
        let header = button(
            row![
                if open {
                    icon::expanded()
                } else {
                    icon::collapsed()
                },
                text(format!(
                    "{} ({})",
                    fl!("report-problems"),
                    report.problems.len()
                ))
                .font(bold),
            ]
            .spacing(HEADER_SPACING)
            .align_y(Center),
        )
        .style(button::text)
        .padding(0)
        .on_press(on_toggle(index));

        let mut problems = column![header].spacing(SPACING_XS);
        if open {
            for problem in &report.problems {
                problems = problems.push(
                    row![text(problem).style(text::danger).width(Fill)].padding([0.0, LINE_INDENT]),
                );
            }
        }
        body = body.push(problems);
    }

    body
}
