use iced::widget::text::Span;
use iced::widget::{
    Column, button, column, container, markdown, rich_text, row, scrollable, text, text_input,
};
use iced::{Element, Length, alignment};
use iced_fonts::octicons;

use super::super::{
    ChatIcon, ChatMessage, ChatMessageBlock, ChatMessageKind, EMOJI_FONT, Message, theme,
};

pub(in crate::app) const TRANSCRIPT_SCROLLABLE_ID: &str = "chat-transcript";

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

    container(column![
        scrollable(content)
            .id(TRANSCRIPT_SCROLLABLE_ID)
            .height(Length::Fill),
        composer(draft, is_waiting_for_agent),
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .padding([18, 22])
    .style(theme::chat_surface)
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
        text("Start a session").size(22),
        text("Ask the agent to plan, write, debug, or inspect local project work.")
            .size(15)
            .color(theme::muted_text_color()),
    ]
    .spacing(10)
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
        ChatMessageKind::Assistant | ChatMessageKind::System | ChatMessageKind::Tool => {
            agent_message(message)
        }
    }
}

fn user_message(message: &ChatMessage) -> Element<'_, Message> {
    let bubble = container(markdown_or_emoji_text(message.body(), &message.markdown))
        .max_width(620)
        .padding([12, 14])
        .style(theme::user_message_bubble);

    let content = column![
        message_header(octicons::person().size(14), &message.author, false),
        bubble,
    ]
    .width(Length::Fill)
    .spacing(6)
    .align_x(alignment::Horizontal::Right);

    centered_transcript_lane(content).into()
}

fn agent_message(message: &ChatMessage) -> Element<'_, Message> {
    let icon = match message.kind {
        ChatMessageKind::System => octicons::alert().size(14),
        ChatMessageKind::Assistant => octicons::hubot().size(14),
        ChatMessageKind::User => octicons::person().size(14),
        ChatMessageKind::Tool => octicons::tools().size(14),
    };
    let body_style = match message.kind {
        ChatMessageKind::System => theme::system_message_body,
        ChatMessageKind::Tool => theme::tool_message_body,
        _ => theme::assistant_message_body,
    };
    let is_emphasized = matches!(
        message.kind,
        ChatMessageKind::System | ChatMessageKind::Tool
    );

    let content = column![
        message_header(icon, &message.author, is_emphasized),
        container(message_blocks(&message.blocks))
            .padding([12, 14])
            .style(body_style),
    ]
    .spacing(8);

    centered_transcript_lane(content).into()
}

fn message_blocks<'a>(blocks: &'a [ChatMessageBlock]) -> Element<'a, Message> {
    let mut content = Column::new().spacing(10).width(Length::Fill);

    for block in blocks {
        content = content.push(message_block(block));
    }

    content.into()
}

fn message_block<'a>(block: &'a ChatMessageBlock) -> Element<'a, Message> {
    match block {
        ChatMessageBlock::Markdown { markdown, source } => markdown_or_emoji_text(source, markdown),
        ChatMessageBlock::Icon {
            icon,
            markdown,
            source,
        } => row![
            container(response_icon(*icon))
                .width(24)
                .height(24)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .style(theme::inline_icon_badge),
            container(markdown_or_emoji_text(source, markdown)).width(Length::Fill),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into(),
    }
}

fn markdown_or_emoji_text<'a>(
    source: &'a str,
    markdown: &'a [markdown::Item],
) -> Element<'a, Message> {
    if should_render_as_plain_emoji_text(source) {
        text_with_explicit_emoji(source)
    } else {
        markdown::view(markdown, theme::markdown_settings()).map(Message::MarkdownLinkClicked)
    }
}

fn text_with_explicit_emoji<'a>(source: &'a str) -> Element<'a, Message> {
    rich_text(emoji_spans(source))
        .on_link_click(|_| Message::DraftChanged(String::new()))
        .size(16)
        .width(Length::Fill)
        .into()
}

