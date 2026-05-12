mod components;
mod theme;

use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use async_openai::types::{ChatCompletionRequestMessage, CompletionUsage};
use iced::futures::{SinkExt, channel::mpsc};
use iced::widget::markdown;
use iced::{Element, Length, Task, application, stream};

use crate::agent::{
    agent::AgentStreamCallback,
    agent_orchestrator::{self, AgentOrchestrator},
};
use crate::repo::SqliteDb;
use crate::repo::record::record_impl::{
    MESSAGE_ROLE_ASSISTANT, MESSAGE_ROLE_USER, MessageRecord, SessionRecord,
};
use crate::repo::repo::Repo;
use crate::repo::repo_impl::{MessageRepo, SessionRepo};

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
    Send,
    ChatStreamToken(String),
    ChatStreamCompleted(String),
    ChatStreamFailed(String),
    ChatStreamUsage(CompletionUsage),
    MarkdownLinkClicked(markdown::Uri),
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

    fn body(&self) -> &str {
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
    messages: Vec<ChatMessage>,
    streaming_assistant_index: Option<usize>,
    is_waiting_for_agent: bool,
}

impl CoworkApp {
    fn new(db: Rc<SqliteDb>, agent_orchestrator: Arc<AgentOrchestrator>) -> Self {
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
            messages: Vec::new(),
            streaming_assistant_index: None,
            is_waiting_for_agent: false,
        };

        if let Err(error) = app.persist_session(now) {
            app.messages.push(ChatMessage::system(format!(
                "Failed to create chat session: {error}"
            )));
        }

        app
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DraftChanged(value) => {
                self.draft = value;
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
        iced::widget::container(iced::widget::row![
            components::sidebar(),
            components::chat_area(&self.messages, &self.draft, self.is_waiting_for_agent),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::app_background)
        .into()
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
