use super::record_filter::RecordFilter;
use crate::log::ilog::ILog;
use anyhow::bail;

const TAG: &str = "RecordOperation";

pub trait RecordOperation<T> {
    fn read_all(&self) -> anyhow::Result<Vec<T>>;
    fn read(&self, filters: &[RecordFilter]) -> anyhow::Result<Vec<T>>;
    fn upsert(&self, item: T) -> anyhow::Result<()>;
    fn delete(&self, filters: &[RecordFilter]) -> anyhow::Result<()>;
}

pub fn build_where_clause(filters: &[RecordFilter]) -> anyhow::Result<String> {
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
