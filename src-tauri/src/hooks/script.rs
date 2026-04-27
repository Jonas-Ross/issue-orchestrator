use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::error::Result;

const TEMPLATE: &str = include_str!("hook.sh.template");

/// Idempotently write `hook.sh` to disk and chmod +x. Safe to call on
/// every app start; the contents come from a const so Rust always
/// publishes the version it expects.
pub fn ensure_hook_script(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, TEMPLATE)?;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}
