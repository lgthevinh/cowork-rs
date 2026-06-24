use std::path::PathBuf;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, bail};
use async_openai::types::CompletionUsage;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

use crate::agent::agent::AgentStreamCallback;
use crate::agent::agent_orchestrator::{self, AgentOrchestrator};
use crate::record::record_file::RecordFile;
use crate::record::{RecordFilter, RecordSqlite};
use crate::storage::chat_record::{
    MESSAGE_ROLE_ASSISTANT, MESSAGE_ROLE_SYSTEM, MESSAGE_ROLE_TOOL, MESSAGE_ROLE_USER,
    MessageRecord, SessionRecord,
};
use crate::storage::llm_provider_config::{LLM_PROVIDER_CONFIG_PATH, LlmProviderConfig};
use crate::storage::mcp_servers_config::{
    MCP_SERVERS_CONFIG_PATH, McpServersConfig, load_or_init_mcp_servers_config,
};

const EMPTY_AGENT_RESPONSE: &str = "The agent returned an empty response.";

pub struct CoworkBackend {
    db: Mutex<RecordSqlite>,
    db_path: PathBuf,
    agent_orchestrator: AgentOrchestrator,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapResponse {
    pub db_path: String,
    pub active_session: SessionRecord,
    pub sessions: Vec<SessionRecord>,
    pub messages: Vec<MessageRecord>,
    pub settings: SettingsSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedSession {
    pub active_session: SessionRecord,
    pub sessions: Vec<SessionRecord>,
    pub messages: Vec<MessageRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub provider_config: LlmProviderConfig,
    pub mcp_config_json: String,
    pub mcp_configured_server_count: usize,
    pub mcp_server_count: usize,
    pub mcp_tool_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub session_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ChatStreamEvent {
    Token {
        token: String,
    },
    Completed {
        full_text: String,
    },
    Error {
        error: String,
    },
    Usage {
        summary: String,
    },
    ToolStarted {
        tool_name: String,
        arguments: String,
    },
    ToolCompleted {
        tool_name: String,
        result: String,
    },
}

impl CoworkBackend {
    pub fn init() -> anyhow::Result<Self> {
        let db = RecordSqlite::open("data.db").context("failed to open sqlite record storage")?;
        db.init::<SessionRecord>()
            .context("failed to initialize session schema")?;
        db.init::<MessageRecord>()
            .context("failed to initialize message schema")?;
        let db_path = db.resolved_path();

        let agent_orchestrator =
            agent_orchestrator::init().context("failed to initialize agent orchestrator")?;

        Ok(Self {
            db: Mutex::new(db),
            db_path,
            agent_orchestrator,
        })
    }

    pub fn bootstrap(&self) -> anyhow::Result<BootstrapResponse> {
        let loaded = self.load_initial_session()?;

        Ok(BootstrapResponse {
            db_path: self.db_path.display().to_string(),
            active_session: loaded.active_session,
            sessions: loaded.sessions,
            messages: loaded.messages,
            settings: self.settings_snapshot()?,
        })
    }

    pub fn load_session(&self, session_id: &str) -> anyhow::Result<LoadedSession> {
        let db = self.db()?;
        let session = read_session(&db, session_id)?;
        let messages = read_messages(&db, session_id)?;
        let sessions = db.read_all::<SessionRecord>()?;

        Ok(LoadedSession {
            active_session: session,
            sessions,
            messages,
        })
    }

    pub fn new_session(&self) -> anyhow::Result<LoadedSession> {
        let mut db = self.db()?;
        let session = self.create_session(&mut db)?;
        let sessions = db.read_all::<SessionRecord>()?;

        Ok(LoadedSession {
            active_session: session,
            sessions,
            messages: Vec::new(),
        })
    }

    pub fn delete_session(&self, session_id: &str) -> anyhow::Result<LoadedSession> {
        let mut db = self.db()?;
        let filters = [RecordFilter::text("session_id", session_id)];
        db.delete::<MessageRecord>(&filters)?;
        db.delete::<SessionRecord>(&filters)?;

        if let Some(session) = db.read_all::<SessionRecord>()?.into_iter().next() {
            let messages = read_messages(&db, &session.session_id)?;
            let sessions = db.read_all::<SessionRecord>()?;
            return Ok(LoadedSession {
                active_session: session,
                sessions,
                messages,
            });
        }

        let session = self.create_session(&mut db)?;
        let sessions = db.read_all::<SessionRecord>()?;

        Ok(LoadedSession {
            active_session: session,
            sessions,
            messages: Vec::new(),
        })
    }

    pub async fn send_message(
        &self,
        request: SendMessageRequest,
        events: Channel<ChatStreamEvent>,
    ) -> anyhow::Result<LoadedSession> {
        let content = request.content.trim().to_owned();
        if content.is_empty() {
            bail!("message content cannot be empty");
        }

        let request_messages = {
            let db = self.db()?;
            let mut session = read_session(&db, &request.session_id)?;
            let mut messages = read_messages(&db, &request.session_id)?;
            let next_sequence = next_sequence(&messages);
            let now = now_millis();

            if session.title == "New session" {
                session.title = title_from_message(&content);
            }
            session.updated_at = now;

            db.upsert(MessageRecord {
                message_id: format!("{}-{next_sequence}", session.session_id),
                session_id: session.session_id.clone(),
                sequence: next_sequence,
                role: MESSAGE_ROLE_USER,
                content: content.clone(),
                created_at: now,
            })?;
            db.upsert(session)?;

            messages.push(MessageRecord {
                message_id: format!("{}-{next_sequence}", request.session_id),
                session_id: request.session_id.clone(),
                sequence: next_sequence,
                role: MESSAGE_ROLE_USER,
                content,
                created_at: now,
            });

            chat_request_messages(&messages)?
        };

        let callback = TauriAgentStreamCallback::new(events);
        let response = self
            .agent_orchestrator
            .chat_completion_stream_response(request_messages, &callback)
            .await;

        match response {
            Ok(response) => {
                let body = if response.content.trim().is_empty() {
                    if callback.tool_event_seen() {
                        None
                    } else {
                        Some(String::from(EMPTY_AGENT_RESPONSE))
                    }
                } else {
                    Some(response.content)
                };

                let db = self.db()?;
                if let Some(body) = body {
                    let messages = read_messages(&db, &request.session_id)?;
                    db.upsert(MessageRecord {
                        message_id: format!("{}-{}", request.session_id, next_sequence(&messages)),
                        session_id: request.session_id.clone(),
                        sequence: next_sequence(&messages),
                        role: MESSAGE_ROLE_ASSISTANT,
                        content: body,
                        created_at: now_millis(),
                    })?;
                }

                let mut session = read_session(&db, &request.session_id)?;
                session.updated_at = now_millis();
                db.upsert(session)?;
                drop(db);

                self.load_session(&request.session_id)
            }
            Err(error) => {
                callback.on_error(&error).await;
                Err(error)
            }
        }
    }

    pub fn settings_snapshot(&self) -> anyhow::Result<SettingsSnapshot> {
        let provider_config = load_provider_config()?;
        let mcp_config = load_or_init_mcp_servers_config()?;
        let mcp_config_json = serde_json::to_string_pretty(&mcp_config)?;

        Ok(SettingsSnapshot {
            provider_config,
            mcp_config_json,
            mcp_configured_server_count: self.agent_orchestrator.mcp_configured_server_count(),
            mcp_server_count: self.agent_orchestrator.mcp_server_count(),
            mcp_tool_count: self.agent_orchestrator.mcp_tool_count(),
        })
    }

    pub fn save_provider_config(
        &self,
        config: LlmProviderConfig,
    ) -> anyhow::Result<SettingsSnapshot> {
        config.validate_resolved()?;
        let record_file = RecordFile::open(LLM_PROVIDER_CONFIG_PATH)?;
        record_file.write(&config)?;
        self.agent_orchestrator.load_provider_config()?;
        self.settings_snapshot()
    }

    pub fn save_mcp_servers_config(&self, config_json: &str) -> anyhow::Result<SettingsSnapshot> {
        let config: McpServersConfig = serde_json::from_str(config_json)?;
        let record_file = RecordFile::open(MCP_SERVERS_CONFIG_PATH)?;
        record_file.write(&config)?;
        self.agent_orchestrator.reload_mcp_servers_config()?;
        self.settings_snapshot()
    }

    pub fn reload_provider_config(&self) -> anyhow::Result<SettingsSnapshot> {
        self.agent_orchestrator.load_provider_config()?;
        self.settings_snapshot()
    }

    pub fn reload_mcp_servers_config(&self) -> anyhow::Result<SettingsSnapshot> {
        self.agent_orchestrator.reload_mcp_servers_config()?;
        self.settings_snapshot()
    }

    fn load_initial_session(&self) -> anyhow::Result<LoadedSession> {
        let mut db = self.db()?;

        if let Some(session) = db.read_all::<SessionRecord>()?.into_iter().next() {
            let messages = read_messages(&db, &session.session_id)?;
            let sessions = db.read_all::<SessionRecord>()?;
            return Ok(LoadedSession {
                active_session: session,
                sessions,
                messages,
            });
        }

        let session = self.create_session(&mut db)?;
        let sessions = db.read_all::<SessionRecord>()?;

        Ok(LoadedSession {
            active_session: session,
            sessions,
            messages: Vec::new(),
        })
    }

    fn create_session(
        &self,
        db: &mut std::sync::MutexGuard<'_, RecordSqlite>,
    ) -> anyhow::Result<SessionRecord> {
        let now = now_millis();
        let model = self
            .agent_orchestrator
            .default_agent()
            .map(|agent| agent.model())
            .unwrap_or_else(|| String::from("unknown"));

        let session = SessionRecord {
            session_id: format!("session-{now}"),
            title: String::from("New session"),
            model,
            created_at: now,
            updated_at: now,
            temperature: 0.7,
            top_p: 100,
            top_k: 40,
        };

        db.upsert(session.clone())?;

        Ok(session)
    }

    fn db(&self) -> anyhow::Result<std::sync::MutexGuard<'_, RecordSqlite>> {
        self.db
            .lock()
            .map_err(|_| anyhow::anyhow!("sqlite record storage lock is poisoned"))
    }
}

struct TauriAgentStreamCallback {
    events: Channel<ChatStreamEvent>,
    error_sent: AtomicBool,
    tool_seen: AtomicBool,
}

impl TauriAgentStreamCallback {
    fn new(events: Channel<ChatStreamEvent>) -> Self {
        Self {
            events,
            error_sent: AtomicBool::new(false),
            tool_seen: AtomicBool::new(false),
        }
    }

    fn send(&self, event: ChatStreamEvent) {
        let _ = self.events.send(event);
    }

    fn tool_event_seen(&self) -> bool {
        self.tool_seen.load(Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl AgentStreamCallback for TauriAgentStreamCallback {
    async fn on_token(&self, token: &str) {
        self.send(ChatStreamEvent::Token {
            token: token.to_owned(),
        });
    }

    async fn on_complete(&self, full_text: &str) {
        self.send(ChatStreamEvent::Completed {
            full_text: full_text.to_owned(),
        });
    }

    async fn on_error(&self, error: &anyhow::Error) {
        if !self.error_sent.swap(true, Ordering::Relaxed) {
            self.send(ChatStreamEvent::Error {
                error: format!("{error:#}"),
            });
        }
    }

    async fn on_usage(&self, usage: &CompletionUsage) {
        self.send(ChatStreamEvent::Usage {
            summary: format!("{usage:?}"),
        });
    }

    async fn on_tool_start(&self, tool_name: &str, arguments: &str) {
        self.tool_seen.store(true, Ordering::Relaxed);
        self.send(ChatStreamEvent::ToolStarted {
            tool_name: display_tool_name(tool_name),
            arguments: arguments.to_owned(),
        });
    }

    async fn on_tool_result(&self, tool_name: &str, result: &str) {
        self.tool_seen.store(true, Ordering::Relaxed);
        self.send(ChatStreamEvent::ToolCompleted {
            tool_name: display_tool_name(tool_name),
            result: result.to_owned(),
        });
    }
}

fn read_session(db: &RecordSqlite, session_id: &str) -> anyhow::Result<SessionRecord> {
    db.read::<SessionRecord>(&[RecordFilter::text("session_id", session_id)])?
        .into_iter()
        .next()
        .with_context(|| format!("session could not be found: {session_id}"))
}

fn read_messages(db: &RecordSqlite, session_id: &str) -> anyhow::Result<Vec<MessageRecord>> {
    db.read::<MessageRecord>(&[RecordFilter::text("session_id", session_id)])
}

fn load_provider_config() -> anyhow::Result<LlmProviderConfig> {
    let record_file = RecordFile::open(LLM_PROVIDER_CONFIG_PATH)?;
    record_file.init(&LlmProviderConfig::default_config())?;
    record_file.read()
}

fn chat_request_messages(
    messages: &[MessageRecord],
) -> anyhow::Result<Vec<async_openai::types::ChatCompletionRequestMessage>> {
    let mut request_messages = Vec::new();

    for message in messages {
        if message.content.trim().is_empty() {
            continue;
        }

        match message.role {
            MESSAGE_ROLE_USER => {
                request_messages.push(agent_orchestrator::user_message_request(&message.content)?);
            }
            MESSAGE_ROLE_ASSISTANT => {
                if message.content.trim() == EMPTY_AGENT_RESPONSE {
                    continue;
                }
                request_messages.push(agent_orchestrator::assistant_message_request(
                    &message.content,
                )?);
            }
            MESSAGE_ROLE_SYSTEM | MESSAGE_ROLE_TOOL => {}
            _ => {}
        }
    }

    Ok(request_messages)
}

fn next_sequence(messages: &[MessageRecord]) -> i64 {
    messages
        .iter()
        .map(|message| message.sequence)
        .max()
        .map_or(0, |sequence| sequence + 1)
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn title_from_message(message: &str) -> String {
    let mut title = message.trim().chars().take(48).collect::<String>();

    if title.is_empty() {
        title = String::from("New session");
    }

    title
}

fn display_tool_name(tool_name: impl AsRef<str>) -> String {
    let tool_name = tool_name.as_ref().trim();

    if tool_name.is_empty() {
        String::from("Unknown tool")
    } else {
        tool_name.to_owned()
    }
}
