use std::path::{Path, PathBuf};

use anyhow::bail;
use rusqlite::{Row, ToSql};

use super::sqlite_db::SqliteDb;
use super::{RecordFilter, RecordSchema};
use crate::log::ilog::ILog;

const TAG: &str = "RecordSqlite";

pub trait SqliteRecord: RecordSchema + Sized {
    fn select_all_sql() -> &'static str;
    fn select_filtered_sql(where_clause: &str) -> String;
    fn upsert_sql() -> &'static str;
    fn delete_sql(where_clause: &str) -> String {
        format!("DELETE FROM {} {where_clause}", Self::table_name())
    }
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self>;
    fn upsert_params(&self) -> Vec<&dyn ToSql>;
}

pub struct RecordSqlite {
    db: SqliteDb,
}

impl RecordSqlite {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        ILog::d(TAG, &format!("open: path={}", path.as_ref().display()));

        Ok(Self {
            db: SqliteDb::open(path)?,
        })
    }

    pub fn init<T>(&self) -> anyhow::Result<()>
    where
        T: RecordSchema,
    {
        ILog::d(TAG, &format!("init: table={}", T::table_name()));
        self.db.init_record::<T>()
    }

    pub fn read_all<T>(&self) -> anyhow::Result<Vec<T>>
    where
        T: SqliteRecord,
    {
        ILog::d(TAG, &format!("read_all: table={}", T::table_name()));

        let mut statement = self.db.conn().prepare(T::select_all_sql())?;
        let rows = statement.query_map([], T::from_row)?;
        let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;

        ILog::d(TAG, &format!("read_all: completed count={}", records.len()));
        Ok(records)
    }

    pub fn read<T>(&self, filters: &[RecordFilter]) -> anyhow::Result<Vec<T>>
    where
        T: SqliteRecord,
    {
        ILog::d(
            TAG,
            &format!("read: table={} filters={}", T::table_name(), filters.len()),
        );

        let where_clause = build_where_clause(filters)?;
        let sql = T::select_filtered_sql(&where_clause);
        let params = filter_params(filters);

        let mut statement = self.db.conn().prepare(&sql)?;
        let rows = statement.query_map(params.as_slice(), T::from_row)?;
        let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;

        ILog::d(TAG, &format!("read: completed count={}", records.len()));
        Ok(records)
    }

    pub fn upsert<T>(&self, item: T) -> anyhow::Result<()>
    where
        T: SqliteRecord,
    {
        ILog::d(TAG, &format!("upsert: table={}", T::table_name()));

        let params = item.upsert_params();
        let rows_changed = self.db.conn().execute(T::upsert_sql(), params.as_slice())?;

        ILog::d(
            TAG,
            &format!("upsert: completed rows_changed={rows_changed}"),
        );
        Ok(())
    }

    pub fn delete<T>(&self, filters: &[RecordFilter]) -> anyhow::Result<()>
    where
        T: SqliteRecord,
    {
        ILog::d(
            TAG,
            &format!(
                "delete: table={} filters={}",
                T::table_name(),
                filters.len()
            ),
        );

        let where_clause = build_where_clause(filters)?;
        let sql = T::delete_sql(&where_clause);
        let params = filter_params(filters);
        let rows_changed = self.db.conn().execute(&sql, params.as_slice())?;

        ILog::d(
            TAG,
            &format!("delete: completed rows_changed={rows_changed}"),
        );
        Ok(())
    }

    pub fn resolved_path(&self) -> PathBuf {
        self.db.resolved_path()
    }
}

fn build_where_clause(filters: &[RecordFilter]) -> anyhow::Result<String> {
    ILog::d(
        TAG,
        &format!("build_where_clause: filters={}", filters.len()),
    );

    if filters.is_empty() {
        ILog::d(TAG, "build_where_clause: rejected empty filters");
        bail!("record filters cannot be empty");
    }

    let predicates = filters
        .iter()
        .enumerate()
        .map(|(index, filter)| format!("{} = ?{}", filter.column, index + 1))
        .collect::<Vec<_>>()
        .join(" AND ");

    let where_clause = format!("WHERE {predicates}");
    ILog::d(TAG, &format!("build_where_clause: clause={where_clause}"));

    Ok(where_clause)
}

fn filter_params(filters: &[RecordFilter]) -> Vec<&dyn ToSql> {
    ILog::d(TAG, &format!("filter_params: filters={}", filters.len()));
    filters
        .iter()
        .map(|filter| &filter.value as &dyn ToSql)
        .collect()
}
