use iced::widget::{button, column, container, opaque, row, stack, text};
use iced::{Element, Length, alignment};
use iced_fonts::octicons;

use super::super::{Message, SettingsTab, theme};

pub(in crate::app) fn settings_dialog<'a>(
    active_tab: SettingsTab,
    session_model: &'a str,
    message_count: usize,
    db_path: String,
    is_icon_font_loaded: bool,
    is_emoji_font_loaded: bool,
    emoji_font_path: Option<String>,
    emoji_font_error: Option<String>,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    let dialog = container(row![
        settings_sidebar(active_tab),
        settings_content(
            active_tab,
            session_model,
            message_count,
            db_path,
            is_icon_font_loaded,
            is_emoji_font_loaded,
            emoji_font_path,
            emoji_font_error,
            is_waiting_for_agent
        ),
    ])
    .width(760)
    .height(500)
    .style(theme::settings_dialog);

    stack![
        container("")
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::modal_backdrop),
        opaque(
            container(dialog)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
        ),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn settings_sidebar(active_tab: SettingsTab) -> Element<'static, Message> {
    container(
        column![
            text("Settings").size(20),
            settings_tab_button(
                octicons::gear().size(14),
                "General",
                SettingsTab::General,
                active_tab
            ),
            settings_tab_button(
                octicons::hubot().size(14),
                "Agent",
                SettingsTab::Agent,
                active_tab
            ),
            settings_tab_button(
                octicons::tools().size(14),
                "Tools",
                SettingsTab::Tools,
                active_tab
            ),
            settings_tab_button(
                octicons::database().size(14),
                "Storage",
                SettingsTab::Storage,
                active_tab
            ),
        ]
        .spacing(10),
    )
    .width(190)
    .height(Length::Fill)
    .padding(16)
    .style(theme::settings_sidebar)
    .into()
}

fn settings_tab_button(
    icon: impl Into<Element<'static, Message>>,
    label: &'static str,
    tab: SettingsTab,
    active_tab: SettingsTab,
) -> Element<'static, Message> {
    let button = button(icon_label(icon, label))
        .on_press(Message::SettingsTabSelected(tab))
        .width(Length::Fill)
        .padding([9, 10]);

    if tab == active_tab {
        button.style(theme::active_settings_button).into()
    } else {
        button.style(theme::quiet_button).into()
    }
}

fn settings_content<'a>(
    active_tab: SettingsTab,
    session_model: &'a str,
    message_count: usize,
    db_path: String,
    is_icon_font_loaded: bool,
    is_emoji_font_loaded: bool,
    emoji_font_path: Option<String>,
    emoji_font_error: Option<String>,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    let content = match active_tab {
        SettingsTab::General => general_tab(
            is_icon_font_loaded,
            is_emoji_font_loaded,
            emoji_font_path,
            emoji_font_error,
        ),
        SettingsTab::Agent => agent_tab(session_model, is_waiting_for_agent),
        SettingsTab::Tools => tools_tab(),
        SettingsTab::Storage => storage_tab(message_count, db_path),
    };

    container(column![settings_header(active_tab), content].spacing(18))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
}

fn settings_header(active_tab: SettingsTab) -> Element<'static, Message> {
    row![
        column![
            text(active_tab.title()).size(22),
            text(active_tab.subtitle())
                .size(13)
                .color(theme::muted_text_color()),
        ]
        .spacing(3)
        .width(Length::Fill),
        button(icon_label(octicons::x().size(14), "Close"))
            .on_press(Message::CloseSettings)
            .padding([8, 12])
            .style(theme::quiet_button),
    ]
    .align_y(alignment::Vertical::Center)
    .into()
}

fn general_tab<'a>(
    is_icon_font_loaded: bool,
    is_emoji_font_loaded: bool,
    emoji_font_path: Option<String>,
    emoji_font_error: Option<String>,
) -> Element<'a, Message> {
    let mut rows = vec![
        detail_row("Theme", "System default"),
        detail_row("Window", "Desktop app"),
        detail_row("Configuration", "Environment variables"),
        detail_row(
            "Icons",
            if is_icon_font_loaded {
                "Octicons loaded"
            } else {
                "Octicons loading"
            },
        ),
        detail_row(
            "Emoji font",
            if is_emoji_font_loaded {
                "Loaded"
            } else {
                "Not loaded"
            },
        ),
        detail_row("Emoji probe", "✅ 🚀 😀 ⚠️ 🛠️"),
    ];

    if let Some(path) = emoji_font_path {
        rows.push(detail_row("Emoji font file", path));
    }

    if let Some(error) = emoji_font_error {
        rows.push(detail_row("Emoji font error", error));
    }

    settings_panel(rows)
}

fn agent_tab<'a>(session_model: &'a str, is_waiting_for_agent: bool) -> Element<'a, Message> {
    settings_panel(vec![
        detail_row("Preset", "Default"),
        detail_row("Model", session_model),
        detail_row(
            "Status",
            if is_waiting_for_agent {
                "Responding"
            } else {
                "Ready"
            },
        ),
        detail_row("Temperature", "0.7"),
        detail_row("Top P", "100"),
        detail_row("Top K", "40"),
    ])
}

fn tools_tab<'a>() -> Element<'a, Message> {
    settings_panel(vec![
        detail_row("Runtime", "Not enabled"),
        detail_row("MCP", "Planned"),
        detail_row("Tool schemas", "Planned"),
    ])
}

fn storage_tab<'a>(message_count: usize, db_path: String) -> Element<'a, Message> {
    let directory = std::path::Path::new(&db_path)
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| String::from("."));

    settings_panel(vec![
        detail_row("Storage", "SQLite local"),
        detail_row("Database file", db_path),
        detail_row("Directory", directory),
        detail_row("Current messages", message_count.to_string()),
        detail_row("Session loading", "Planned"),
    ])
}

fn settings_panel<'a>(rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut content = column!().spacing(12);

    for row in rows {
        content = content.push(row);
    }

    container(content)
        .width(Length::Fill)
        .padding(14)
        .style(theme::panel)
        .into()
}

fn detail_row<'a>(label: &'a str, value: impl Into<String>) -> Element<'a, Message> {
    row![
        text(label)
            .size(13)
            .color(theme::muted_text_color())
            .width(Length::FillPortion(2)),
        text(value.into()).size(13).width(Length::FillPortion(5)),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn icon_label<'a>(icon: impl Into<Element<'a, Message>>, label: &'a str) -> Element<'a, Message> {
    let icon = icon.into();

    row![icon, text(label).size(14)]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
}
