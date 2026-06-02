#[path = "../src/log/mod.rs"]
mod log;

#[path = "../src/record/record_file.rs"]
mod record_file;

use record_file::RecordFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TestRecord {
    id: String,
    count: u32,
}

#[test]
fn init_writes_default_value_when_file_is_missing() {
    let path = unique_test_path("init");
    let record_file = RecordFile::open(&path).expect("open record file");
    assert_eq!(record_file.path(), path.as_path());

    let default = TestRecord {
        id: "default".to_owned(),
        count: 1,
    };

    record_file.init(&default).expect("init record file");
    assert_eq!(
        record_file.resolved_path(),
        path.canonicalize().expect("canonical record file path")
    );

    let actual: TestRecord = record_file.read().expect("read record file");
    assert_eq!(actual, default);

    let _ = record_file.delete();
}

#[test]
fn init_keeps_existing_file_content() {
    let path = unique_test_path("existing");
    let record_file = RecordFile::open(&path).expect("open record file");
    let original = TestRecord {
        id: "original".to_owned(),
        count: 1,
    };
    let default = TestRecord {
        id: "default".to_owned(),
        count: 2,
    };

    record_file.write(&original).expect("write original");
    record_file.init(&default).expect("init record file");

    let actual: TestRecord = record_file.read().expect("read record file");
    assert_eq!(actual, original);

    let _ = record_file.delete();
}

#[test]
fn write_replaces_record_file_content() {
    let path = unique_test_path("replace");
    let record_file = RecordFile::open(&path).expect("open record file");
    let first = TestRecord {
        id: "first".to_owned(),
        count: 1,
    };
    let second = TestRecord {
        id: "second".to_owned(),
        count: 2,
    };

    record_file.write(&first).expect("write first");
    record_file.write(&second).expect("write second");

    let actual: TestRecord = record_file.read().expect("read record file");
    assert_eq!(actual, second);

    let _ = record_file.delete();
}

fn unique_test_path(label: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "cowork-rs-record-file-{label}-{}-{timestamp}.json",
        std::process::id()
    ))
}
