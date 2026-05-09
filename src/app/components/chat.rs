use iced::widget::{Column, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use super::super::{ChatMessage, Message};

pub(in crate::app) fn chat_area<'a>(
    messages: &'a [ChatMessage],
    draft: &'a str,
) -> Element<'a, Message> {
    let mut transcript = Column::new().spacing(10);

    for message in messages {
        transcript = transcript.push(message_bubble(message));
    }

    container(
        column![
            text("Agent Chat").size(24),
            scrollable(transcript).height(Length::Fill),
            composer(draft),
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(20)
    .into()
}

fn message_bubble(message: &ChatMessage) -> Element<'_, Message> {
    container(
        column![
            text(&message.author).size(14),
            text(&message.body).size(16).width(Length::Fill),
        ]
        .spacing(4),
    )
    .width(Length::Fill)
    .padding(12)
    .into()
}

fn composer(draft: &str) -> Element<'_, Message> {
    row![
        text_input("Ask an agent to do something...", draft)
            .on_input(Message::DraftChanged)
            .on_submit(Message::Send)
            .padding(12)
            .size(16),
        button("Send").on_press(Message::Send).padding(12),
    ]
    .spacing(10)
    .into()
}
