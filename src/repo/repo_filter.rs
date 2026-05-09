use rusqlite::types::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct RepoFilter {
    pub column: &'static str,
    pub value: Value,
}

impl RepoFilter {
    pub fn text(column: &'static str, value: impl Into<String>) -> Self {
        Self {
            column,
            value: Value::Text(value.into()),
        }
    }

    pub fn integer(column: &'static str, value: i64) -> Self {
        Self {
            column,
            value: Value::Integer(value),
        }
    }

    pub fn real(column: &'static str, value: f64) -> Self {
        Self {
            column,
            value: Value::Real(value),
        }
    }

    pub fn null(column: &'static str) -> Self {
        Self {
            column,
            value: Value::Null,
        }
    }
}
