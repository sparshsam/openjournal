/// Secure credential storage using the OS credential manager.
///
/// Platform backends:
/// - Windows: Credential Manager
/// - macOS: Keychain
/// - Linux: Secret Service (libsecret)
///
/// Keys are never stored in OpenJournal's SQLite database.
/// Priority order: env var → credential store → session override.

use keyring::{Entry, Error as KeyringError};

const SERVICE_NAME: &str = "OpenJournal";

/// Credential store key names for different providers.
pub enum CredentialKey {
    DeepSeek,
    OpenAiCompatible,
}

impl CredentialKey {
    fn as_str(&self) -> &'static str {
        match self {
            CredentialKey::DeepSeek => "deepseek_api_key",
            CredentialKey::OpenAiCompatible => "openai_compatible_api_key",
        }
    }
}

/// Save a credential to the OS credential manager.
pub fn save_credential(key: &CredentialKey, value: &str) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE_NAME, key.as_str())?;
    entry.set_password(value)?;
    Ok(())
}

/// Load a credential from the OS credential manager.
/// Returns `None` if no credential exists.
pub fn load_credential(key: &CredentialKey) -> anyhow::Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, key.as_str())?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Delete a credential from the OS credential manager.
pub fn delete_credential(key: &CredentialKey) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE_NAME, key.as_str())?;
    entry.delete_credential()?;
    Ok(())
}

/// Resolve an API key using the priority chain:
/// 1. `OPENJOURNAL_DEEPSEEK_API_KEY` env var
/// 2. `DEEPSEEK_API_KEY` env var
/// 3. OS credential store
/// 4. Provided session override (if non-empty)
///
/// Returns (resolved_key, source_label) where source_label is one of:
/// "env", "credential", "session"
pub fn resolve_api_key(session_override: &str) -> (String, &'static str) {
    // 1. Env var priority 1
    if let Ok(key) = std::env::var("OPENJOURNAL_DEEPSEEK_API_KEY") {
        if !key.is_empty() {
            return (key, "env");
        }
    }
    // 2. Env var priority 2
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        if !key.is_empty() {
            return (key, "env");
        }
    }
    // 3. OS credential store
    if let Ok(Some(key)) = load_credential(&CredentialKey::DeepSeek) {
        if !key.is_empty() {
            return (key, "credential");
        }
    }
    // 4. Session override
    if !session_override.is_empty() {
        return (session_override.to_string(), "session");
    }
    (String::new(), "missing")
}

/// Mask an API key for display: `sk-••••••••abcd`
pub fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    let len = key.len();
    if len > 8 {
        format!("sk-••••••••{}", &key[len - 4..])
    } else if len > 4 {
        format!("{}••••", &key[..2])
    } else {
        "••••".to_string()
    }
}

/// Status information about the API key source for the frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyStatus {
    pub source: String,          // "env" | "credential" | "session" | "missing"
    pub masked_key: String,      // "sk-••••••••abcd" or empty
    pub has_env_var: bool,
    pub has_credential: bool,
}

/// Get the current API key status (without exposing the key).
pub fn get_api_key_status(session_override: &str) -> ApiKeyStatus {
    let has_oj_env = std::env::var("OPENJOURNAL_DEEPSEEK_API_KEY")
        .ok()
        .map_or(false, |v| !v.is_empty());
    let has_ds_env = std::env::var("DEEPSEEK_API_KEY")
        .ok()
        .map_or(false, |v| !v.is_empty());
    let has_env_var = has_oj_env || has_ds_env;
    let has_credential = load_credential(&CredentialKey::DeepSeek)
        .ok()
        .flatten()
        .map_or(false, |v| !v.is_empty());

    let (resolved_key, source) = resolve_api_key(session_override);
    let masked_key = mask_api_key(&resolved_key);

    ApiKeyStatus {
        source: if resolved_key.is_empty() { "missing" } else { source }.to_string(),
        masked_key,
        has_env_var,
        has_credential,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to run a test with no env vars set.
    fn with_clean_env<F: FnOnce()>(f: F) {
        let oj = std::env::var_os("OPENJOURNAL_DEEPSEEK_API_KEY");
        let ds = std::env::var_os("DEEPSEEK_API_KEY");
        std::env::remove_var("OPENJOURNAL_DEEPSEEK_API_KEY");
        std::env::remove_var("DEEPSEEK_API_KEY");
        f();
        if let Some(v) = oj { std::env::set_var("OPENJOURNAL_DEEPSEEK_API_KEY", v); }
        if let Some(v) = ds { std::env::set_var("DEEPSEEK_API_KEY", v); }
    }

    #[test]
    fn mask_typical_key() {
        let key = "sk-abcdefghijklmnop7890";
        let masked = mask_api_key(key);
        assert_eq!(masked, "sk-••••••••7890");
        assert!(!masked.contains("abcdefghijklmnop"));
    }

    #[test]
    fn mask_short_key() {
        assert_eq!(mask_api_key("ab12"), "••••");
        assert_eq!(mask_api_key("abc"), "••••");
        assert_eq!(mask_api_key(""), "");
    }

    #[test]
    fn mask_edge_key() {
        let key = "sk-a1b2";
        let masked = mask_api_key(key);
        assert!(!masked.contains("a1b2"));
    }

    #[test]
    fn resolve_empty_returns_missing() {
        with_clean_env(|| {
            let (key, source) = resolve_api_key("");
            assert_eq!(key, "");
            assert_eq!(source, "missing");
        });
    }

    #[test]
    fn resolve_session_override() {
        with_clean_env(|| {
            let (key, source) = resolve_api_key("sk-test-session-key");
            assert_eq!(key, "sk-test-session-key");
            assert_eq!(source, "session");
        });
    }

    #[test]
    fn get_status_without_keys() {
        with_clean_env(|| {
            let status = get_api_key_status("");
            assert_eq!(status.source, "missing");
            assert_eq!(status.masked_key, "");
            assert!(!status.has_credential);
        });
    }

    #[test]
    fn get_status_with_session() {
        with_clean_env(|| {
            let status = get_api_key_status("sk-test-session-key-here");
            assert_eq!(status.source, "session");
            assert_eq!(status.masked_key, "sk-••••••••here");
        });
    }

    #[test]
    fn save_and_delete_credential() {
        // Only tested if credential store is available
        let entry = keyring::Entry::new(SERVICE_NAME, "deepseek_api_key_test").unwrap();
        if entry.set_password("sk-test-credential").is_ok() {
            let loaded = entry.get_password().ok();
            assert_eq!(loaded, Some("sk-test-credential".to_string()));
            let _ = entry.delete_credential();
            let after = entry.get_password().ok();
            assert!(after.is_none() || after == Some(String::new()));
        }
    }
}
