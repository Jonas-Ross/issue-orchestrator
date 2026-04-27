use std::io::{Read, Write};
use std::sync::Mutex;
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc;

use crate::error::{Error, Result};

/// Events streamed by the per-PTY reader thread.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    Data(String),
    Eof,
}

/// Owns one running shell behind a PTY pair. Stateless w.r.t. session
/// identity — that lives in the registry. Dropping the handles kills
/// the child via `Drop`.
pub struct PtyHandles {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl PtyHandles {
    pub fn write(&self, data: &str) -> Result<()> {
        let mut w = self
            .writer
            .lock()
            .map_err(|e| Error::Pty(format!("writer lock: {e}")))?;
        w.write_all(data.as_bytes())
            .map_err(|e| Error::Pty(e.to_string()))?;
        w.flush().map_err(|e| Error::Pty(e.to_string()))?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        let m = self
            .master
            .lock()
            .map_err(|e| Error::Pty(format!("master lock: {e}")))?;
        m.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| Error::Pty(e.to_string()))?;
        Ok(())
    }
}

impl Drop for PtyHandles {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

/// Spawn a child process attached to a fresh PTY pair. The reader thread
/// drains the master and emits `PtyEvent::Data(String)` on `tx`; on EOF
/// or read error it emits `PtyEvent::Eof` and exits.
///
/// UTF-8 sequences split across read boundaries are held in a leftover
/// buffer until the next read completes them, so the frontend never sees
/// a truncated multibyte glyph.
pub fn spawn_pty(
    cmd: CommandBuilder,
    cols: u16,
    rows: u16,
    tx: mpsc::Sender<PtyEvent>,
) -> Result<PtyHandles> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| Error::Pty(e.to_string()))?;

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| Error::Pty(e.to_string()))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| Error::Pty(e.to_string()))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| Error::Pty(e.to_string()))?;

    thread::spawn(move || {
        let mut leftover: Vec<u8> = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = tx.blocking_send(PtyEvent::Eof);
                    break;
                }
                Ok(n) => {
                    let mut combined = std::mem::take(&mut leftover);
                    combined.extend_from_slice(&buf[..n]);
                    let valid_up_to = match std::str::from_utf8(&combined) {
                        Ok(_) => combined.len(),
                        Err(e) => e.valid_up_to(),
                    };
                    if valid_up_to > 0 {
                        // SAFETY: validated as UTF-8 above.
                        let s = unsafe {
                            std::str::from_utf8_unchecked(&combined[..valid_up_to])
                        }
                        .to_owned();
                        if tx.blocking_send(PtyEvent::Data(s)).is_err() {
                            break;
                        }
                    }
                    leftover = combined[valid_up_to..].to_vec();
                }
                Err(_) => {
                    let _ = tx.blocking_send(PtyEvent::Eof);
                    break;
                }
            }
        }
    });

    Ok(PtyHandles {
        master: Mutex::new(pair.master),
        writer: Mutex::new(writer),
        child: Mutex::new(child),
    })
}
