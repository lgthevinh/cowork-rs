use super::repo_filter::RepoFilter;
use crate::log::ilog::ILog;
use anyhow::bail;

const TAG: &str = "Repo";

pub trait Repo<T> {
    fn read_all(&self) -> anyhow::Result<Vec<T>>;
    fn read(&self, filters: &[RepoFilter]) -> anyhow::Result<Vec<T>>;
    fn upsert(&self, item: T) -> anyhow::Result<()>;
    fn delete(&self, filters: &[RepoFilter]) -> anyhow::Result<()>;
}

pub fn build_where_clause(filters: &[RepoFilter]) -> anyhow::Result<String> {
    ILog::d(
        TAG,
        &format!("build_where_clause: filters={}", filters.len()),
    );

    if filters.is_empty() {
        ILog::d(TAG, "build_where_clause: rejected empty filters");
        bail!("repo filters cannot be empty");
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
