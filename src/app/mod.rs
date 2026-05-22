mod components;
mod theme;

use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use async_openai::types::{ChatCompletionRequestMessage, CompletionUsage};
use iced::futures::{SinkExt, channel::mpsc};
use iced::widget::{button, column, container, markdown, row, stack, text};
use iced::{Element, Length, Task, alignment, application, font, stream};
use iced_fonts::{OCTICONS_FONT_BYTES, octicons};

use crate::agent::{
    agent::AgentStreamCallback,
    agent_orchestrator::{self, AgentOrchestrator},
};
use crate::repo::SqliteDb;
use crate::repo::record::record_impl::{
    MESSAGE_ROLE_ASSISTANT, MESSAGE_ROLE_SYSTEM, MESSAGE_ROLE_TOOL, MESSAGE_ROLE_USER,
    MessageRecord, SessionRecord,
};
use crate::repo::repo::Repo;
use crate::repo::repo_filter::RepoFilter;
use crate::repo::repo_impl::{MessageRepo, SessionRepo};

const INITIAL_RECENT_SESSION_COUNT: usize = 5;
const RECENT_SESSION_PAGE_SIZE: usize = 5;

pub fn run(db: SqliteDb, agent_orchestrator: AgentOrchestrator) -> iced::Result {
    let db = Rc::new(db);
    let agent_orchestrator = Arc::new(agent_orchestrator);

    application(
        move || CoworkApp::new(Rc::clone(&db), Arc::clone(&agent_orchestrator)),
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
    IconFontLoaded(Result<(), font::Error>),
    Send,
    ChatStreamToken(String),
    ChatStreamCompleted(String),
    ChatStreamFailed(String),
    ChatStreamUsage(CompletionUsage),
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChatMessageKind {
    Assistant,
    System,
    User,
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

    fn new(kind: ChatMessageKind, author: impl Into<String>, body: impl Into<String>) -> Self {
        let body = body.into();

        Self {
            kind,
            author: author.into(),
            body: body.clone(),
            markdown: markdown::parse(&body).collect(),
        }
    }

    fn append_body(&mut self, token: &str) {
        self.body.push_str(token);
        self.markdown = markdown::parse(&self.body).collect();
    }

    fn set_body(&mut self, body: impl Into<String>) {
        self.body = body.into();
        self.markdown = markdown::parse(&self.body).collect();
    }

    pub(super) fn body(&self) -> &str {
        &self.body
    }
}

struct CoworkApp {
    db: Rc<SqliteDb>,
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
    messages: Vec<ChatMessage>,
    streaming_assistant_index: Option<usize>,
    is_waiting_for_agent: bool,
}

impl CoworkApp {
    fn new(db: Rc<SqliteDb>, agent_orchestrator: Arc<AgentOrchestrator>) -> (Self, Task<Message>) {
        let now = now_millis();
        let session_id = format!("session-{now}");
        let session_title = String::from("New session");
        let session_model = agent_orchestrator
            .default_agent()
            .map(|agent| agent.model().to_owned())
            .unwrap_or_else(|| String::from("unknown"));

        let mut app = Self {
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
            messages: Vec::new(),
            streaming_assistant_index: None,
            is_waiting_for_agent: false,
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
            font::load(OCTICONS_FONT_BYTES).map(Message::IconFontLoaded),
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
            Message::IconFontLoaded(result) => {
                self.is_icon_font_loaded = result.is_ok();

                if let Err(error) = result {
                    self.messages.push(ChatMessage::system(format!(
                        "Failed to load Octicons font: {error:?}"
                    )));
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

                Task::stream(stream::channel(100, async move |output| {
                    let callback = UiAgentStreamCallback::new(output);

                    if let Err(error) = agent_orchestrator
                        .chat_completion_stream_response(request_messages, &callback)
                        .await
                    {
                        callback.on_error(&error).await;
                    }
                }))
            }
            Message::ChatStreamToken(token) => {
                if let Some(message) = self.streaming_assistant_message_mut() {
                    message.append_body(&token);
                }

                Task::none()
            }
            Message::ChatStreamCompleted(full_text) => {
                self.is_waiting_for_agent = false;

                let body = if full_text.trim().is_empty() {
                    String::from("The agent returned an empty response.")
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

                Task::none()
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
            Message::MarkdownLinkClicked(uri) => {
                let _clicked_uri = uri;
                Task::none()
            }
        }
    }

    fn persist_session(&self, updated_at: i64) -> anyhow::Result<()> {
        let repo = SessionRepo::new(&self.db);

        repo.upsert(SessionRecord {
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
        let repo = SessionRepo::new(&self.db);

        match repo.read_all() {
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
        MessageRepo::new(&self.db).read(&[RepoFilter::text("session_id", session_id)])
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
        let filters = [RepoFilter::text("session_id", session_id)];
        MessageRepo::new(&self.db).delete(&filters)?;
        SessionRepo::new(&self.db).delete(&filters)?;

        Ok(())
    }

    fn persist_message(&mut self, role: u16, content: &str) -> anyhow::Result<()> {
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        let repo = MessageRepo::new(&self.db);

        repo.upsert(MessageRecord {
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
                    messages.push(agent_orchestrator::assistant_message_request(
                        message.body(),
                    )?);
                }
                ChatMessageKind::System => {}
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
            stack![
                shell,
                components::settings_dialog(
                    self.settings_tab,
                    &self.session_model,
                    self.messages.len(),
                    self.is_icon_font_loaded,
                    self.is_waiting_for_agent
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
        MESSAGE_ROLE_TOOL => ChatMessage::system(format!("Tool\n\n{}", record.content)),
        _ => ChatMessage::system(record.content),
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
            self.send(Message::ChatStreamFailed(error.to_string()))
                .await;
        }
    }

    async fn on_usage(&self, usage: &CompletionUsage) {
        self.send(Message::ChatStreamUsage(usage.clone())).await;
    }
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
