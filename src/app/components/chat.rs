use iced::widget::text::Span;
use iced::widget::{
    Column, button, column, container, markdown, rich_text, row, scrollable, text, text_input,
};
use iced::{Background, Element, Font, Length, Renderer, Theme, alignment, border, font, padding};
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
    let bubble = container(markdown_or_emoji_text(
        message.body(),
        &message.markdown,
        MarkdownWidth::Shrink,
    ))
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
        ChatMessageBlock::Markdown { markdown, source } => {
            markdown_or_emoji_text(source, markdown, MarkdownWidth::Fill)
        }
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
            container(markdown_or_emoji_text(
                source,
                markdown,
                MarkdownWidth::Fill
            ))
            .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center)
        .into(),
    }
}

fn markdown_or_emoji_text<'a>(
    source: &'a str,
    markdown: &'a [markdown::Item],
    width: MarkdownWidth,
) -> Element<'a, Message> {
    if should_render_as_plain_emoji_text(source) {
        text_with_explicit_emoji(source, width)
    } else if let Some(segments) = split_markdown_table_segments(source) {
        render_markdown_segments(segments, width)
    } else {
        markdown::view_with(
            markdown,
            theme::markdown_settings(),
            &ChatMarkdownViewer { width },
        )
    }
}

struct SimpleMarkdownTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

enum MarkdownSegment {
    Markdown(String),
    Table(SimpleMarkdownTable),
}

fn split_markdown_table_segments(source: &str) -> Option<Vec<MarkdownSegment>> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut segments = Vec::new();
    let mut markdown_buffer = Vec::new();
    let mut index = 0;
    let mut found_table = false;

    while index < lines.len() {
        let line = lines[index].trim();
        let next_line = lines.get(index + 1).map(|line| line.trim());

        if is_table_row(line) && next_line.is_some_and(is_table_separator) {
            push_markdown_segment(&mut segments, &mut markdown_buffer);

            let table_start = index;
            index += 2;

            while index < lines.len() && is_table_row(lines[index].trim()) {
                index += 1;
            }

            if let Some(table) = parse_simple_markdown_table(&lines[table_start..index]) {
                segments.push(MarkdownSegment::Table(table));
                found_table = true;
            }

            continue;
        }

        markdown_buffer.push(lines[index]);
        index += 1;
    }

    push_markdown_segment(&mut segments, &mut markdown_buffer);

    found_table.then_some(segments)
}

fn push_markdown_segment(segments: &mut Vec<MarkdownSegment>, markdown_buffer: &mut Vec<&str>) {
    let markdown = markdown_buffer.join("\n");
    markdown_buffer.clear();

    if !markdown.trim().is_empty() {
        segments.push(MarkdownSegment::Markdown(markdown));
    }
}

fn parse_simple_markdown_table(lines: &[&str]) -> Option<SimpleMarkdownTable> {
    let lines = lines
        .iter()
        .map(|line| line.trim())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let header = lines.first()?;
    let separator = lines.get(1)?;

    if !is_table_row(header) || !is_table_separator(separator) {
        return None;
    }

    let headers = split_table_row(header);
    if headers.is_empty() {
        return None;
    }

    let rows = lines
        .iter()
        .skip(2)
        .filter(|line| is_table_row(line))
        .map(|line| split_table_row(line))
        .filter(|cells| !cells.is_empty())
        .collect::<Vec<_>>();

    Some(SimpleMarkdownTable { headers, rows })
}

fn is_table_row(line: &str) -> bool {
    line.starts_with('|') && line.ends_with('|') && line.matches('|').count() >= 2
}

fn is_table_separator(line: &str) -> bool {
    if !is_table_row(line) {
        return false;
    }

    split_table_row(line).iter().all(|cell| {
        let cell = cell.trim();
        cell.len() >= 3
            && cell
                .chars()
                .all(|character| matches!(character, '-' | ':' | ' '))
    })
}

fn split_table_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

