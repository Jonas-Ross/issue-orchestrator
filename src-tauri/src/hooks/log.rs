use std::path::Path;
use std::sync::Arc;

use serde_json::Value;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::error::{Error, Result};

/// Append-only JSONL audit log. Each line is the original hook payload as
/// received over the socket (so the schema can drift over time and we
/// still have the raw record). `Logger` is `Clone` so each accept-loop
/// task can hold its own handle; mutation is serialized via the inner
/// async Mutex so we don't block the runtime under hook bursts.
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
        f.flush().await?;
        Ok(())
    }
}
