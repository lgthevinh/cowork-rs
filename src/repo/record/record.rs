pub trait RecordSchema {
    fn table_name() -> &'static str;
    fn create_table_sql() -> &'static str;
}
