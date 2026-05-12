use iced::widget::{
    Column, button, column, container, markdown, row, scrollable, text, text_input,
};
use iced::{Element, Length, alignment};

use super::super::{ChatMessage, ChatMessageKind, Message, theme};

pub(in crate::app) fn chat_area<'a>(
    messages: &'a [ChatMessage],
    draft: &'a str,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    let content = if messages.is_empty() {
        chat_placeholder()
    } else {
        transcript(messages)
    };

    container(
        column![
            scrollable(content).height(Length::Fill),
            composer(draft, is_waiting_for_agent),
        ]
        .spacing(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(20)
    .into()
}

fn transcript(messages: &[ChatMessage]) -> Element<'_, Message> {
    let mut transcript = Column::new().spacing(28).width(Length::Fill);

    for message in messages {
        transcript = transcript.push(message_row(message));
    }

    transcript.into()
}

fn chat_placeholder<'a>() -> Element<'a, Message> {
    let placeholder = column![
        text("NEO Cowork").size(20),
        text("Ask an agent to do something, and the conversation will appear here.")
            .size(15)
            .color(theme::muted_text_color()),
    ]
    .spacing(18)
    .align_x(alignment::Horizontal::Center);

    container(placeholder)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}

fn message_row(message: &ChatMessage) -> Element<'_, Message> {
    match message.kind {
        ChatMessageKind::User => user_message(message),
        ChatMessageKind::Assistant | ChatMessageKind::System => agent_message(message),
    }
}

fn user_message(message: &ChatMessage) -> Element<'_, Message> {
    let bubble = container(
        markdown::view(&message.markdown, theme::markdown_settings())
            .map(Message::MarkdownLinkClicked),
    )
    .max_width(520)
    .padding([10, 14])
    .style(theme::user_message_bubble);

    let content = column![
        text(&message.author)
            .size(12)
            .color(theme::muted_text_color()),
        bubble,
    ]
    .width(Length::Fill)
    .spacing(6)
    .align_x(alignment::Horizontal::Right);

    centered_transcript_lane(content).into()
}

fn agent_message(message: &ChatMessage) -> Element<'_, Message> {
    let avatar_label = match message.kind {
        ChatMessageKind::System => "S",
        _ => "A",
    };

    let header = row![
        container(text(avatar_label).size(13))
            .width(26)
            .height(26)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(theme::agent_avatar),
        text(&message.author).size(14),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    let content = column![
        header,
        markdown::view(&message.markdown, theme::markdown_settings())
            .map(Message::MarkdownLinkClicked),
    ]
    .spacing(12);

    centered_transcript_lane(content).into()
}

fn centered_transcript_lane<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let lane = container(content).width(Length::Fill).max_width(820);

    container(lane).center_x(Length::Fill)
}

fn composer(draft: &str, is_waiting_for_agent: bool) -> Element<'_, Message> {
    let send_button = if is_waiting_for_agent {
        button("Sending...")
    } else {
        button("Send").on_press(Message::Send)
    };

    row![
        text_input("Ask an agent to do something...", draft)
            .on_input(Message::DraftChanged)
            .on_submit(Message::Send)
            .padding(12)
            .size(16),
        send_button.padding(12),
    ]
    .spacing(10)
    .into()
}
