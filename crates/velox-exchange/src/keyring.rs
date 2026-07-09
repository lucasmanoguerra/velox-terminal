//! Secure credential storage using the OS-native keyring.
//!
//! Stores and retrieves broker API credentials (key, secret, URL) in the
//! operating system's credential manager:
//!
//! - **Linux**: Secret Service (GNOME Keyring / KDE Wallet) via DBus
//! - **macOS**: Keychain Services
//! - **Windows**: Windows Credential Manager
//!
//! # Error handling
//!
//! All operations return `Result<(), KeyringError>`. If the keyring is
//! unavailable (headless server, no desktop session, missing libsecret),
//! operations fail gracefully — the app continues to function, just
//! without persistent credential storage.
//!
//! # Example
//!
//! ```rust,ignore
//! use velox_broker::BrokerConfig;
//! use velox_exchange::keyring;
//!
//! let config = BrokerConfig {
//!     api_key: "my_key".into(),
//!     api_secret: "my_secret".into(),
//!     base_url: "https://api.binance.com".into(),
//!     paper_trading: false,
//! };
//!
//! keyring::save(&config).ok();
//! if let Ok(Some(loaded)) = keyring::load() {
//!     assert_eq!(loaded.api_key, "my_key");
//! }
//! keyring::delete().ok();
//! ```

use serde::{Deserialize, Serialize};
use velox_broker::BrokerConfig;

/// Service name used as the keyring entry identifier.
const SERVICE_NAME: &str = "velox-terminal";

/// Username used as the keyring entry identifier.
const USERNAME: &str = "binance-api";

/// Lightweight JSON wrapper for keyring storage.
#[derive(Serialize, Deserialize)]
struct StoredConfig {
    api_key: String,
    api_secret: String,
    base_url: String,
    paper_trading: bool,
}

impl From<&BrokerConfig> for StoredConfig {
    fn from(c: &BrokerConfig) -> Self {
        Self {
            api_key: c.api_key.clone(),
            api_secret: c.api_secret.clone(),
            base_url: c.base_url.clone(),
            paper_trading: c.paper_trading,
        }
    }
}

impl From<StoredConfig> for BrokerConfig {
    fn from(s: StoredConfig) -> Self {
        Self {
            api_key: s.api_key,
            api_secret: s.api_secret,
            base_url: s.base_url,
            paper_trading: s.paper_trading,
        }
    }
}

/// Save broker credentials to the OS keyring.
///
/// Returns `Ok(())` on success, or `Err(String)` if the keyring is
/// unavailable or the write fails.
pub fn save(config: &BrokerConfig) -> Result<(), String> {
    let stored = StoredConfig::from(config);
    let json = serde_json::to_string(&stored).map_err(|e| format!("JSON serialization: {e}"))?;

    let entry = keyring::Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| format!("Keyring entry creation: {e}"))?;
    entry
        .set_password(&json)
        .map_err(|e| format!("Keyring set_password: {e}"))?;

    Ok(())
}

/// Load broker credentials from the OS keyring.
///
/// Returns `Ok(Some(config))` if credentials exist, `Ok(None)` if no
/// credentials are stored, or `Err(String)` on error.
pub fn load() -> Result<Option<BrokerConfig>, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| format!("Keyring entry creation: {e}"))?;

    match entry.get_password() {
        Ok(json) => {
            let stored: StoredConfig =
                serde_json::from_str(&json).map_err(|e| format!("JSON deserialization: {e}"))?;
            Ok(Some(BrokerConfig::from(stored)))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Keyring get_password: {e}")),
    }
}

/// Delete stored broker credentials from the OS keyring.
///
/// Returns `Ok(())` even if no credentials exist, or `Err(String)` on
/// actual errors (e.g., keyring unavailable).
pub fn delete() -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| format!("Keyring entry creation: {e}"))?;

    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // already absent
        Err(e) => Err(format!("Keyring delete_credential: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BrokerConfig used in tests (doesn't touch the real keyring).
    fn test_config() -> BrokerConfig {
        BrokerConfig {
            api_key: "test_key_12345".into(),
            api_secret: "test_secret_67890".into(),
            base_url: "https://test.binance.com".into(),
            paper_trading: true,
        }
    }

    #[test]
    fn test_stored_config_roundtrip() {
        let config = test_config();
        let stored = StoredConfig::from(&config);
        let restored = BrokerConfig::from(stored);

        assert_eq!(restored.api_key, "test_key_12345");
        assert_eq!(restored.api_secret, "test_secret_67890");
        assert_eq!(restored.base_url, "https://test.binance.com");
        assert!(restored.paper_trading);
    }

    #[test]
    fn test_stored_config_serialization() {
        let config = test_config();
        let stored = StoredConfig::from(&config);
        let json = serde_json::to_string(&stored).unwrap();

        assert!(json.contains("test_key_12345"));
        assert!(json.contains("test_secret_67890"));
        assert!(json.contains("test.binance.com"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_stored_config_deserialization() {
        let json = r#"{
            "api_key": "k1",
            "api_secret": "s1",
            "base_url": "https://example.com",
            "paper_trading": false
        }"#;
        let stored: StoredConfig = serde_json::from_str(json).unwrap();
        let config = BrokerConfig::from(stored);

        assert_eq!(config.api_key, "k1");
        assert_eq!(config.api_secret, "s1");
        assert_eq!(config.base_url, "https://example.com");
        assert!(!config.paper_trading);
    }
}
