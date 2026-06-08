use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, alignment};
use iced_fonts::octicons;

use super::super::{Message, theme};
use crate::storage::chat_record::SessionRecord;

pub(in crate::app) fn sidebar<'a>(
    active_session_id: &'a str,
    recent_sessions: &'a [SessionRecord],
    visible_session_count: usize,
    width: f32,
) -> Element<'a, Message> {
    container(
        column![
            sidebar_header(),
            button(icon_label(octicons::plus().size(14), "New chat"))
                .on_press(Message::NewSession)
                .width(Length::Fill)
                .padding([11, 12])
                .style(theme::new_chat_button),
            recent_section(active_session_id, recent_sessions, visible_session_count),
        ]
        .spacing(14),
    )
    .width(width)
    .height(Length::Fill)
    .padding(18)
    .style(theme::sidebar)
    .into()
}

fn sidebar_header<'a>() -> Element<'a, Message> {
    column![
        text("Cowork").size(24),
        text("Local agent workspace")
            .size(13)
            .color(theme::muted_text_color()),
    ]
    .spacing(2)
    .into()
}

fn icon_label<'a>(icon: impl Into<Element<'a, Message>>, label: &'a str) -> Element<'a, Message> {
    let icon = icon.into();

    row![icon, text(label).size(14)]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
}

fn recent_section<'a>(
    active_session_id: &'a str,
    recent_sessions: &'a [SessionRecord],
    visible_session_count: usize,
) -> Element<'a, Message> {
    let visible_count = visible_session_count.min(recent_sessions.len());
    let mut list = column![text("Recent").size(12).color(theme::muted_text_color())].spacing(8);

    for session in recent_sessions.iter().take(visible_count) {
        list = list.push(session_card(
            session,
            session.session_id == active_session_id,
        ));
    }

    if recent_sessions.len() > visible_count {
        list = list.push(
            button(icon_label(octicons::chevron_down().size(14), "Load more"))
                .on_press(Message::LoadMoreSessions)
                .width(Length::Fill)
                .padding([8, 10])
                .style(theme::quiet_button),
        );
    }

    list.into()
}

fn session_card(session: &SessionRecord, is_active: bool) -> Element<'_, Message> {
    let style = if is_active {
        theme::selected_session
    } else {
        theme::session_card
    };
    let title = session.title.trim();
    let title = if title.is_empty() {
        "New session"
    } else {
        title
    };
    let session_id = session.session_id.clone();
    let delete_session_id = session.session_id.clone();
    let mut content = row![
        button(text(title).size(14))
            .on_press(Message::SessionSelected(session_id))
            .width(Length::Fill)
            .padding(0)
            .style(theme::session_title_button),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    if is_active {
        content = content.push(
            button(octicons::x().size(14))
                .on_press(Message::DeleteSession(delete_session_id))
                .padding(4)
                .style(theme::delete_session_button),
        );
    }

    container(content)
        .width(Length::Fill)
        .padding([8, 10])
        .style(style)
        .into()
}
