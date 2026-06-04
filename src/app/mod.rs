mod components;
mod theme;

use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use async_openai::types::{ChatCompletionRequestMessage, CompletionUsage};
use iced::futures::{SinkExt, channel::mpsc};
use iced::widget::{button, column, container, markdown, operation, row, stack, text, text_editor};
use iced::{Element, Length, Task, alignment, application, font, stream};
use iced_fonts::{OCTICONS_FONT_BYTES, octicons};
use std::path::PathBuf;

use crate::agent::{
    agent::AgentStreamCallback,
    agent_orchestrator::{self, AgentOrchestrator},
};
use crate::record::record_file::RecordFile;
use crate::record::{RecordFilter, RecordSqlite};
use crate::storage::chat_record::{
    MESSAGE_ROLE_ASSISTANT, MESSAGE_ROLE_SYSTEM, MESSAGE_ROLE_TOOL, MESSAGE_ROLE_USER,
    MessageRecord, SessionRecord,
};
use crate::storage::llm_provider_config::{
    LLM_PROVIDER_CONFIG_PATH, LlmModelConfig, LlmProviderConfig,
};
use crate::storage::mcp_servers_config::{
    MCP_SERVERS_CONFIG_PATH, McpServersConfig, load_or_init_mcp_servers_config,
};

const INITIAL_RECENT_SESSION_COUNT: usize = 5;
const RECENT_SESSION_PAGE_SIZE: usize = 5;
const EMPTY_AGENT_RESPONSE: &str = "The agent returned an empty response.";
const EMOJI_FONT_ENV: &str = "COWORK_EMOJI_FONT";
const EMOJI_FONT_CANDIDATES: &[&str] = &[
    "assets/fonts/emoji.ttf",
    "assets/fonts/emoji.otf",
    "assets/fonts/NotoEmoji-Regular.ttf",
    "assets/fonts/NotoColorEmoji.ttf",
    "assets/fonts/Noto-COLRv1.ttf",
    "assets/fonts/TwemojiMozilla.ttf",
    "/usr/share/fonts/google-noto-emoji-fonts/NotoEmoji-Regular.ttf",
    "/usr/share/fonts/google-noto-color-emoji-fonts/Noto-COLRv1.ttf",
    "/usr/share/fonts/truetype/noto/NotoEmoji-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    "/usr/share/fonts/noto/NotoEmoji-Regular.ttf",
    "/usr/share/fonts/noto/NotoColorEmoji.ttf",
];
pub(super) const EMOJI_FONT: iced::Font = iced::Font::with_name("Noto Emoji");

pub fn run(db: RecordSqlite, agent_orchestrator: AgentOrchestrator) -> iced::Result {
    let db = Rc::new(db);
    let mcp_server_count = agent_orchestrator.mcp_server_count();
    let mcp_tool_count = agent_orchestrator.mcp_tool_count();
    let mcp_configured_server_count = agent_orchestrator.mcp_configured_server_count();
    let agent_orchestrator = Arc::new(agent_orchestrator);

    application(
        move || {
            CoworkApp::new(
                Rc::clone(&db),
                Arc::clone(&agent_orchestrator),
                mcp_configured_server_count,
                mcp_server_count,
                mcp_tool_count,
            )
        },
        CoworkApp::update,
        CoworkApp::view,
    )
    .title(theme::title)
    .theme(theme::theme)
    .window_size(theme::WINDOW_SIZE)
    .centered()
    .run()
}

