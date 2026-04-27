use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::error::{Error, Result};

/// Append-only JSONL audit log. Each line is the original hook payload as
/// received over the socket (so the schema can drift over time and we
/// still have the raw record). `Logger` is `Clone` so each accept-loop
/// task can hold its own handle; mutation is serialized via the inner
/// Mutex.
#[derive(Clone)]
pub struct Logger {
    inner: Arc<Mutex<File>>,
}

impl Logger {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(file)),
        })
    }

    pub fn append(&self, raw: &Value) -> Result<()> {
        let mut f = self
            .inner
            .lock()
            .map_err(|e| Error::Hooks(format!("log lock: {e}")))?;
        serde_json::to_writer(&mut *f, raw).map_err(|e| Error::Hooks(e.to_string()))?;
        f.write_all(b"\n")?;
        f.flush()?;
        Ok(())
    }
}
