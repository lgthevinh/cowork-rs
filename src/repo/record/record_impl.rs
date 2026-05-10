use crate::repo::RecordSchema;

pub const MESSAGE_ROLE_SYSTEM: u16 = 0;
pub const MESSAGE_ROLE_ASSISTANT: u16 = 1;
pub const MESSAGE_ROLE_USER: u16 = 2;
pub const MESSAGE_ROLE_TOOL: u16 = 3;

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
        "sessions"
    }

    fn create_table_sql() -> &'static str {
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

impl RecordSchema for MessageRecord {
    fn table_name() -> &'static str {
        "messages"
    }

    fn create_table_sql() -> &'static str {
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
