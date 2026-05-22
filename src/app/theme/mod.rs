use iced::widget::{button, container, markdown};
use iced::{Background, Border, Color, Theme};

use super::CoworkApp;

pub(super) const WINDOW_SIZE: (f32, f32) = (1100.0, 720.0);

const ACCENT: Color = Color::from_rgb8(0x2e, 0xc2, 0x7e);
const BACKGROUND: Color = Color::from_rgb8(0xf5, 0xf4, 0xef);
const SURFACE: Color = Color::from_rgb8(0xfb, 0xfa, 0xf7);
const PANEL: Color = Color::from_rgb8(0xff, 0xff, 0xfc);
const SIDEBAR: Color = Color::from_rgb8(0xec, 0xef, 0xeb);
const USER_BUBBLE: Color = Color::from_rgb8(0xee, 0xf8, 0xf2);
const BORDER: Color = Color::from_rgb8(0xdd, 0xdb, 0xd2);
const TEXT: Color = Color::from_rgb8(0x24, 0x26, 0x22);
const MUTED_TEXT: Color = Color::from_rgb8(0x62, 0x68, 0x61);
const SUBTLE_TEXT: Color = Color::from_rgb8(0x8a, 0x8d, 0x86);

pub(super) fn title(_: &CoworkApp) -> String {
    String::from("Cowork")
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

pub(in crate::app) fn top_bar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(PANEL)),
        border: Border {
            width: 1.0,
            color: BORDER,
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn chat_surface(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(SURFACE)),
        border: Border {
            width: 1.0,
            color: BORDER,
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn panel(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(PANEL)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn modal_backdrop(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color { a: 0.36, ..TEXT })),
        ..container::Style::default()
    }
}

pub(in crate::app) fn settings_dialog(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(PANEL)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn settings_sidebar(_: &Theme) -> container::Style {
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

pub(in crate::app) fn selected_session(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(PANEL)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: ACCENT,
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn session_card(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(Color { a: 0.0, ..PANEL })),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: Color { a: 0.0, ..BORDER },
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn new_chat_button(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => ACCENT,
        button::Status::Active | button::Status::Disabled => USER_BUBBLE,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: TEXT,
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        ..button::Style::default()
    }
}

pub(in crate::app) fn session_title_button(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: if status == button::Status::Disabled {
            SUBTLE_TEXT
        } else {
            TEXT
        },
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub(in crate::app) fn delete_session_button(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(Background::Color(Color { a: 0.12, ..TEXT }))
        }
        button::Status::Active | button::Status::Disabled => None,
    };

    button::Style {
        background,
        text_color: if status == button::Status::Disabled {
            SUBTLE_TEXT
        } else {
            MUTED_TEXT
        },
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub(in crate::app) fn active_settings_button(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => ACCENT,
        button::Status::Active | button::Status::Disabled => USER_BUBBLE,
    };

    button::Style {
        background: Some(Background::Color(background)),
        text_color: TEXT,
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub(in crate::app) fn user_message_bubble(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(USER_BUBBLE)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn assistant_message_body(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(Color { a: 0.0, ..PANEL })),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: Color { a: 0.0, ..BORDER },
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn system_message_body(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(Color::from_rgb8(0xff, 0xf7, 0xe8))),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: Color::from_rgb8(0xe8, 0xc9, 0x8a),
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn inline_icon_badge(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(USER_BUBBLE)),
        border: Border {
            width: 1.0,
            radius: 7.0.into(),
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

pub(in crate::app) fn system_avatar(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color::from_rgb8(0xb7, 0x7e, 0x33))),
        border: Border {
            radius: 7.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn composer(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(PANEL)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn status_pill(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT),
        background: Some(Background::Color(USER_BUBBLE)),
        border: Border {
            width: 1.0,
            radius: 8.0.into(),
            color: BORDER,
        },
        ..container::Style::default()
    }
}

pub(in crate::app) fn quiet_button(_: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Some(Background::Color(PANEL)),
        button::Status::Active | button::Status::Disabled => None,
    };

    button::Style {
        background,
        text_color: if status == button::Status::Disabled {
            SUBTLE_TEXT
        } else {
            TEXT
        },
        border: Border {
            radius: 6.0.into(),
            ..Border::default()
        },
        ..button::Style::default()
    }
}

pub(in crate::app) fn muted_text_color() -> Color {
    MUTED_TEXT
}

pub(in crate::app) fn warning_text_color() -> Color {
    Color::from_rgb8(0x8a, 0x5a, 0x20)
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
