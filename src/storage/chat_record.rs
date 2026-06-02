use crate::log::ilog::ILog;
use crate::record::{RecordSchema, SqliteRecord};
use rusqlite::{Row, ToSql};
use serde::{Deserialize, Serialize};

const TAG: &str = "RecordSchema";

pub const MESSAGE_ROLE_SYSTEM: u16 = 0;
pub const MESSAGE_ROLE_ASSISTANT: u16 = 1;
pub const MESSAGE_ROLE_USER: u16 = 2;
pub const MESSAGE_ROLE_TOOL: u16 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub title: String,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub temperature: f64,
    pub top_p: i16,
    pub top_k: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub message_id: String,
    pub session_id: String,
    pub sequence: i64,
    pub role: u16,
    pub content: String,
    pub created_at: i64,
}

impl RecordSchema for SessionRecord {
    fn table_name() -> &'static str {
        ILog::d(TAG, "SessionRecord::table_name: sessions");
        "sessions"
    }

    fn create_table_sql() -> &'static str {
        ILog::d(TAG, "SessionRecord::create_table_sql: building schema SQL");
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            model TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            temperature REAL NOT NULL,
            top_p INTEGER NOT NULL,
            top_k INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_updated_at
            ON sessions(updated_at);
        "#
    }
}

impl SqliteRecord for SessionRecord {
    fn select_all_sql() -> &'static str {
        r#"
        SELECT
            session_id,
            title,
            model,
            created_at,
            updated_at,
            temperature,
            top_p,
            top_k
        FROM sessions
        ORDER BY updated_at DESC
        "#
    }

    fn select_filtered_sql(where_clause: &str) -> String {
        format!(
            r#"
            SELECT
                session_id,
                title,
                model,
                created_at,
                updated_at,
                temperature,
                top_p,
                top_k
            FROM sessions
            {where_clause}
            ORDER BY updated_at DESC
            "#
        )
    }

    fn upsert_sql() -> &'static str {
        r#"
        INSERT INTO sessions (
            session_id,
            title,
            model,
            created_at,
            updated_at,
            temperature,
            top_p,
            top_k
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(session_id) DO UPDATE SET
            title = excluded.title,
            model = excluded.model,
            created_at = excluded.created_at,
            updated_at = excluded.updated_at,
            temperature = excluded.temperature,
            top_p = excluded.top_p,
            top_k = excluded.top_k
        "#
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        ILog::d(TAG, "SessionRecord::from_row: mapping row");
        Ok(Self {
            session_id: row.get("session_id")?,
            title: row.get("title")?,
            model: row.get("model")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            temperature: row.get("temperature")?,
            top_p: row.get("top_p")?,
            top_k: row.get("top_k")?,
        })
    }

    fn upsert_params(&self) -> Vec<&dyn ToSql> {
        vec![
            &self.session_id,
            &self.title,
            &self.model,
            &self.created_at,
            &self.updated_at,
            &self.temperature,
            &self.top_p,
            &self.top_k,
        ]
    }
}

impl RecordSchema for MessageRecord {
    fn table_name() -> &'static str {
        ILog::d(TAG, "MessageRecord::table_name: messages");
        "messages"
    }

    fn create_table_sql() -> &'static str {
        ILog::d(TAG, "MessageRecord::create_table_sql: building schema SQL");
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            message_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            role INTEGER NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (session_id)
                REFERENCES sessions(session_id)
                ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_sequence
            ON messages(session_id, sequence);
        "#
    }
}

impl SqliteRecord for MessageRecord {
    fn select_all_sql() -> &'static str {
        r#"
        SELECT
            message_id,
            session_id,
            sequence,
            role,
            content,
            created_at
        FROM messages
        ORDER BY session_id, sequence
        "#
    }

    fn select_filtered_sql(where_clause: &str) -> String {
        format!(
            r#"
            SELECT
                message_id,
                session_id,
                sequence,
                role,
                content,
                created_at
            FROM messages
            {where_clause}
            ORDER BY sequence
            "#
        )
    }

    fn upsert_sql() -> &'static str {
        r#"
        INSERT INTO messages (
            message_id,
            session_id,
            sequence,
            role,
            content,
            created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(message_id) DO UPDATE SET
            session_id = excluded.session_id,
            sequence = excluded.sequence,
            role = excluded.role,
            content = excluded.content,
            created_at = excluded.created_at
        "#
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        ILog::d(TAG, "MessageRecord::from_row: mapping row");
        Ok(Self {
            message_id: row.get("message_id")?,
            session_id: row.get("session_id")?,
            sequence: row.get("sequence")?,
            role: row.get("role")?,
            content: row.get("content")?,
            created_at: row.get("created_at")?,
        })
    }

    fn upsert_params(&self) -> Vec<&dyn ToSql> {
        vec![
            &self.message_id,
            &self.session_id,
            &self.sequence,
            &self.role,
            &self.content,
            &self.created_at,
        ]
    }
}
