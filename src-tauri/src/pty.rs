use std::io::{Read, Write};
use std::sync::Mutex;
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter};

/// Owns one running shell behind a PTY pair. The reader half runs on its
/// own OS thread (because portable-pty's Read impl is blocking); writes
/// and resizes go through `&self` methods that lock the appropriate half.
pub struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl PtySession {
    pub fn spawn(app: AppHandle, cols: u16, rows: u16) -> Result<Self, String> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        // CommandBuilder starts with an empty env in portable-pty 0.8, so
        // copy the parent's. Otherwise bash launches without PATH and is
        // unusable.
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        let mut cmd = CommandBuilder::new(shell);
        for (key, val) in std::env::vars() {
            cmd.env(key, val);
        }
        cmd.env("TERM", "xterm-256color");
        if let Ok(home) = std::env::var("HOME") {
            cmd.cwd(home);
        }

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        // After spawn the child owns the slave fd; drop our handle.
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        // Reader thread: drain PTY → forward bytes to the frontend as
        // Tauri events. Real OS thread, not a tokio task, because `read`
        // blocks.
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break, // EOF — child exited
                    Ok(n) => {
                        // M1 limitation: lossy UTF-8 can mangle a multi-byte
                        // sequence split across reads. M2 will swap this for
                        // a small leftover-bytes buffer.
                        let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
                        if app.emit("pty:data", chunk).is_err() {
                            break; // window closed
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            child: Mutex::new(child),
        })
    }

    pub fn write(&self, data: &str) -> Result<(), String> {
        let mut writer = self.writer.lock().map_err(|e| e.to_string())?;
        writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        let master = self.master.lock().map_err(|e| e.to_string())?;
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Quitting the app drops AppState → drops PtySession → kills the
        // shell. Without this, closing master alone doesn't always SIGHUP
        // the child fast enough and you can leak processes.
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}
