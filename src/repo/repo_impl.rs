use rusqlite::{Connection, Row, ToSql, params};

use super::repo::{Repo, build_where_clause};
use super::repo_filter::RepoFilter;
use super::sqlite_db::SqliteDb;
use crate::repo::record::record_impl::{MessageRecord, SessionRecord};

pub struct SessionRepo<'a> {
    conn: &'a Connection,
}

impl<'a> SessionRepo<'a> {
    pub fn new(db: &'a SqliteDb) -> Self {
        Self { conn: db.conn() }
    }

    pub fn from_conn(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

pub struct MessageRepo<'a> {
    conn: &'a Connection,
}

impl<'a> MessageRepo<'a> {
    pub fn new(db: &'a SqliteDb) -> Self {
        Self { conn: db.conn() }
    }

    pub fn from_conn(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl Repo<SessionRecord> for SessionRepo<'_> {
    fn read_all(&self) -> anyhow::Result<Vec<SessionRecord>> {
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

        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn read(&self, filters: &[RepoFilter]) -> anyhow::Result<Vec<SessionRecord>> {
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

        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn upsert(&self, item: SessionRecord) -> anyhow::Result<()> {
        self.conn.execute(
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

        Ok(())
    }

    fn delete(&self, filters: &[RepoFilter]) -> anyhow::Result<()> {
        let where_clause = build_where_clause(filters)?;
        let sql = format!("DELETE FROM sessions {where_clause}");
        let params = filter_params(filters);

        self.conn.execute(&sql, params.as_slice())?;

        Ok(())
    }
}

impl Repo<MessageRecord> for MessageRepo<'_> {
    fn read_all(&self) -> anyhow::Result<Vec<MessageRecord>> {
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

        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn read(&self, filters: &[RepoFilter]) -> anyhow::Result<Vec<MessageRecord>> {
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

        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn upsert(&self, item: MessageRecord) -> anyhow::Result<()> {
        self.conn.execute(
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

        Ok(())
    }

    fn delete(&self, filters: &[RepoFilter]) -> anyhow::Result<()> {
        let where_clause = build_where_clause(filters)?;
        let sql = format!("DELETE FROM messages {where_clause}");
        let params = filter_params(filters);

        self.conn.execute(&sql, params.as_slice())?;

        Ok(())
    }
}

fn filter_params(filters: &[RepoFilter]) -> Vec<&dyn ToSql> {
    filters
        .iter()
        .map(|filter| &filter.value as &dyn ToSql)
        .collect()
}

fn session_from_row(row: &Row<'_>) -> rusqlite::Result<SessionRecord> {
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
    Ok(MessageRecord {
        message_id: row.get("message_id")?,
        session_id: row.get("session_id")?,
        sequence: row.get("sequence")?,
        role: row.get("role")?,
        content: row.get("content")?,
        created_at: row.get("created_at")?,
    })
}
