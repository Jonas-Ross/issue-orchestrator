use crate::issues::secrets::{self, ProviderSecretKind};

/// Write a provider token (Jira/Linear API key) into the macOS Keychain.
/// Tokens are NEVER returned by any other IPC; this is the only path
/// from the renderer to the credential store.
#[tauri::command]
#[specta::specta]
pub fn set_provider_secret(
    repo_name: String,
    kind: ProviderSecretKind,
    token: String,
) -> Result<(), String> {
    secrets::set_token(kind.as_str(), &repo_name, &token).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn delete_provider_secret(
    repo_name: String,
    kind: ProviderSecretKind,
) -> Result<(), String> {
    secrets::delete_token(kind.as_str(), &repo_name).map_err(|e| e.to_string())
}

/// Read-only check the settings UI uses to render "✓ Token saved" vs
/// "Set token…". Never returns the token itself.
#[tauri::command]
#[specta::specta]
pub fn provider_secret_exists(
    repo_name: String,
    kind: ProviderSecretKind,
) -> Result<bool, String> {
    secrets::token_exists(kind.as_str(), &repo_name).map_err(|e| e.to_string())
}