#[derive(Debug, Clone)]
pub(super) enum Message {
    DraftChanged(String),
    NewSession,
    SessionSelected(String),
    DeleteSession(String),
    LoadMoreSessions,
    OpenSettings,
    CloseSettings,
    SettingsTabSelected(SettingsTab),
    LlmProviderNameChanged(String),
    LlmBaseUrlChanged(String),
    LlmApiKeyChanged(String),
    LlmDefaultModelChanged(String),
    LlmModelsChanged(String),
    ChangeLlmProviderConfig,
    CancelLlmProviderConfigChange,
    SaveLlmProviderConfig,
    ReloadLlmProviderConfig,
    McpServersConfigEdited(text_editor::Action),
    ChangeMcpServersConfig,
    CancelMcpServersConfigChange,
    SaveMcpServersConfig,
    ReloadMcpServersConfig,
    IconFontLoaded(Result<(), font::Error>),
    EmojiFontBytesLoaded(Result<(String, Vec<u8>), String>),
    EmojiFontLoaded(String, Result<(), font::Error>),
    Send,
    ChatStreamToken(String),
    ChatStreamCompleted(String),
    ChatStreamFailed(String),
    ChatStreamUsage(CompletionUsage),
    ToolCallStarted {
        tool_name: String,
        arguments: String,
    },
    ToolCallCompleted {
        tool_name: String,
        result: String,
    },
    ToggleToolCallDetail(usize),
    MarkdownLinkClicked(markdown::Uri),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsTab {
    General,
    Agent,
    Tools,
    Storage,
}

impl SettingsTab {
    fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Agent => "Agent",
            Self::Tools => "Tools",
            Self::Storage => "Storage",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::General => "Application defaults and desktop behavior.",
            Self::Agent => "Active compile-time preset and model parameters.",
            Self::Tools => "Tool runtime and schema capabilities.",
            Self::Storage => "Local persistence and session data.",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct ChatMessage {
    pub(super) kind: ChatMessageKind,
    pub(super) author: String,
    body: String,
    pub(super) markdown: Vec<markdown::Item>,
    pub(super) blocks: Vec<ChatMessageBlock>,
    pub(super) tool_detail: Option<ToolCallDetail>,
    pub(super) is_tool_detail_open: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ToolCallDetail {
    pub(super) tool_name: String,
    pub(super) arguments: String,
    pub(super) result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChatMessageKind {
    Assistant,
    System,
    Tool,
    User,
}

#[derive(Debug, Clone)]
pub(super) enum ChatMessageBlock {
    Markdown {
        source: String,
        markdown: Vec<markdown::Item>,
    },
    Icon {
        icon: ChatIcon,
        source: String,
        markdown: Vec<markdown::Item>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChatIcon {
    Alert,
    Check,
    Code,
    Info,
    Link,
    Rocket,
    Star,
    Tools,
    X,
    Zap,
}

impl ChatMessage {
    fn assistant(body: impl Into<String>) -> Self {
        Self::new(ChatMessageKind::Assistant, "Assistant", body)
    }

    fn system(body: impl Into<String>) -> Self {
        Self::new(ChatMessageKind::System, "System", body)
    }

    fn user(body: impl Into<String>) -> Self {
        Self::new(ChatMessageKind::User, "You", body)
    }

    fn tool(tool_name: &str, body: impl Into<String>) -> Self {
        Self::new(ChatMessageKind::Tool, tool_name, body)
    }

    fn tool_call_started(tool_name: impl Into<String>, arguments: impl Into<String>) -> Self {
        let tool_name = display_tool_name(tool_name.into());
        let arguments = arguments.into();
        let mut message = Self::tool(&tool_name, format!(":tools: `{tool_name}` is running"));
        message.tool_detail = Some(ToolCallDetail {
            tool_name,
            arguments,
            result: None,
        });
        message
    }

    fn new(kind: ChatMessageKind, author: impl Into<String>, body: impl Into<String>) -> Self {
        let body = body.into();

        Self {
            kind,
            author: author.into(),
            body: body.clone(),
            markdown: markdown::parse(&body).collect(),
            blocks: parse_chat_blocks(&body),
            tool_detail: None,
            is_tool_detail_open: false,
        }
    }

    fn append_body(&mut self, token: &str) {
        self.body.push_str(token);
        self.markdown = markdown::parse(&self.body).collect();
        self.blocks = parse_chat_blocks(&self.body);
    }

    fn set_body(&mut self, body: impl Into<String>) {
        self.body = body.into();
        self.markdown = markdown::parse(&self.body).collect();
        self.blocks = parse_chat_blocks(&self.body);
    }

    pub(super) fn body(&self) -> &str {
        &self.body
    }

    fn complete_tool_call(&mut self, tool_name: &str, result: String) {
        let tool_name = display_tool_name(tool_name);
        let result_len = result.len();

        if let Some(detail) = &mut self.tool_detail {
            detail.tool_name.clone_from(&tool_name);
            detail.result = Some(result);
        } else {
            self.tool_detail = Some(ToolCallDetail {
                tool_name: tool_name.clone(),
                arguments: String::new(),
                result: Some(result),
            });
        }

        self.set_body(format!(
            ":check: `{tool_name}` completed ({result_len} chars)"
        ));
    }
}

fn display_tool_name(tool_name: impl AsRef<str>) -> String {
    let tool_name = tool_name.as_ref().trim();

    if tool_name.is_empty() {
        String::from("Unknown tool")
    } else {
        tool_name.to_owned()
    }
}

struct CoworkApp {
    db: Rc<RecordSqlite>,
    db_path: PathBuf,
    agent_orchestrator: Arc<AgentOrchestrator>,
    session_id: String,
    session_title: String,
    session_model: String,
    session_created_at: i64,
    next_sequence: i64,
    draft: String,
    recent_sessions: Vec<SessionRecord>,
    visible_session_count: usize,
    is_settings_open: bool,
    settings_tab: SettingsTab,
    is_icon_font_loaded: bool,
    is_emoji_font_loaded: bool,
    emoji_font_path: Option<String>,
    emoji_font_error: Option<String>,
    llm_provider_name: String,
    llm_base_url: String,
    llm_api_key: String,
    llm_default_model: String,
    llm_models: String,
    is_llm_provider_changing: bool,
    llm_config_status: Option<String>,
    mcp_config_editor: text_editor::Content,
    is_mcp_config_changing: bool,
    mcp_config_status: Option<String>,
    messages: Vec<ChatMessage>,
    streaming_assistant_index: Option<usize>,
    is_waiting_for_agent: bool,
    mcp_configured_server_count: usize,
    mcp_server_count: usize,
    mcp_tool_count: usize,
}

impl CoworkApp {
    fn new(
        db: Rc<RecordSqlite>,
        agent_orchestrator: Arc<AgentOrchestrator>,
        mcp_configured_server_count: usize,
        mcp_server_count: usize,
        mcp_tool_count: usize,
    ) -> (Self, Task<Message>) {
        let now = now_millis();
        let session_id = format!("session-{now}");
        let session_title = String::from("New session");
        let session_model = agent_orchestrator
            .default_agent()
            .map(|agent| agent.model())
            .unwrap_or_else(|| String::from("unknown"));
        let db_path = db.resolved_path();
        let (llm_provider_form, llm_config_status) = match load_llm_provider_form() {
            Ok(form) => (form, None),
            Err(error) => (
                LlmProviderForm::from_config(&LlmProviderConfig::default_config()),
                Some(format!("Failed to load provider config: {error}")),
            ),
        };
        let (mcp_config_editor, mcp_config_status) = match load_mcp_servers_config_json() {
            Ok(config_json) => (text_editor::Content::with_text(&config_json), None),
            Err(error) => (
                text_editor::Content::with_text(
                    &mcp_config_json(&McpServersConfig::default_config())
                        .unwrap_or_else(|_| String::from("{}")),
                ),
                Some(format!("Failed to load MCP config: {error}")),
            ),
        };

        let mut app = Self {
            db_path,
            db,
            agent_orchestrator,
            session_id,
            session_title,
            session_model,
            session_created_at: now,
            next_sequence: 0,
            draft: String::new(),
            recent_sessions: Vec::new(),
            visible_session_count: INITIAL_RECENT_SESSION_COUNT,
            is_settings_open: false,
            settings_tab: SettingsTab::Agent,
            is_icon_font_loaded: false,
            is_emoji_font_loaded: false,
            emoji_font_path: None,
            emoji_font_error: None,
            llm_provider_name: llm_provider_form.provider_name,
            llm_base_url: llm_provider_form.base_url,
            llm_api_key: llm_provider_form.api_key,
            llm_default_model: llm_provider_form.default_model,
            llm_models: llm_provider_form.models,
            is_llm_provider_changing: false,
            llm_config_status,
            mcp_config_editor,
            is_mcp_config_changing: false,
            mcp_config_status,
            messages: Vec::new(),
            streaming_assistant_index: None,
            is_waiting_for_agent: false,
            mcp_configured_server_count,
            mcp_server_count,
            mcp_tool_count,
        };

        app.refresh_recent_sessions();
        if let Some(session) = app.recent_sessions.first().cloned() {
            app.load_session(session);
        } else if let Err(error) = app.persist_session(now) {
            app.messages.push(ChatMessage::system(format!(
                "Failed to create chat session: {error}"
            )));
            app.refresh_recent_sessions();
        } else {
            app.refresh_recent_sessions();
        }

        (
            app,
            Task::batch([
                font::load(OCTICONS_FONT_BYTES).map(Message::IconFontLoaded),
                Task::perform(load_emoji_font_bytes(), Message::EmojiFontBytesLoaded),
            ]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DraftChanged(value) => {
                self.draft = value;
                Task::none()
            }
            Message::NewSession => {
                if self.is_waiting_for_agent {
                    return Task::none();
                }

                self.start_new_session();
                Task::none()
            }
            Message::SessionSelected(session_id) => {
                if self.is_waiting_for_agent || session_id == self.session_id {
                    return Task::none();
                }

                self.select_session(&session_id);
                Task::none()
            }
            Message::DeleteSession(session_id) => {
                if self.is_waiting_for_agent {
                    return Task::none();
                }

                self.delete_session(&session_id);
                Task::none()
            }
            Message::LoadMoreSessions => {
                self.visible_session_count = (self.visible_session_count
                    + RECENT_SESSION_PAGE_SIZE)
                    .min(self.recent_sessions.len());
                Task::none()
            }
            Message::OpenSettings => {
                self.is_settings_open = true;
                Task::none()
            }
            Message::CloseSettings => {
                self.is_settings_open = false;
                Task::none()
            }
            Message::SettingsTabSelected(tab) => {
                self.settings_tab = tab;
                self.is_settings_open = true;
                Task::none()
            }
            Message::LlmProviderNameChanged(value) => {
                self.llm_provider_name = value;
                self.llm_config_status = None;
                Task::none()
            }
            Message::LlmBaseUrlChanged(value) => {
                self.llm_base_url = value;
                self.llm_config_status = None;
                Task::none()
            }
            Message::LlmApiKeyChanged(value) => {
                self.llm_api_key = value;
                self.llm_config_status = None;
                Task::none()
            }
            Message::LlmDefaultModelChanged(value) => {
                self.llm_default_model = value;
                self.llm_config_status = None;
                Task::none()
            }
            Message::LlmModelsChanged(value) => {
                self.llm_models = value;
                self.llm_config_status = None;
                Task::none()
            }
            Message::ChangeLlmProviderConfig => {
                self.is_llm_provider_changing = true;
                self.llm_config_status = None;
                Task::none()
            }
            Message::CancelLlmProviderConfigChange => {
                self.cancel_llm_provider_config_change();
                Task::none()
            }
            Message::SaveLlmProviderConfig => {
                self.save_llm_provider_config();
                Task::none()
            }
            Message::ReloadLlmProviderConfig => {
                self.reload_llm_provider_config();
                Task::none()
            }
            Message::McpServersConfigEdited(action) => {
                self.mcp_config_editor.perform(action);
                self.mcp_config_status = None;
                Task::none()
            }
            Message::ChangeMcpServersConfig => {
                self.is_mcp_config_changing = true;
                self.mcp_config_status = None;
                Task::none()
            }
            Message::CancelMcpServersConfigChange => {
                self.cancel_mcp_servers_config_change();
                Task::none()
            }
            Message::SaveMcpServersConfig => {
                self.save_mcp_servers_config();
                Task::none()
            }
            Message::ReloadMcpServersConfig => {
                self.reload_mcp_servers_config();
                Task::none()
            }
            Message::IconFontLoaded(result) => {
                self.is_icon_font_loaded = result.is_ok();

                if let Err(error) = result {
                    self.messages.push(ChatMessage::system(format!(
                        "Failed to load Octicons font: {error:?}"
                    )));
                }

                Task::none()
            }
            Message::EmojiFontBytesLoaded(result) => match result {
                Ok((path, bytes)) => {
                    self.emoji_font_path = Some(path.clone());
                    self.emoji_font_error = None;
                    font::load(bytes)
                        .map(move |result| Message::EmojiFontLoaded(path.clone(), result))
                }
                Err(error) => {
                    self.is_emoji_font_loaded = false;
                    self.emoji_font_path = None;
                    self.emoji_font_error = Some(error);
                    Task::none()
                }
            },
            Message::EmojiFontLoaded(path, result) => {
                self.is_emoji_font_loaded = result.is_ok();

                match result {
                    Ok(()) => {
                        self.emoji_font_path = Some(path);
                        self.emoji_font_error = None;
                    }
                    Err(error) => {
                        self.emoji_font_path = Some(path);
                        self.emoji_font_error = Some(format!("{error:?}"));
                    }
                }

                Task::none()
            }
            Message::Send => {
                let content = self.draft.trim();

                if content.is_empty() || self.is_waiting_for_agent {
                    return Task::none();
                }

                self.messages.push(ChatMessage::user(content.to_owned()));

                if self.session_title == "New session" {
                    self.session_title = title_from_message(content);
                }

                let user_message = content.to_owned();
                if let Err(error) = self.persist_message(MESSAGE_ROLE_USER, &user_message) {
                    self.messages.push(ChatMessage::system(format!(
                        "Failed to save user message: {error}"
                    )));
                }

                if let Err(error) = self.persist_session(now_millis()) {
                    self.messages.push(ChatMessage::system(format!(
                        "Failed to update chat session: {error}"
                    )));
                }
                self.refresh_recent_sessions();

                let request_messages = match self.chat_request_messages() {
                    Ok(messages) => messages,
                    Err(error) => {
                        self.messages.push(ChatMessage::system(format!(
                            "Failed to build chat request messages: {error}"
                        )));
                        return Task::none();
                    }
                };

                let agent_orchestrator = Arc::clone(&self.agent_orchestrator);
                self.draft.clear();
                self.is_waiting_for_agent = true;
                self.messages.push(ChatMessage::assistant(String::new()));
                self.streaming_assistant_index = Some(self.messages.len() - 1);

                Task::batch([
                    Task::stream(stream::channel(100, async move |output| {
                        let callback = UiAgentStreamCallback::new(output);

                        if let Err(error) = agent_orchestrator
                            .chat_completion_stream_response(request_messages, &callback)
                            .await
                        {
                            callback.on_error(&error).await;
                        }
                    })),
                    scroll_to_bottom(),
                ])
            }
            Message::ChatStreamToken(token) => {
                if let Some(message) = self.streaming_assistant_message_mut() {
                    message.append_body(&token);
                }

                scroll_to_bottom()
            }
            Message::ChatStreamCompleted(full_text) => {
                self.is_waiting_for_agent = false;

                if full_text.trim().is_empty() && self.is_streaming_after_tool_message() {
                    self.remove_streaming_assistant_message();
                    if let Err(error) = self.persist_session(now_millis()) {
                        self.messages.push(ChatMessage::system(format!(
                            "Failed to update chat session: {error}"
                        )));
                    }
                    self.refresh_recent_sessions();
                    return scroll_to_bottom();
                }

                let body = if full_text.trim().is_empty() {
                    String::from(EMPTY_AGENT_RESPONSE)
                } else {
                    full_text
                };

                if let Some(message) = self.streaming_assistant_message_mut() {
                    message.set_body(body.clone());
                } else {
                    self.messages.push(ChatMessage::assistant(body.clone()));
                }
                self.streaming_assistant_index = None;

                if let Err(error) = self.persist_message(MESSAGE_ROLE_ASSISTANT, &body) {
                    self.messages.push(ChatMessage::system(format!(
                        "Failed to save assistant message: {error}"
                    )));
                }

                if let Err(error) = self.persist_session(now_millis()) {
                    self.messages.push(ChatMessage::system(format!(
                        "Failed to update chat session: {error}"
                    )));
                }
                self.refresh_recent_sessions();

                scroll_to_bottom()
            }
            Message::ChatStreamFailed(error) => {
                self.is_waiting_for_agent = false;
                self.remove_streaming_assistant_message();
                self.messages.push(ChatMessage::system(format!(
                    "Agent request failed: {error}"
                )));

                Task::none()
            }
            Message::ChatStreamUsage(usage) => {
                let _usage = usage;
                Task::none()
            }
            Message::ToolCallStarted {
                tool_name,
                arguments,
            } => {
                if let Some(index) = self.streaming_assistant_index {
                    if let Some(message) = self.messages.get_mut(index) {
                        *message = ChatMessage::tool_call_started(tool_name.clone(), arguments);
                    }
                    self.messages.push(ChatMessage::assistant(String::new()));
                    self.streaming_assistant_index = Some(self.messages.len() - 1);
                }
                scroll_to_bottom()
            }
            Message::ToolCallCompleted { tool_name, result } => {
                if let Some(index) = self.streaming_assistant_index {
                    if index > 0 {
                        if let Some(tool_msg) = self.messages.get_mut(index - 1) {
                            if tool_msg.kind == ChatMessageKind::Tool {
                                tool_msg.complete_tool_call(&tool_name, result);
                            }
                        }
                    }
                }
                scroll_to_bottom()
            }
            Message::ToggleToolCallDetail(index) => {
                if let Some(message) = self.messages.get_mut(index) {
                    message.is_tool_detail_open = !message.is_tool_detail_open;
                }

                Task::none()
            }
            Message::MarkdownLinkClicked(uri) => {
                let _clicked_uri = uri;
                Task::none()
            }
        }
    }

    fn persist_session(&self, updated_at: i64) -> anyhow::Result<()> {
        self.db.upsert(SessionRecord {
            session_id: self.session_id.clone(),
            title: self.session_title.clone(),
            model: self.session_model.clone(),
            created_at: self.session_created_at,
            updated_at,
            temperature: 0.7,
            top_p: 100,
            top_k: 40,
        })
    }

    fn start_new_session(&mut self) {
        let now = now_millis();

        self.session_id = format!("session-{now}");
        self.session_title = String::from("New session");
        self.session_created_at = now;
        self.next_sequence = 0;
        self.draft.clear();
        self.messages.clear();
        self.streaming_assistant_index = None;

        if let Err(error) = self.persist_session(now) {
            self.messages.push(ChatMessage::system(format!(
                "Failed to create chat session: {error}"
            )));
        }

        self.visible_session_count = INITIAL_RECENT_SESSION_COUNT;
        self.refresh_recent_sessions();
    }

    fn refresh_recent_sessions(&mut self) {
        match self.db.read_all::<SessionRecord>() {
            Ok(sessions) => {
                self.recent_sessions = sessions;
                self.visible_session_count = normalized_visible_session_count(
                    self.visible_session_count,
                    self.recent_sessions.len(),
                );
            }
            Err(error) => {
                self.messages.push(ChatMessage::system(format!(
                    "Failed to load recent sessions: {error}"
                )));
            }
        }
    }

    fn save_llm_provider_config(&mut self) {
        let config = self.llm_provider_config_from_form();
        let result = (|| -> anyhow::Result<()> {
            config.validate_resolved()?;
            let record_file = RecordFile::open(LLM_PROVIDER_CONFIG_PATH)?;
            record_file.write(&config)?;
            self.agent_orchestrator.load_provider_config()?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                self.session_model = self.active_agent_model();
                self.is_llm_provider_changing = false;
                self.llm_config_status = Some(String::from("Saved"));
            }
            Err(error) => {
                self.llm_config_status = Some(format!("Save failed: {error}"));
            }
        }
    }

    fn cancel_llm_provider_config_change(&mut self) {
        match load_llm_provider_form() {
            Ok(form) => {
                self.apply_llm_provider_form(form);
                self.is_llm_provider_changing = false;
                self.llm_config_status = None;
            }
            Err(error) => {
                self.llm_config_status = Some(format!("Cancel failed: {error}"));
            }
        }
    }

    fn reload_llm_provider_config(&mut self) {
        let result = (|| -> anyhow::Result<LlmProviderForm> {
            let form = load_llm_provider_form()?;
            self.agent_orchestrator.load_provider_config()?;
            Ok(form)
        })();

        match result {
            Ok(form) => {
                self.apply_llm_provider_form(form);
                self.session_model = self.active_agent_model();
                self.is_llm_provider_changing = false;
                self.llm_config_status = Some(String::from("Reloaded"));
            }
            Err(error) => {
                self.llm_config_status = Some(format!("Reload failed: {error}"));
            }
        }
    }

    fn save_mcp_servers_config(&mut self) {
        let result = (|| -> anyhow::Result<McpServersConfig> {
            let config: McpServersConfig = serde_json::from_str(&self.mcp_config_editor.text())?;
            let record_file = RecordFile::open(MCP_SERVERS_CONFIG_PATH)?;
            record_file.write(&config)?;
            let config = self.agent_orchestrator.reload_mcp_servers_config()?;
            Ok(config)
        })();

        match result {
            Ok(config) => {
                self.apply_mcp_servers_config(config);
                self.is_mcp_config_changing = false;
                self.mcp_config_status = Some(String::from("Saved"));
            }
            Err(error) => {
                self.mcp_config_status = Some(format!("Save failed: {error}"));
            }
        }
    }

    fn cancel_mcp_servers_config_change(&mut self) {
        match load_mcp_servers_config_json() {
            Ok(config_json) => {
                self.mcp_config_editor = text_editor::Content::with_text(&config_json);
                self.is_mcp_config_changing = false;
                self.mcp_config_status = None;
            }
            Err(error) => {
                self.mcp_config_status = Some(format!("Cancel failed: {error}"));
            }
        }
    }

    fn reload_mcp_servers_config(&mut self) {
        let result = (|| -> anyhow::Result<McpServersConfig> {
            let config = self.agent_orchestrator.reload_mcp_servers_config()?;
            Ok(config)
        })();

        match result {
            Ok(config) => {
                self.apply_mcp_servers_config(config);
                self.is_mcp_config_changing = false;
                self.mcp_config_status = Some(String::from("Reloaded"));
            }
            Err(error) => {
                self.mcp_config_status = Some(format!("Reload failed: {error}"));
            }
        }
    }

    fn apply_mcp_servers_config(&mut self, config: McpServersConfig) {
        match mcp_config_json(&config) {
            Ok(config_json) => {
                self.mcp_config_editor = text_editor::Content::with_text(&config_json);
            }
            Err(error) => {
                self.mcp_config_status = Some(format!("Failed to show MCP config: {error}"));
            }
        }

        self.mcp_configured_server_count = self.agent_orchestrator.mcp_configured_server_count();
        self.mcp_server_count = self.agent_orchestrator.mcp_server_count();
        self.mcp_tool_count = self.agent_orchestrator.mcp_tool_count();
    }

    fn apply_llm_provider_form(&mut self, form: LlmProviderForm) {
        self.llm_provider_name = form.provider_name;
        self.llm_base_url = form.base_url;
        self.llm_api_key = form.api_key;
        self.llm_default_model = form.default_model;
        self.llm_models = form.models;
    }

    fn llm_provider_config_from_form(&self) -> LlmProviderConfig {
        let default_model = self.llm_default_model.trim().to_owned();
        let models = parse_model_list(&self.llm_models, &default_model);

        LlmProviderConfig {
            provider_name: self.llm_provider_name.trim().to_owned(),
            base_url: self.llm_base_url.trim().to_owned(),
            api_key: self.llm_api_key.trim().to_owned(),
            default_model,
            models,
        }
    }

    fn llm_api_key_status(&self) -> String {
        if !self.llm_api_key.trim().is_empty() {
            return String::from("Saved");
        }

        if std::env::var("OPENAI_API_KEY")
            .ok()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return String::from("Using computer setting");
        }

        String::from("Missing")
    }

    fn active_agent_model(&self) -> String {
        self.agent_orchestrator
            .default_agent()
            .map(|agent| agent.model())
            .unwrap_or_else(|| String::from("unknown"))
    }

    fn select_session(&mut self, session_id: &str) {
        let session = self
            .recent_sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .cloned()
            .or_else(|| {
                self.refresh_recent_sessions();
                self.recent_sessions
                    .iter()
                    .find(|session| session.session_id == session_id)
                    .cloned()
            });

        match session {
            Some(session) => self.load_session(session),
            None => {
                self.messages.push(ChatMessage::system(format!(
                    "Session could not be found: {session_id}"
                )));
            }
        }
    }

    fn load_session(&mut self, session: SessionRecord) {
        match self.load_session_messages(&session.session_id) {
            Ok(messages) => {
                self.session_id = session.session_id;
                self.session_title = session.title;
                self.session_model = session.model;
                self.session_created_at = session.created_at;
                self.next_sequence = messages
                    .iter()
                    .map(|message| message.sequence)
                    .max()
                    .map_or(0, |sequence| sequence + 1);
                self.draft.clear();
                self.messages = messages
                    .into_iter()
                    .map(chat_message_from_record)
                    .collect::<Vec<_>>();
                self.streaming_assistant_index = None;
            }
            Err(error) => {
                self.messages.push(ChatMessage::system(format!(
                    "Failed to load session messages: {error}"
                )));
            }
        }
    }

    fn load_session_messages(&self, session_id: &str) -> anyhow::Result<Vec<MessageRecord>> {
        self.db
            .read::<MessageRecord>(&[RecordFilter::text("session_id", session_id)])
    }

    fn delete_session(&mut self, session_id: &str) {
        if let Err(error) = self.delete_session_records(session_id) {
            self.messages.push(ChatMessage::system(format!(
                "Failed to delete session: {error}"
            )));
            return;
        }

        let deleted_active_session = session_id == self.session_id;
        self.refresh_recent_sessions();

        if deleted_active_session {
            if let Some(session) = self.recent_sessions.first().cloned() {
                self.load_session(session);
            } else {
                self.start_new_session();
            }
        }
    }

    fn delete_session_records(&self, session_id: &str) -> anyhow::Result<()> {
        let filters = [RecordFilter::text("session_id", session_id)];
        self.db.delete::<MessageRecord>(&filters)?;
        self.db.delete::<SessionRecord>(&filters)?;

        Ok(())
    }

    fn persist_message(&mut self, role: u16, content: &str) -> anyhow::Result<()> {
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        self.db.upsert(MessageRecord {
            message_id: format!("{}-{sequence}", self.session_id),
            session_id: self.session_id.clone(),
            sequence,
            role,
            content: content.to_owned(),
            created_at: now_millis(),
        })
    }

    fn streaming_assistant_message_mut(&mut self) -> Option<&mut ChatMessage> {
        let index = self.streaming_assistant_index?;
        self.messages.get_mut(index)
    }

    fn remove_streaming_assistant_message(&mut self) {
        if let Some(index) = self.streaming_assistant_index.take() {
            if matches!(
                self.messages.get(index),
                Some(message) if message.kind == ChatMessageKind::Assistant
            ) {
                self.messages.remove(index);
            }
        }
    }

    fn is_streaming_after_tool_message(&self) -> bool {
        let Some(index) = self.streaming_assistant_index else {
            return false;
        };

        if !matches!(
            self.messages.get(index),
            Some(message)
                if message.kind == ChatMessageKind::Assistant && message.body().trim().is_empty()
        ) {
            return false;
        }

        index > 0
            && matches!(
                self.messages.get(index - 1),
                Some(message) if message.kind == ChatMessageKind::Tool
            )
    }

    fn chat_request_messages(&self) -> anyhow::Result<Vec<ChatCompletionRequestMessage>> {
        let mut messages = Vec::new();

        for message in &self.messages {
            if message.body().trim().is_empty() {
                continue;
            }

            match message.kind {
                ChatMessageKind::User => {
                    messages.push(agent_orchestrator::user_message_request(message.body())?);
                }
                ChatMessageKind::Assistant => {
                    if message.body().trim() == EMPTY_AGENT_RESPONSE {
                        continue;
                    }

                    messages.push(agent_orchestrator::assistant_message_request(
                        message.body(),
                    )?);
                }
                ChatMessageKind::System | ChatMessageKind::Tool => {}
            }
        }

        Ok(messages)
    }

    fn view(&self) -> Element<'_, Message> {
        let shell = container(column![
            top_bar(
                &self.session_title,
                &self.session_model,
                self.is_waiting_for_agent
            ),
            row![
                components::sidebar(
                    &self.session_id,
                    &self.recent_sessions,
                    self.visible_session_count,
                ),
                components::chat_area(&self.messages, &self.draft, self.is_waiting_for_agent),
            ]
            .height(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::app_background);

        if self.is_settings_open {
            let llm_api_key_status = self.llm_api_key_status();

            stack![
                shell,
                components::settings_dialog(
                    self.settings_tab,
                    &self.session_model,
                    &self.llm_provider_name,
                    &self.llm_base_url,
                    &self.llm_api_key,
                    llm_api_key_status,
                    &self.llm_default_model,
                    &self.llm_models,
                    self.is_llm_provider_changing,
                    self.llm_config_status.clone(),
                    self.messages.len(),
                    self.db_path.display().to_string(),
                    self.is_icon_font_loaded,
                    self.is_emoji_font_loaded,
                    self.emoji_font_path.clone(),
                    self.emoji_font_error.clone(),
                    self.is_waiting_for_agent,
                    self.mcp_configured_server_count,
                    self.mcp_server_count,
                    self.mcp_tool_count,
                    &self.mcp_config_editor,
                    self.is_mcp_config_changing,
                    self.mcp_config_status.clone(),
                ),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            shell.into()
        }
    }
}

fn top_bar<'a>(
    session_title: &'a str,
    session_model: &'a str,
    is_waiting_for_agent: bool,
) -> Element<'a, Message> {
    let status = if is_waiting_for_agent {
        "Responding"
    } else {
        "Ready"
    };

    container(
        row![
            column![
                text(session_title).size(16),
                text(session_model)
                    .size(12)
                    .color(theme::muted_text_color()),
            ]
            .spacing(2)
            .width(Length::Fill),
            container(text(status).size(13))
                .padding([6, 10])
                .style(theme::status_pill),
            button(
                row![octicons::gear().size(14), text("Settings").size(13)]
                    .spacing(7)
                    .align_y(alignment::Vertical::Center)
            )
            .on_press(Message::OpenSettings)
            .padding([8, 12])
            .style(theme::quiet_button),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .height(58)
    .padding([10, 18])
    .style(theme::top_bar)
    .into()
}

fn chat_message_from_record(record: MessageRecord) -> ChatMessage {
    match record.role {
        MESSAGE_ROLE_ASSISTANT => ChatMessage::assistant(record.content),
        MESSAGE_ROLE_USER => ChatMessage::user(record.content),
        MESSAGE_ROLE_SYSTEM => ChatMessage::system(record.content),
        MESSAGE_ROLE_TOOL => ChatMessage::tool("Tool", record.content),
        _ => ChatMessage::system(record.content),
    }
}

fn parse_chat_blocks(body: &str) -> Vec<ChatMessageBlock> {
    let mut blocks = Vec::new();
    let mut markdown_chunk = String::new();

    for line in body.lines() {
        if let Some((icon, rest)) = parse_icon_line(line) {
            push_markdown_block(&mut blocks, &mut markdown_chunk);
            blocks.push(ChatMessageBlock::Icon {
                icon,
                source: rest.trim().to_owned(),
                markdown: markdown::parse(rest.trim()).collect(),
            });
        } else {
            markdown_chunk.push_str(line);
            markdown_chunk.push('\n');
        }
    }

    push_markdown_block(&mut blocks, &mut markdown_chunk);

    if blocks.is_empty() {
        blocks.push(ChatMessageBlock::Markdown {
            source: body.to_owned(),
            markdown: markdown::parse(body).collect(),
        });
    }

    blocks
}

fn push_markdown_block(blocks: &mut Vec<ChatMessageBlock>, markdown_chunk: &mut String) {
    if markdown_chunk.trim().is_empty() {
        markdown_chunk.clear();
        return;
    }

    blocks.push(ChatMessageBlock::Markdown {
        source: markdown_chunk.trim().to_owned(),
        markdown: markdown::parse(markdown_chunk.trim()).collect(),
    });
    markdown_chunk.clear();
}

fn parse_icon_line(line: &str) -> Option<(ChatIcon, &str)> {
    let trimmed = line.trim_start();

    if let Some(rest) = trimmed.strip_prefix("[icon:") {
        let (name, rest) = rest.split_once(']')?;
        return chat_icon_from_name(name).map(|icon| (icon, rest));
    }

    let rest = trimmed.strip_prefix(':')?;
    let (name, rest) = rest.split_once(':')?;

    chat_icon_from_name(name).map(|icon| (icon, rest))
}

fn chat_icon_from_name(name: &str) -> Option<ChatIcon> {
    match name.trim().to_ascii_lowercase().as_str() {
        "alert" | "error" | "warning" => Some(ChatIcon::Alert),
        "check" | "success" | "done" => Some(ChatIcon::Check),
        "code" | "file_code" => Some(ChatIcon::Code),
        "info" | "note" => Some(ChatIcon::Info),
        "link" | "external" | "link_external" => Some(ChatIcon::Link),
        "rocket" => Some(ChatIcon::Rocket),
        "star" => Some(ChatIcon::Star),
        "tool" | "tools" => Some(ChatIcon::Tools),
        "x" | "close" | "failed" => Some(ChatIcon::X),
        "zap" | "bolt" => Some(ChatIcon::Zap),
        _ => None,
    }
}

struct UiAgentStreamCallback {
    output: tokio::sync::Mutex<mpsc::Sender<Message>>,
    error_sent: AtomicBool,
}

impl UiAgentStreamCallback {
    fn new(output: mpsc::Sender<Message>) -> Self {
        Self {
            output: tokio::sync::Mutex::new(output),
            error_sent: AtomicBool::new(false),
        }
    }

    async fn send(&self, message: Message) {
        let mut output = self.output.lock().await;
        let _ = output.send(message).await;
    }
}

#[async_trait::async_trait]
impl AgentStreamCallback for UiAgentStreamCallback {
    async fn on_token(&self, token: &str) {
        self.send(Message::ChatStreamToken(token.to_owned())).await;
    }

    async fn on_complete(&self, full_text: &str) {
        self.send(Message::ChatStreamCompleted(full_text.to_owned()))
            .await;
    }

    async fn on_error(&self, error: &anyhow::Error) {
        if !self.error_sent.swap(true, Ordering::Relaxed) {
            self.send(Message::ChatStreamFailed(format!("{error:#}")))
                .await;
        }
    }

    async fn on_usage(&self, usage: &CompletionUsage) {
        self.send(Message::ChatStreamUsage(usage.clone())).await;
    }

    async fn on_tool_start(&self, tool_name: &str, arguments: &str) {
        self.send(Message::ToolCallStarted {
            tool_name: tool_name.to_owned(),
            arguments: arguments.to_owned(),
        })
        .await;
    }

    async fn on_tool_result(&self, tool_name: &str, result: &str) {
        self.send(Message::ToolCallCompleted {
            tool_name: tool_name.to_owned(),
            result: result.to_owned(),
        })
        .await;
    }
}

fn scroll_to_bottom() -> Task<Message> {
    operation::snap_to_end(components::TRANSCRIPT_SCROLLABLE_ID)
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn title_from_message(message: &str) -> String {
    let title = message.trim();
    let mut title = title.chars().take(48).collect::<String>();

    if title.is_empty() {
        title = String::from("New session");
    }

    title
}

fn normalized_visible_session_count(current: usize, total: usize) -> usize {
    if total <= INITIAL_RECENT_SESSION_COUNT {
        total
    } else {
        current.clamp(INITIAL_RECENT_SESSION_COUNT, total)
    }
}

struct LlmProviderForm {
    provider_name: String,
    base_url: String,
    api_key: String,
    default_model: String,
    models: String,
}

impl LlmProviderForm {
    fn from_config(config: &LlmProviderConfig) -> Self {
        Self {
            provider_name: config.provider_name.clone(),
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            default_model: config.default_model.clone(),
            models: config
                .models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

fn load_llm_provider_form() -> anyhow::Result<LlmProviderForm> {
    let record_file = RecordFile::open(LLM_PROVIDER_CONFIG_PATH)?;
    record_file.init(&LlmProviderConfig::default_config())?;
    let config: LlmProviderConfig = record_file.read()?;

    Ok(LlmProviderForm::from_config(&config))
}

fn load_mcp_servers_config_json() -> anyhow::Result<String> {
    let config = load_or_init_mcp_servers_config()?;
    mcp_config_json(&config)
}

fn mcp_config_json(config: &McpServersConfig) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(config)?)
}

fn parse_model_list(models: &str, default_model: &str) -> Vec<LlmModelConfig> {
    let mut parsed = models
        .split(|ch| ch == ',' || ch == '\n')
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(|model| LlmModelConfig {
            id: model.to_owned(),
            display_name: model.to_owned(),
        })
        .collect::<Vec<_>>();

    if parsed.is_empty() && !default_model.trim().is_empty() {
        let model = default_model.trim().to_owned();
        parsed.push(LlmModelConfig {
            id: model.clone(),
            display_name: model,
        });
    }

    parsed
}

async fn load_emoji_font_bytes() -> Result<(String, Vec<u8>), String> {
    let mut candidates = Vec::new();

    if let Ok(path) = std::env::var(EMOJI_FONT_ENV) {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }

    candidates.extend(EMOJI_FONT_CANDIDATES.iter().map(PathBuf::from));

    for path in candidates {
        if !path.is_file() {
            continue;
        }

        let bytes = std::fs::read(&path)
            .map_err(|error| format!("failed to read emoji font {}: {error}", path.display()))?;

        return Ok((path.display().to_string(), bytes));
    }

    Err(format!(
        "no emoji font found; set {EMOJI_FONT_ENV} to a .ttf or .otf emoji font"
    ))
}
