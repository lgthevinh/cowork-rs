use super::RecordSchema;
use rusqlite::Connection;
use std::path::Path;

pub struct SqliteDb {
    conn: Connection,
}

impl SqliteDb {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn init_record<T>(&self) -> anyhow::Result<()>
    where
        T: RecordSchema,
    {
        self.conn.execute_batch(T::create_table_sql())?;
        Ok(())
    }

    pub fn init_records(&self, create_table_sql: &[&str]) -> anyhow::Result<()> {
        for sql in create_table_sql {
            self.conn.execute_batch(sql)?;
        }

        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
