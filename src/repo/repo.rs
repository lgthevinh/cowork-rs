use super::repo_filter::RepoFilter;
use anyhow::bail;

pub trait Repo<T> {
    fn read_all(&self) -> anyhow::Result<Vec<T>>;
    fn read(&self, filters: &[RepoFilter]) -> anyhow::Result<Vec<T>>;
    fn upsert(&self, item: T) -> anyhow::Result<()>;
    fn delete(&self, filters: &[RepoFilter]) -> anyhow::Result<()>;
}

pub fn build_where_clause(filters: &[RepoFilter]) -> anyhow::Result<String> {
    if filters.is_empty() {
        bail!("repo filters cannot be empty");
    }

    let predicates = filters
        .iter()
        .enumerate()
        .map(|(index, filter)| format!("{} = ?{}", filter.column, index + 1))
        .collect::<Vec<_>>()
        .join(" AND ");

    Ok(format!("WHERE {predicates}"))
}
