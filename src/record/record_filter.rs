use crate::log::ilog::ILog;
use rusqlite::types::Value;

const TAG: &str = "RecordFilter";

#[derive(Debug, Clone, PartialEq)]
pub struct RecordFilter {
    pub column: &'static str,
    pub value: Value,
}

impl RecordFilter {
    pub fn text(column: &'static str, value: impl Into<String>) -> Self {
        let value = value.into();
        ILog::d(
            TAG,
            &format!("text: column={column} value_len={}", value.len()),
        );

        Self {
            column,
            value: Value::Text(value),
        }
    }

    pub fn integer(column: &'static str, value: i64) -> Self {
        ILog::d(TAG, &format!("integer: column={column} value={value}"));
        Self {
            column,
            value: Value::Integer(value),
        }
    }

    pub fn real(column: &'static str, value: f64) -> Self {
        ILog::d(TAG, &format!("real: column={column} value={value}"));
        Self {
            column,
            value: Value::Real(value),
        }
    }

    pub fn null(column: &'static str) -> Self {
        ILog::d(TAG, &format!("null: column={column}"));
        Self {
            column,
            value: Value::Null,
        }
    }
}
