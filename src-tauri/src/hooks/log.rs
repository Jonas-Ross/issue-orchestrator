use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::error::{Error, Result};

/// Append-only JSONL audit log. Each line is the original hook payload
/// verbatim, so the schema can drift over time and we still have the
/// raw record.
#[derive(Clone)]
pub struct Logger {
    inner: Arc<Mutex<File>>,
}

impl Logger {
    pub async fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;
        Ok(Self {
            inner: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn append(&self, raw: &Value) -> Result<()> {
        let mut bytes = serde_json::to_vec(raw).map_err(|e| Error::Hooks(e.to_string()))?;
        bytes.push(b'\n');
        let mut f = self.inner.lock().await;
        f.write_all(&bytes).await?;
        // Required: tokio::fs::File queues writes onto a background blocking
        // thread; without flush(), a concurrent read of the same file can
        // race ahead of pending queued writes (manifests as missing lines
        // in hooks::tests::hook_routes_status_change_to_session under CI).
        f.flush().await?;
        Ok(())
    }
}
