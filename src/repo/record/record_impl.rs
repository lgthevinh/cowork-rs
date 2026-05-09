use crate::repo::RecordSchema;

pub struct SessionRecord {
    pub id: String,
    pub name: String,
}

impl RecordSchema for SessionRecord {
    fn table_name() -> &'static str {
        "sessions"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        );
        "#
    }
}
