use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use super::super::{Message, theme};

pub(in crate::app) fn sidebar() -> Element<'static, Message> {
    container(
        column![
            text("NEO Cowork").size(28),
            text("Workspace").size(16).color(theme::muted_text_color()),
            button("New chat"),
            button("Agents"),
            button("Tools"),
            button("Settings"),
        ]
        .spacing(14),
    )
    .width(240)
    .height(Length::Fill)
    .padding(20)
    .style(theme::sidebar)
    .into()
}
