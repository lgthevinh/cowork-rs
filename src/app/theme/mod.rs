use iced::widget::{container, markdown};
use iced::{Background, Border, Color, Theme};

use super::CoworkApp;

pub(super) const WINDOW_SIZE: (f32, f32) = (1100.0, 720.0);

const ACCENT: Color = Color::from_rgb8(0x2e, 0xc2, 0x7e);
const BACKGROUND: Color = Color::from_rgb8(0xf7, 0xfb, 0xf8);
const SIDEBAR: Color = Color::from_rgb8(0xee, 0xf8, 0xf2);
const USER_BUBBLE: Color = Color::from_rgb8(0xee, 0xf8, 0xf2);
const BORDER: Color = Color::from_rgb8(0xd8, 0xea, 0xdf);
const TEXT: Color = Color::from_rgb8(0x1f, 0x2a, 0x24);
const MUTED_TEXT: Color = Color::from_rgb8(0x5f, 0x70, 0x66);

pub(super) fn title(_: &CoworkApp) -> String {
    String::from("NEO Cowork")
}

pub(super) fn theme(_: &CoworkApp) -> Theme {
    soft_white_green()
}

pub(in crate::app) fn markdown_settings() -> markdown::Settings {
    markdown::Settings::with_text_size(16, soft_white_green())
}

pub(in crate::app) fn app_background(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(BACKGROUND)),
        ..container::Style::default()
    }
}

pub(in crate::app) fn sidebar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(SIDEBAR)),
        border: Border {
            width: 1.0,
            color: BORDER,
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn user_message_bubble(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(USER_BUBBLE)),
        border: Border {
            width: 1.0,
            radius: 16.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn agent_avatar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(ACCENT)),
        border: Border {
            radius: 7.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn muted_text_color() -> Color {
    MUTED_TEXT
}

fn soft_white_green() -> Theme {
    Theme::custom(
        "Soft White Green",
        iced::theme::Palette {
            background: BACKGROUND,
            text: TEXT,
            primary: ACCENT,
            success: ACCENT,
            warning: Color::from_rgb8(0xb7, 0x7e, 0x33),
            danger: Color::from_rgb8(0xc3, 0x42, 0x3f),
        },
    )
}