fn emoji_spans(source: &str) -> Vec<Span<'_, (), iced::Font>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut current_is_emoji: Option<bool> = None;

    for character in source.chars() {
        let is_emoji = is_emoji_character(character);

        if current_is_emoji.is_some_and(|value| value != is_emoji) {
            spans.push(span_for(
                std::mem::take(&mut current),
                current_is_emoji.unwrap_or(false),
            ));
        }

        current_is_emoji = Some(is_emoji);
        current.push(character);
    }

    if !current.is_empty() {
        spans.push(span_for(current, current_is_emoji.unwrap_or(false)));
    }

    spans
}

fn span_for(content: String, is_emoji: bool) -> Span<'static, (), iced::Font> {
    let span = Span::new(content);

    if is_emoji {
        span.font(EMOJI_FONT)
    } else {
        span
    }
}

fn should_render_as_plain_emoji_text(source: &str) -> bool {
    source.chars().any(is_emoji_character) && !contains_markdown_syntax(source)
}

fn contains_markdown_syntax(source: &str) -> bool {
    source.contains("```")
        || source.contains('`')
        || source.contains("**")
        || source.contains("__")
        || source.contains('[')
        || source.lines().any(|line| {
            matches!(
                line.trim_start().chars().next(),
                Some('#' | '-' | '*' | '>' | '|')
            )
        })
}

fn is_emoji_character(character: char) -> bool {
    matches!(
        character as u32,
        0x00A9 | 0x00AE
            | 0x203C
            | 0x2049
            | 0x2122
            | 0x2139
            | 0x2194..=0x21AA
            | 0x231A..=0x231B
            | 0x2328
            | 0x23CF
            | 0x23E9..=0x23F3
            | 0x23F8..=0x23FA
            | 0x24C2
            | 0x25AA..=0x25AB
            | 0x25B6
            | 0x25C0
            | 0x25FB..=0x25FE
            | 0x2600..=0x27BF
            | 0x2934..=0x2935
            | 0x2B05..=0x2B55
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
            | 0x1F000..=0x1FAFF
            | 0xFE0F
            | 0x200D
    )
}

fn response_icon<'a>(icon: ChatIcon) -> Element<'a, Message> {
    match icon {
        ChatIcon::Alert => octicons::alert().size(14).into(),
        ChatIcon::Check => octicons::check().size(14).into(),
        ChatIcon::Code => octicons::code().size(14).into(),
        ChatIcon::Info => octicons::info().size(14).into(),
        ChatIcon::Link => octicons::link_external().size(14).into(),
        ChatIcon::Rocket => octicons::rocket().size(14).into(),
        ChatIcon::Star => octicons::star().size(14).into(),
        ChatIcon::Tools => octicons::tools().size(14).into(),
        ChatIcon::X => octicons::x().size(14).into(),
        ChatIcon::Zap => octicons::zap().size(14).into(),
    }
}

fn message_header<'a>(
    icon: impl Into<Element<'a, Message>>,
    author: &'a str,
    is_system: bool,
) -> Element<'a, Message> {
    let icon = icon.into();
    let avatar_style = if is_system {
        theme::system_avatar
    } else {
        theme::agent_avatar
    };

    row![
        container(icon)
            .width(26)
            .height(26)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(avatar_style),
        text(author).size(14).color(if is_system {
            theme::warning_text_color()
        } else {
            theme::muted_text_color()
        }),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn centered_transcript_lane<'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Container<'a, Message> {
    let lane = container(content).width(Length::Fill).max_width(820);

    container(lane).center_x(Length::Fill)
}

fn composer(draft: &str, is_waiting_for_agent: bool) -> Element<'_, Message> {
    let send_button = if is_waiting_for_agent {
        button(icon_label(octicons::clock().size(14), "Waiting"))
    } else if draft.trim().is_empty() {
        button(icon_label(octicons::paper_airplane().size(14), "Send"))
    } else {
        button(icon_label(octicons::paper_airplane().size(14), "Send")).on_press(Message::Send)
    };

    container(
        row![
            text_input("Ask the agent to work on something...", draft)
                .on_input(Message::DraftChanged)
                .on_submit(Message::Send)
                .padding(12)
                .size(15),
            send_button.padding([12, 16]),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .padding(10)
    .style(theme::composer)
    .into()
}

fn icon_label<'a>(icon: impl Into<Element<'a, Message>>, label: &'a str) -> Element<'a, Message> {
    let icon = icon.into();

    row![icon, text(label).size(14)]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
}