fn render_markdown_segments<'a>(
    segments: Vec<MarkdownSegment>,
    width: MarkdownWidth,
) -> Element<'a, Message> {
    let mut content = Column::new().spacing(12).width(width.length());

    for segment in segments {
        content = content.push(match segment {
            MarkdownSegment::Markdown(source) => render_simple_markdown_text(&source, width),
            MarkdownSegment::Table(table) => render_simple_markdown_table(table),
        });
    }

    content.into()
}

fn render_simple_markdown_text<'a>(source: &str, width: MarkdownWidth) -> Element<'a, Message> {
    let mut content = Column::new().spacing(6).width(width.length());
    let mut has_lines = false;

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        has_lines = true;
        if let Some(item) = trimmed.strip_prefix("- ") {
            content = content.push(
                row![
                    text("•").size(16).color(theme::muted_text_color()),
                    rich_text(inline_markdown_spans(item))
                        .size(16)
                        .line_height(1.45)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Top),
            );
        } else {
            content = content.push(
                rich_text(inline_markdown_spans(trimmed))
                    .size(16)
                    .line_height(1.45)
                    .width(width.length()),
            );
        }
    }

    if has_lines {
        content.into()
    } else {
        text("").into()
    }
}

fn inline_markdown_spans(source: &str) -> Vec<Span<'static, (), Font>> {
    let mut spans = Vec::new();
    let mut remaining = source;

    while !remaining.is_empty() {
        if let Some(rest) = remaining.strip_prefix("**") {
            if let Some(end) = rest.find("**") {
                spans.push(markdown_span(&rest[..end], InlineStyle::Bold));
                remaining = &rest[end + 2..];
                continue;
            }
        }

        if let Some(rest) = remaining.strip_prefix('`') {
            if let Some(end) = rest.find('`') {
                spans.push(markdown_span(&rest[..end], InlineStyle::Code));
                remaining = &rest[end + 1..];
                continue;
            }
        }

        let next_bold = remaining.find("**");
        let next_code = remaining.find('`');
        let next_marker = [next_bold, next_code]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(remaining.len());

        let take = next_marker.max(1);
        spans.push(markdown_span(&remaining[..take], InlineStyle::Normal));
        remaining = &remaining[take..];
    }

    spans
}

#[derive(Debug, Clone, Copy)]
enum InlineStyle {
    Normal,
    Bold,
    Code,
}

fn markdown_span(content: &str, style: InlineStyle) -> Span<'static, (), Font> {
    let span = Span::new(content.to_owned());

    match style {
        InlineStyle::Normal => span,
        InlineStyle::Bold => span.font(Font {
            weight: font::Weight::Bold,
            ..Font::default()
        }),
        InlineStyle::Code => span
            .font(Font::MONOSPACE)
            .background(Background::Color(iced::Color::from_rgb8(0xee, 0xef, 0xea)))
            .border(
                border::rounded(4)
                    .width(1)
                    .color(iced::Color::from_rgb8(0xd6, 0xd5, 0xcd)),
            )
            .padding(padding::left(4).right(4).top(1).bottom(1)),
    }
}

fn render_simple_markdown_table<'a>(table: SimpleMarkdownTable) -> Element<'a, Message> {
    let column_count = table.headers.len();
    let mut content = Column::new().spacing(0).width(Length::Shrink);

    content = content.push(render_table_row(table.headers, column_count, true));

    for row in table.rows {
        content = content.push(render_table_row(row, column_count, false));
    }

    container(
        scrollable(container(content)).direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default().width(6).scroller_width(6),
        )),
    )
    .width(Length::Shrink)
    .style(theme::markdown_table)
    .into()
}

fn render_table_row<'a>(
    cells: Vec<String>,
    column_count: usize,
    is_header: bool,
) -> Element<'a, Message> {
    let mut row_content = row![].spacing(0).width(Length::Shrink);

    for column_index in 0..column_count {
        let cell = cells.get(column_index).map_or("", String::as_str);
        row_content = row_content.push(render_table_cell(cell, column_index, is_header));
    }

    row_content.into()
}

