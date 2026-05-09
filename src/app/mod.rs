mod components;
mod theme;

use iced::{Element, Length, application};

pub fn run() -> iced::Result {
    application(CoworkApp::default, CoworkApp::update, CoworkApp::view)
        .title(theme::title)
        .theme(theme::theme)
        .window_size(theme::WINDOW_SIZE)
        .centered()
        .run()
}

#[derive(Debug, Clone)]
pub(super) enum Message {
    DraftChanged(String),
    Send,
}

#[derive(Debug, Clone)]
pub(super) struct ChatMessage {
    pub(super) author: String,
    pub(super) body: String,
}

#[derive(Debug)]
struct CoworkApp {
    draft: String,
    messages: Vec<ChatMessage>,
}

impl Default for CoworkApp {
    fn default() -> Self {
        Self {
            draft: String::new(),
            messages: vec![
                ChatMessage {
                    author: String::from("System"),
                    body: String::from("Cowork RS is ready."),
                },
                ChatMessage {
                    author: String::from("Assistant"),
                    body: String::from("Create agents, connect tools, and keep the UI responsive."),
                },
            ],
        }
    }
}

impl CoworkApp {
    fn update(&mut self, message: Message) {
        match message {
            Message::DraftChanged(value) => {
                self.draft = value;
            }
            Message::Send => {
                let content = self.draft.trim();

                if content.is_empty() {
                    return;
                }

                self.messages.push(ChatMessage {
                    author: String::from("You"),
                    body: content.to_owned(),
                });

                self.messages.push(ChatMessage {
                    author: String::from("Assistant"),
                    body: String::from("Agent runtime is not connected yet."),
                });

                self.draft.clear();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        iced::widget::container(iced::widget::row![
            components::sidebar(),
            components::chat_area(&self.messages, &self.draft),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
