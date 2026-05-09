use iced::widget::{button, column, container, text};
use iced::{Element, Length};

use super::super::Message;

pub(in crate::app) fn sidebar() -> Element<'static, Message> {
    container(
        column![
            text("Cowork RS").size(28),
            text("Workspace").size(16),
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
    .into()
}