fn render_table_cell<'a>(
    source: &str,
    column_index: usize,
    is_header: bool,
) -> Element<'a, Message> {
    let (content, is_code) = normalize_table_cell(source);
    let mut label = text(content)
        .size(if is_header { 15 } else { 14 })
        .color(if is_header {
            theme::text_color()
        } else {
            theme::muted_text_color()
        });

    if is_code {
        label = label.font(Font::MONOSPACE);
    }

    container(label)
        .width(table_column_width(column_index))
        .padding([8, 12])
        .style(if is_header {
            theme::markdown_table_header_cell
        } else {
            theme::markdown_table_cell
        })
        .into()
}

fn normalize_table_cell(source: &str) -> (String, bool) {
    let trimmed = source.trim();

    if trimmed.len() >= 2 && trimmed.starts_with('`') && trimmed.ends_with('`') {
        (
            trimmed
                .trim_start_matches('`')
                .trim_end_matches('`')
                .to_owned(),
            true,
        )
    } else {
        (trimmed.to_owned(), false)
    }
}

fn table_column_width(column_index: usize) -> Length {
    match column_index {
        0 => 96.into(),
        _ => 280.into(),
    }
}

#[derive(Debug, Clone, Copy)]
enum MarkdownWidth {
    Fill,
    Shrink,
}

impl MarkdownWidth {
    fn length(self) -> Length {
        match self {
            Self::Fill => Length::Fill,
            Self::Shrink => Length::Shrink,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ChatMarkdownViewer {
    width: MarkdownWidth,
}

impl<'a> markdown::Viewer<'a, Message, Theme, Renderer> for ChatMarkdownViewer {
    fn on_link_click(url: markdown::Uri) -> Message {
        Message::MarkdownLinkClicked(url)
    }

    fn paragraph(
        &self,
        settings: markdown::Settings,
        text: &markdown::Text,
    ) -> Element<'a, Message> {
        rich_text(text.spans(settings.style))
            .size(settings.text_size)
            .line_height(1.45)
            .width(self.width.length())
            .on_link_click(Self::on_link_click)
            .into()
    }

    fn code_block(
        &self,
        settings: markdown::Settings,
        language: Option<&'a str>,
        _code: &'a str,
        lines: &'a [markdown::Text],
    ) -> Element<'a, Message> {
        let mut code_lines = Column::new().spacing(2).width(Length::Shrink);

        for line in lines {
            code_lines = code_lines.push(
                rich_text(line.spans(settings.style))
                    .on_link_click(Self::on_link_click)
                    .font(settings.style.code_block_font)
                    .size(settings.code_size)
                    .line_height(1.35),
            );
        }

        let body = scrollable(container(code_lines).padding([12, 14])).direction(
            scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default().width(6).scroller_width(6),
            ),
        );

        let content = if let Some(language) = language.filter(|value| !value.trim().is_empty()) {
            column![
                container(
                    text(language.trim())
                        .size(12)
                        .color(theme::muted_text_color())
                )
                .padding([8, 14])
                .width(Length::Fill),
                body,
            ]
            .spacing(0)
        } else {
            column![body]
        };

        container(content)
            .width(Length::Fill)
            .style(theme::markdown_code_block)
            .into()
    }

    fn table(
        &self,
        settings: markdown::Settings,
        columns: &'a [markdown::Column],
        rows: &'a [markdown::Row],
    ) -> Element<'a, Message> {
        container(markdown::table(self, settings, columns, rows))
            .width(Length::Shrink)
            .style(theme::markdown_table)
            .into()
    }
}

fn text_with_explicit_emoji<'a>(source: &'a str, width: MarkdownWidth) -> Element<'a, Message> {
    rich_text(emoji_spans(source))
        .on_link_click(|_| Message::DraftChanged(String::new()))
        .size(16)
        .line_height(1.45)
        .width(width.length())
        .into()
}

fn emoji_spans(source: &str) -> Vec<Span<'static, (), iced::Font>> {
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
