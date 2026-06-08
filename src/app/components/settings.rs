use iced::widget::{
    button, column, container, opaque, row, scrollable, stack, text, text_editor, text_input,
};
use iced::{Element, Length, alignment};
use iced_fonts::octicons;

use super::super::{Message, SettingsTab, theme};

pub(in crate::app) fn settings_dialog<'a>(
    active_tab: SettingsTab,
    session_model: &'a str,
    llm_provider_name: &'a str,
    llm_base_url: &'a str,
    llm_api_key: &'a str,
    llm_api_key_status: String,
    llm_default_model: &'a str,
    llm_models: &'a str,
    is_llm_provider_changing: bool,
    message_count: usize,
    db_path: String,
    is_icon_font_loaded: bool,
    is_emoji_font_loaded: bool,
    emoji_font_path: Option<String>,
    emoji_font_error: Option<String>,
    is_waiting_for_agent: bool,
    mcp_configured_server_count: usize,
    mcp_server_count: usize,
    mcp_tool_count: usize,
    mcp_config_editor: &'a text_editor::Content,
    is_mcp_config_changing: bool,
) -> Element<'a, Message> {
    let dialog = container(
        row![
            settings_sidebar(active_tab),
            settings_content(
                active_tab,
                session_model,
                llm_provider_name,
                llm_base_url,
                llm_api_key,
                llm_api_key_status,
                llm_default_model,
                llm_models,
                is_llm_provider_changing,
                message_count,
                db_path,
                is_icon_font_loaded,
                is_emoji_font_loaded,
                emoji_font_path,
                emoji_font_error,
                is_waiting_for_agent,
                mcp_configured_server_count,
                mcp_server_count,
                mcp_tool_count,
                mcp_config_editor,
                is_mcp_config_changing,
            ),
        ]
        .height(Length::Fill),
    )
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
    llm_provider_name: &'a str,
    llm_base_url: &'a str,
    llm_api_key: &'a str,
    llm_api_key_status: String,
    llm_default_model: &'a str,
    llm_models: &'a str,
    is_llm_provider_changing: bool,
    message_count: usize,
    db_path: String,
    is_icon_font_loaded: bool,
    is_emoji_font_loaded: bool,
    emoji_font_path: Option<String>,
    emoji_font_error: Option<String>,
    is_waiting_for_agent: bool,
    mcp_configured_server_count: usize,
    mcp_server_count: usize,
    mcp_tool_count: usize,
    mcp_config_editor: &'a text_editor::Content,
    is_mcp_config_changing: bool,
) -> Element<'a, Message> {
    let content = match active_tab {
        SettingsTab::General => general_tab(
            is_icon_font_loaded,
            is_emoji_font_loaded,
            emoji_font_path,
            emoji_font_error,
        ),
        SettingsTab::Agent => agent_tab(
            session_model,
            llm_provider_name,
            llm_base_url,
            llm_api_key,
            llm_api_key_status,
            llm_default_model,
            llm_models,
            is_llm_provider_changing,
            is_waiting_for_agent,
        ),
        SettingsTab::Tools => tools_tab(
            mcp_configured_server_count,
            mcp_server_count,
            mcp_tool_count,
            mcp_config_editor,
            is_mcp_config_changing,
            is_waiting_for_agent,
        ),
        SettingsTab::Storage => storage_tab(message_count, db_path),
    };

    let body = scrollable(content).width(Length::Fill).height(Length::Fill);

    container(
        column![settings_header(active_tab), body]
            .spacing(18)
            .height(Length::Fill),
    )
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
            .padding([6, 10])
            .style(theme::quiet_button),
    ]
    .align_y(alignment::Vertical::Top)
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
        detail_row("Provider settings", "preference/llm-provider.json"),
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

fn agent_tab<'a>(
    session_model: &'a str,
    llm_provider_name: &'a str,
    llm_base_url: &'a str,
    llm_api_key: &'a str,
    llm_api_key_status: String,
    llm_default_model: &'a str,
    llm_models: &'a str,
    is_llm_provider_changing: bool,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    if is_llm_provider_changing {
        return agent_change_tab(
            session_model,
            llm_provider_name,
            llm_base_url,
            llm_api_key,
            llm_default_model,
            llm_models,
            is_waiting_for_agent,
        );
    }

    let rows = vec![
        detail_row("Preset", "Default"),
        detail_row("Active model", session_model),
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
        section_label("Provider"),
        detail_row("Provider", llm_provider_name),
        detail_row("Service URL", llm_base_url),
        detail_row("API key", llm_api_key_status),
        detail_row("Main model", llm_default_model),
        detail_row("Available models", llm_models),
        row![
            button(icon_label(octicons::pencil().size(14), "Change"))
                .on_press(Message::ChangeLlmProviderConfig)
                .padding([7, 10])
                .style(theme::quiet_button),
            button(icon_label(octicons::sync().size(14), "Reload"))
                .on_press(Message::ReloadLlmProviderConfig)
                .padding([7, 10])
                .style(theme::quiet_button),
        ]
        .spacing(8)
        .into(),
    ];

    settings_panel(rows)
}

