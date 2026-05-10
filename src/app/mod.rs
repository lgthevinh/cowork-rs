mod components;
mod theme;

use std::rc::Rc;
use std::sync::Arc;

use iced::{Element, Length, Task, application};

use crate::agent::agent_orchestrator::AgentOrchestrator;
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
    ChatCompleted(Result<String, String>),
}

#[derive(Debug, Clone)]
pub(super) struct ChatMessage {
    pub(super) author: String,
    pub(super) body: String,
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
            messages: vec![
                ChatMessage {
                    author: String::from("System"),
                    body: String::from("Cowork RS is ready."),
                },
                ChatMessage {
                    author: String::from("Assistant"),
                    body: String::from("Create agents, connect tools, and keep the UI responsive."),
                },
            ],
            is_waiting_for_agent: false,
        };

        if let Err(error) = app.persist_session(now) {
            app.messages.push(ChatMessage {
                author: String::from("System"),
                body: format!("Failed to create chat session: {error}"),
            });
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

                self.messages.push(ChatMessage {
                    author: String::from("You"),
                    body: content.to_owned(),
                });

                if self.session_title == "New session" {
                    self.session_title = title_from_message(content);
                }

                let user_message = content.to_owned();
                if let Err(error) = self.persist_message(MESSAGE_ROLE_USER, &user_message) {
                    self.messages.push(ChatMessage {
                        author: String::from("System"),
                        body: format!("Failed to save user message: {error}"),
                    });
                }

                if let Err(error) = self.persist_session(now_millis()) {
                    self.messages.push(ChatMessage {
                        author: String::from("System"),
                        body: format!("Failed to update chat session: {error}"),
                    });
                }

                let agent_orchestrator = Arc::clone(&self.agent_orchestrator);
                self.draft.clear();
                self.is_waiting_for_agent = true;

                Task::perform(
                    async move {
                        agent_orchestrator
                            .chat_completion(&user_message)
                            .await
                            .map_err(|error| error.to_string())
                    },
                    Message::ChatCompleted,
                )
            }
            Message::ChatCompleted(result) => {
                self.is_waiting_for_agent = false;

                let body = match result {
                    Ok(content) if !content.trim().is_empty() => content,
                    Ok(_) => String::from("The agent returned an empty response."),
                    Err(error) => format!("Agent request failed: {error}"),
                };

                self.messages.push(ChatMessage {
                    author: String::from("Assistant"),
                    body: body.clone(),
                });

                if let Err(error) = self.persist_message(MESSAGE_ROLE_ASSISTANT, &body) {
                    self.messages.push(ChatMessage {
                        author: String::from("System"),
                        body: format!("Failed to save assistant message: {error}"),
                    });
                }

                if let Err(error) = self.persist_session(now_millis()) {
                    self.messages.push(ChatMessage {
                        author: String::from("System"),
                        body: format!("Failed to update chat session: {error}"),
                    });
                }

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

    fn view(&self) -> Element<'_, Message> {
        iced::widget::container(iced::widget::row![
            components::sidebar(),
            components::chat_area(&self.messages, &self.draft, self.is_waiting_for_agent),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
