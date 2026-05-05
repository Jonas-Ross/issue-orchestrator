//! Per-repo provider tokens, stored in the macOS Keychain.
//!
//! Service is the constant `SERVICE`; account is `<kind>:<repo_name>`,
//! e.g. `"jira:alpha"`. Tokens are read in-process by the issue
//! provider factory and never leave the Rust side over IPC except via
//! the explicit `set_provider_secret` setter.

use crate::error::{Error, Result};

#[cfg(target_os = "macos")]
const SERVICE: &str = "issue-orchestrator";

/// The Keychain account string for `(kind, repo_name)`. Public so the
/// IPC commands and tests can format it identically.
pub fn account(kind: &str, repo_name: &str) -> String {
    format!("{kind}:{repo_name}")
}

#[cfg(target_os = "macos")]
fn entry(kind: &str, repo_name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, &account(kind, repo_name))
        .map_err(|e| Error::Config(format!("keychain: {e}")))
}

#[cfg(target_os = "macos")]
pub fn get_token(kind: &str, repo_name: &str) -> Result<String> {
    let e = entry(kind, repo_name)?;
    e.get_password().map_err(|err| match err {
        keyring::Error::NoEntry => Error::Config(format!(
            "no token in Keychain for {}",
            account(kind, repo_name)
        )),
        other => Error::Config(format!("keychain read: {other}")),
    })
}

#[cfg(target_os = "macos")]
pub fn set_token(kind: &str, repo_name: &str, token: &str) -> Result<()> {
    let e = entry(kind, repo_name)?;
    e.set_password(token)
        .map_err(|err| Error::Config(format!("keychain write: {err}")))
}

#[cfg(target_os = "macos")]
pub fn delete_token(kind: &str, repo_name: &str) -> Result<()> {
    let e = entry(kind, repo_name)?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(Error::Config(format!("keychain delete: {err}"))),
    }
}

#[cfg(target_os = "macos")]
pub fn token_exists(kind: &str, repo_name: &str) -> Result<bool> {
    let e = entry(kind, repo_name)?;
    match e.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(err) => Err(Error::Config(format!("keychain read: {err}"))),
    }
}

// Non-macOS stubs so the project still builds (and the rest of the test
// suite still runs) under Linux dev. The app only ships on macOS.
#[cfg(not(target_os = "macos"))]
pub fn get_token(_kind: &str, _repo_name: &str) -> Result<String> {
    Err(Error::Config("Keychain only available on macOS".into()))
}

#[cfg(not(target_os = "macos"))]
pub fn set_token(_kind: &str, _repo_name: &str, _token: &str) -> Result<()> {
    Err(Error::Config("Keychain only available on macOS".into()))
}

#[cfg(not(target_os = "macos"))]
pub fn delete_token(_kind: &str, _repo_name: &str) -> Result<()> {
    Err(Error::Config("Keychain only available on macOS".into()))
}

#[cfg(not(target_os = "macos"))]
pub fn token_exists(_kind: &str, _repo_name: &str) -> Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_format() {
        assert_eq!(account("jira", "alpha"), "jira:alpha");
        assert_eq!(account("linear", "my repo"), "linear:my repo");
    }
}