fn agent_change_tab<'a>(
    session_model: &'a str,
    llm_provider_name: &'a str,
    llm_base_url: &'a str,
    llm_api_key: &'a str,
    llm_default_model: &'a str,
    llm_models: &'a str,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    let save_button = button(icon_label(octicons::check().size(14), "Save changes"))
        .padding([7, 10])
        .style(theme::quiet_button);
    let save_button = if is_waiting_for_agent {
        save_button
    } else {
        save_button.on_press(Message::SaveLlmProviderConfig)
    };

    let rows = vec![
        detail_row("Active model", session_model),
        detail_row(
            "Status",
            if is_waiting_for_agent {
                "Responding"
            } else {
                "Ready"
            },
        ),
        section_label("Change provider"),
        input_row(
            "Provider",
            "OpenAI compatible",
            llm_provider_name,
            Message::LlmProviderNameChanged,
        ),
        input_row(
            "Service URL",
            "https://api.openai.com/v1",
            llm_base_url,
            Message::LlmBaseUrlChanged,
        ),
        input_row(
            "API key",
            "Uses OPENAI_API_KEY when empty",
            llm_api_key,
            Message::LlmApiKeyChanged,
        ),
        input_row(
            "Main model",
            "mimo-v2.5",
            llm_default_model,
            Message::LlmDefaultModelChanged,
        ),
        input_row(
            "Available models",
            "model-a, model-b",
            llm_models,
            Message::LlmModelsChanged,
        ),
        row![
            save_button,
            button(icon_label(octicons::x().size(14), "Cancel"))
                .on_press(Message::CancelLlmProviderConfigChange)
                .padding([7, 10])
                .style(theme::quiet_button),
        ]
        .spacing(8)
        .into(),
    ];

    settings_panel(rows)
}

fn section_label<'a>(label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(14)
        .color(theme::muted_text_color())
        .width(Length::Fill)
        .into()
}

fn tools_tab<'a>(
    mcp_configured_server_count: usize,
    mcp_server_count: usize,
    mcp_tool_count: usize,
    mcp_config_editor: &'a text_editor::Content,
    is_mcp_config_changing: bool,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    if is_mcp_config_changing {
        return tools_change_tab(
            mcp_configured_server_count,
            mcp_server_count,
            mcp_tool_count,
            mcp_config_editor,
            is_waiting_for_agent,
        );
    }

    let mcp_status = if mcp_server_count > 0 {
        format!("{mcp_server_count} server(s), {mcp_tool_count} tool(s)")
    } else {
        "No servers connected".to_owned()
    };

    let rows = vec![
        detail_row("Runtime", "Active"),
        detail_row("Config file", "preference/mcp-servers.json"),
        detail_row(
            "Configured servers",
            mcp_configured_server_count.to_string(),
        ),
        detail_row("MCP", mcp_status),
        detail_row("Tool schemas", "JSON Schema"),
        row![
            button(icon_label(octicons::pencil().size(14), "Change"))
                .on_press(Message::ChangeMcpServersConfig)
                .padding([7, 10])
                .style(theme::quiet_button),
            button(icon_label(octicons::sync().size(14), "Reload"))
                .on_press(Message::ReloadMcpServersConfig)
                .padding([7, 10])
                .style(theme::quiet_button),
        ]
        .spacing(8)
        .into(),
    ];

    settings_panel(rows)
}

fn tools_change_tab<'a>(
    mcp_configured_server_count: usize,
    mcp_server_count: usize,
    mcp_tool_count: usize,
    mcp_config_editor: &'a text_editor::Content,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    let save_button = button(icon_label(octicons::check().size(14), "Save changes"))
        .padding([7, 10])
        .style(theme::quiet_button);
    let save_button = if is_waiting_for_agent {
        save_button
    } else {
        save_button.on_press(Message::SaveMcpServersConfig)
    };

    let rows = vec![
        detail_row("Config file", "preference/mcp-servers.json"),
        detail_row(
            "Configured servers",
            mcp_configured_server_count.to_string(),
        ),
        detail_row(
            "Connected",
            format!("{mcp_server_count} server(s), {mcp_tool_count} tool(s)"),
        ),
        section_label("Change MCP servers"),
        editor_row("Config JSON", mcp_config_editor),
        row![
            save_button,
            button(icon_label(octicons::x().size(14), "Cancel"))
                .on_press(Message::CancelMcpServersConfigChange)
                .padding([7, 10])
                .style(theme::quiet_button),
        ]
        .spacing(8)
        .into(),
    ];

    settings_panel(rows)
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

fn input_row<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    row![
        text(label)
            .size(13)
            .color(theme::muted_text_color())
            .width(Length::FillPortion(2)),
        text_input(placeholder, value)
            .on_input(on_input)
            .size(13)
            .padding([6, 8])
            .width(Length::FillPortion(5)),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn editor_row<'a>(label: &'a str, content: &'a text_editor::Content) -> Element<'a, Message> {
    row![
        text(label)
            .size(13)
            .color(theme::muted_text_color())
            .width(Length::FillPortion(2)),
        container(
            text_editor(content)
                .placeholder(r#"{"mcpServers":{}}"#)
                .on_action(Message::McpServersConfigEdited)
                .height(180)
                .padding([6, 8])
        )
        .width(Length::FillPortion(5))
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Top)
    .into()
}

fn icon_label<'a>(icon: impl Into<Element<'a, Message>>, label: &'a str) -> Element<'a, Message> {
    let icon = icon.into();

    row![icon, text(label).size(14)]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into()
}
