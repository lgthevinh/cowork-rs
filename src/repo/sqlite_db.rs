use super::RecordSchema;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub struct SqliteDb {
    conn: Connection,
    path: PathBuf,
}

impl SqliteDb {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(Self { conn, path })
    }

    pub fn init_record<T>(&self) -> anyhow::Result<()>
    where
        T: RecordSchema,
    {
        self.conn.execute_batch(T::create_table_sql())?;
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn resolved_path(&self) -> PathBuf {
        let path = self
            .conn
            .path()
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.path.clone());

        path.canonicalize().unwrap_or(path)
    }
}
