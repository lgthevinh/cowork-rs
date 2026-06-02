use rusqlite::{Connection, Row, ToSql, params};

use crate::log::ilog::ILog;
use crate::record::{
    RecordFilter, RecordOperation, SqliteDb, record_operation::build_where_clause,
};
use crate::storage::chat_record::{MessageRecord, SessionRecord};

const SESSION_TAG: &str = "SessionRecordOperation";
const MESSAGE_TAG: &str = "MessageRecordOperation";
const TAG: &str = "ChatRecordOperation";

pub struct SessionRecordOperation<'a> {
    conn: &'a Connection,
}

impl<'a> SessionRecordOperation<'a> {
    pub fn new(db: &'a SqliteDb) -> Self {
        ILog::d(SESSION_TAG, "new: creating record operation from SqliteDb");
        Self { conn: db.conn() }
    }

    pub fn from_conn(conn: &'a Connection) -> Self {
        ILog::d(
            SESSION_TAG,
            "from_conn: creating record operation from connection",
        );
        Self { conn }
    }
}

pub struct MessageRecordOperation<'a> {
    conn: &'a Connection,
}

impl<'a> MessageRecordOperation<'a> {
    pub fn new(db: &'a SqliteDb) -> Self {
        ILog::d(MESSAGE_TAG, "new: creating record operation from SqliteDb");
        Self { conn: db.conn() }
    }

    pub fn from_conn(conn: &'a Connection) -> Self {
        ILog::d(
            MESSAGE_TAG,
            "from_conn: creating record operation from connection",
        );
        Self { conn }
    }
}

impl RecordOperation<SessionRecord> for SessionRecordOperation<'_> {
    fn read_all(&self) -> anyhow::Result<Vec<SessionRecord>> {
        ILog::d(SESSION_TAG, "read_all: start");
        let mut statement = self.conn.prepare(
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
            "#,
        )?;

        let rows = statement.query_map([], session_from_row)?;

        let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        ILog::d(
            SESSION_TAG,
            &format!("read_all: completed count={}", records.len()),
        );

        Ok(records)
    }

    fn read(&self, filters: &[RecordFilter]) -> anyhow::Result<Vec<SessionRecord>> {
        ILog::d(
            SESSION_TAG,
            &format!("read: start filters={}", filters.len()),
        );
        let where_clause = build_where_clause(filters)?;
        let sql = format!(
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
        );

        let mut statement = self.conn.prepare(&sql)?;
        let params = filter_params(filters);
        let rows = statement.query_map(params.as_slice(), session_from_row)?;

        let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        ILog::d(
            SESSION_TAG,
            &format!("read: completed count={}", records.len()),
        );

        Ok(records)
    }

    fn upsert(&self, item: SessionRecord) -> anyhow::Result<()> {
        ILog::d(
            SESSION_TAG,
            &format!(
                "upsert: session_id={} title_len={} model={}",
                item.session_id,
                item.title.len(),
                item.model
            ),
        );

        let rows_changed = self.conn.execute(
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
            "#,
            params![
                item.session_id,
                item.title,
                item.model,
                item.created_at,
                item.updated_at,
                item.temperature,
                item.top_p,
                item.top_k,
            ],
        )?;
        ILog::d(
            SESSION_TAG,
            &format!(
                "upsert: completed session_id={} rows_changed={rows_changed}",
                item.session_id
            ),
        );

        Ok(())
    }

    fn delete(&self, filters: &[RecordFilter]) -> anyhow::Result<()> {
        ILog::d(
            SESSION_TAG,
            &format!("delete: start filters={}", filters.len()),
        );
        let where_clause = build_where_clause(filters)?;
        let sql = format!("DELETE FROM sessions {where_clause}");
        let params = filter_params(filters);

        let rows_changed = self.conn.execute(&sql, params.as_slice())?;
        ILog::d(
            SESSION_TAG,
            &format!("delete: completed rows_changed={rows_changed}"),
        );

        Ok(())
    }
}

impl RecordOperation<MessageRecord> for MessageRecordOperation<'_> {
    fn read_all(&self) -> anyhow::Result<Vec<MessageRecord>> {
        ILog::d(MESSAGE_TAG, "read_all: start");
        let mut statement = self.conn.prepare(
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
            "#,
        )?;

        let rows = statement.query_map([], message_from_row)?;

        let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        ILog::d(
            MESSAGE_TAG,
            &format!("read_all: completed count={}", records.len()),
        );

        Ok(records)
    }

    fn read(&self, filters: &[RecordFilter]) -> anyhow::Result<Vec<MessageRecord>> {
        ILog::d(
            MESSAGE_TAG,
            &format!("read: start filters={}", filters.len()),
        );
        let where_clause = build_where_clause(filters)?;
        let sql = format!(
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
        );

        let mut statement = self.conn.prepare(&sql)?;
        let params = filter_params(filters);
        let rows = statement.query_map(params.as_slice(), message_from_row)?;

        let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        ILog::d(
            MESSAGE_TAG,
            &format!("read: completed count={}", records.len()),
        );

        Ok(records)
    }

    fn upsert(&self, item: MessageRecord) -> anyhow::Result<()> {
        ILog::d(
            MESSAGE_TAG,
            &format!(
                "upsert: message_id={} session_id={} sequence={} role={} content_len={}",
                item.message_id,
                item.session_id,
                item.sequence,
                item.role,
                item.content.len()
            ),
        );

        let rows_changed = self.conn.execute(
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
            "#,
            params![
                item.message_id,
                item.session_id,
                item.sequence,
                item.role,
                item.content,
                item.created_at,
            ],
        )?;
        ILog::d(
            MESSAGE_TAG,
            &format!(
                "upsert: completed message_id={} rows_changed={rows_changed}",
                item.message_id
            ),
        );

        Ok(())
    }

    fn delete(&self, filters: &[RecordFilter]) -> anyhow::Result<()> {
        ILog::d(
            MESSAGE_TAG,
            &format!("delete: start filters={}", filters.len()),
        );
        let where_clause = build_where_clause(filters)?;
        let sql = format!("DELETE FROM messages {where_clause}");
        let params = filter_params(filters);

        let rows_changed = self.conn.execute(&sql, params.as_slice())?;
        ILog::d(
            MESSAGE_TAG,
            &format!("delete: completed rows_changed={rows_changed}"),
        );

        Ok(())
    }
}

fn filter_params(filters: &[RecordFilter]) -> Vec<&dyn ToSql> {
    ILog::d(TAG, &format!("filter_params: filters={}", filters.len()));
    filters
        .iter()
        .map(|filter| &filter.value as &dyn ToSql)
        .collect()
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<SessionRecord> {
    ILog::d(TAG, "session_from_row: mapping row");
    Ok(SessionRecord {
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

fn message_from_row(row: &Row<'_>) -> rusqlite::Result<MessageRecord> {
    ILog::d(TAG, "message_from_row: mapping row");
    Ok(MessageRecord {
        message_id: row.get("message_id")?,
        session_id: row.get("session_id")?,
        sequence: row.get("sequence")?,
        role: row.get("role")?,
        content: row.get("content")?,
        created_at: row.get("created_at")?,
    })
}
