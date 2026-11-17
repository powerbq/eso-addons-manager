use std::sync::Arc;

use iced::widget::text_editor::{Action, Edit, Motion};
use iced::widget::{container, text_editor};
use iced::{Element, Fill, Font};

use crate::i18n::fl;
use crate::widget::{SPACING_M, plain_editor};

const PADDING: f32 = SPACING_M;

#[derive(Debug, Clone)]
pub enum Message {
    Browsed(Action),
}

#[derive(Default)]
pub struct Log {
    content: text_editor::Content,
}

impl Log {
    pub fn push(&mut self, records: Vec<String>) {
        let mut text = records.join("\n");
        if !self.content.is_empty() {
            text.insert(0, '\n');
        }

        let cursor = self.content.cursor();
        self.content.perform(Action::Move(Motion::DocumentEnd));
        let following = self.content.cursor() == cursor;

        self.content
            .perform(Action::Edit(Edit::Paste(Arc::new(text))));
        if !following {
            self.content.move_to(cursor);
        }
    }

    pub fn text(&self) -> String {
        self.content.text()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Browsed(action) => {
                if !action.is_edit() {
                    self.content.perform(action);
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            text_editor(&self.content)
                .placeholder(fl!("log-empty"))
                .on_action(Message::Browsed)
                .font(Font::MONOSPACE)
                .padding(PADDING)
                .height(Fill)
                .style(plain_editor),
        )
        .height(Fill)
        .into()
    }
}
