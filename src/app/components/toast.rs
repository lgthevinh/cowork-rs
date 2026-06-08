use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, alignment};
use iced_fonts::octicons;

use super::super::{Message, theme};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct Toast {
    pub(crate) id: u64,
    pub(crate) level: ToastLevel,
    pub(crate) message: String,
    pub(crate) created_at: i64,
    pub(crate) expires_at: Option<i64>,
}

impl Toast {
    pub(crate) fn new(
        id: u64,
        level: ToastLevel,
        message: impl Into<String>,
        created_at: i64,
        expires_at: Option<i64>,
    ) -> Self {
        Self {
            id,
            level,
            message: message.into(),
            created_at,
            expires_at,
        }
    }

    fn label(&self) -> &'static str {
        match self.level {
            ToastLevel::Info => "Info",
            ToastLevel::Success => "Saved",
            ToastLevel::Warning => "Warning",
            ToastLevel::Error => "Error",
        }
    }

    fn icon(&self) -> Element<'static, Message> {
        match self.level {
            ToastLevel::Info => octicons::info().size(13).into(),
            ToastLevel::Success => octicons::check().size(13).into(),
            ToastLevel::Warning => octicons::alert().size(13).into(),
            ToastLevel::Error => octicons::x().size(13).into(),
        }
    }

    fn style(&self) -> fn(&iced::Theme) -> iced::widget::container::Style {
        match self.level {
            ToastLevel::Info => theme::toast_info,
            ToastLevel::Success => theme::toast_success,
            ToastLevel::Warning => theme::toast_warning,
            ToastLevel::Error => theme::toast_error,
        }
    }
}

pub(crate) fn toast_overlay<'a>(toasts: &'a [Toast]) -> Element<'a, Message> {
    let mut stack = column![]
        .spacing(10)
        .width(Length::Shrink)
        .align_x(alignment::Horizontal::Right);

    for toast in toasts.iter().rev() {
        stack = stack.push(toast_card(toast));
    }

    container(stack)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(16)
        .align_x(alignment::Horizontal::Right)
        .align_y(alignment::Vertical::Top)
        .into()
}

fn toast_card<'a>(toast: &'a Toast) -> Element<'a, Message> {
    let dismiss = button(octicons::x().size(12))
        .on_press(Message::ToastDismissed(toast.id))
        .padding([4, 6])
        .style(theme::quiet_button);

    let body = column![
        row![
            row![toast.icon(), text(toast.label()).size(12)]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            container(text("")).width(Length::Fill),
            dismiss,
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
        text(&toast.message)
            .size(14)
            .width(Length::Fill)
            .line_height(1.35),
    ]
    .spacing(8)
    .width(Length::Fill);

    container(body)
        .width(Length::Fill)
        .max_width(360)
        .padding([12, 14])
        .style(toast.style())
        .into()
}
