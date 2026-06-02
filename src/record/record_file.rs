use anyhow::Context;
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::log::ilog::ILog;

const TAG: &str = "RecordFile";

pub struct RecordFile {
    path: PathBuf,
}

impl RecordFile {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        ILog::d(TAG, &format!("open: path={}", path.as_ref().display()));

        Ok(Self {
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn init<T>(&self, default_value: &T) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        ILog::d(TAG, &format!("init: path={}", self.path.display()));

        if self.exists() {
            ILog::d(TAG, "init: file already exists");
            return Ok(());
        }

        self.write(default_value)?;
        ILog::d(TAG, "init: completed");

        Ok(())
    }

    pub fn read<T>(&self) -> anyhow::Result<T>
    where
        T: DeserializeOwned,
    {
        ILog::d(TAG, &format!("read: path={}", self.path.display()));

        let bytes = self.read_bytes()?;
        let value = deserialize_json(&bytes)?;

        ILog::d(TAG, "read: completed");
        Ok(value)
    }

    pub fn write<T>(&self, value: &T) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        ILog::d(TAG, &format!("write: path={}", self.path.display()));

        ensure_parent_dir(&self.path)?;
        let bytes = serialize_json(value)?;
        self.write_bytes_atomic(&bytes)?;

        ILog::d(TAG, "write: completed");
        Ok(())
    }

    pub fn exists(&self) -> bool {
        ILog::d(TAG, &format!("exists: path={}", self.path.display()));
        self.path.exists()
    }

    pub fn delete(&self) -> anyhow::Result<()> {
        ILog::d(TAG, &format!("delete: path={}", self.path.display()));

        if !self.exists() {
            ILog::d(TAG, "delete: file does not exist");
            return Ok(());
        }

        fs::remove_file(&self.path)
            .with_context(|| format!("failed to delete record file {}", self.path.display()))?;
        ILog::d(TAG, "delete: completed");

        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn resolved_path(&self) -> PathBuf {
        ILog::d(TAG, "resolved_path: resolving record file path");

        let resolved = self
            .path
            .canonicalize()
            .unwrap_or_else(|_| self.path.clone());
        ILog::d(TAG, &format!("resolved_path: path={}", resolved.display()));

        resolved
    }

    fn read_bytes(&self) -> anyhow::Result<Vec<u8>> {
        fs::read(&self.path)
            .with_context(|| format!("failed to read record file {}", self.path.display()))
    }

    fn write_bytes_atomic(&self, bytes: &[u8]) -> anyhow::Result<()> {
        let temp_path = temp_path_for(&self.path)?;

        fs::write(&temp_path, bytes).with_context(|| {
            format!(
                "failed to write temporary record file {}",
                temp_path.display()
            )
        })?;

        if let Err(error) = fs::rename(&temp_path, &self.path) {
            let _ = fs::remove_file(&temp_path);
            return Err(error).with_context(|| {
                format!(
                    "failed to replace record file {} with {}",
                    self.path.display(),
                    temp_path.display()
                )
            });
        }

        Ok(())
    }
}

fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create parent directory for record file {}",
                    path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn serialize_json<T>(value: &T) -> anyhow::Result<Vec<u8>>
where
    T: Serialize,
{
    serde_json::to_vec_pretty(value).context("failed to serialize record file JSON")
}

fn deserialize_json<T>(bytes: &[u8]) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(bytes).context("failed to deserialize record file JSON")
}

fn temp_path_for(path: &Path) -> anyhow::Result<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = path
        .file_name()
        .context("record file path must include a file name")?
        .to_string_lossy();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX_EPOCH")?
        .as_nanos();

    Ok(parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        timestamp
    )))
}
