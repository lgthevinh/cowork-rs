use crate::log::ilog::ILog;
use crate::record::RecordSchema;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

const TAG: &str = "SqliteDb";

pub struct SqliteDb {
    conn: Connection,
    path: PathBuf,
}

impl SqliteDb {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        ILog::d(TAG, &format!("open: path={}", path.as_ref().display()));
        let path = path.as_ref().to_path_buf();
        let conn = Connection::open(&path)?;
        ILog::d(TAG, "open: connection opened");

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        ILog::d(TAG, "open: foreign_keys pragma enabled");

        Ok(Self { conn, path })
    }

    pub fn init_record<T>(&self) -> anyhow::Result<()>
    where
        T: RecordSchema,
    {
        ILog::d(TAG, &format!("init_record: table={}", T::table_name()));
        self.conn.execute_batch(T::create_table_sql())?;
        ILog::d(
            TAG,
            &format!("init_record: completed table={}", T::table_name()),
        );
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        ILog::d(TAG, "conn: returning shared connection");
        &self.conn
    }

    pub fn resolved_path(&self) -> PathBuf {
        ILog::d(TAG, "resolved_path: resolving database path");
        let path = self
            .conn
            .path()
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.path.clone());

        let resolved = path.canonicalize().unwrap_or(path);
        ILog::d(TAG, &format!("resolved_path: path={}", resolved.display()));
        resolved
    }
}
