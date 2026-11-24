//! Thread-Safe In-Memory Credential Vault for Offline FluxTube Testing.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory key-value vault mimicking the OS Credential Store (macOS Keychain / Windows Credential Manager).
#[derive(Debug, Default, Clone)]
pub struct MockCredentialVault {
    tokens: Arc<RwLock<HashMap<String, String>>>,
}

impl MockCredentialVault {
    /// Creates a new isolated in-memory vault.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a refresh token associated with an account ID.
    pub fn store_token(&self, account_id: &str, token: &str) -> Result<(), String> {
        self.tokens
            .write()
            .map_err(|e| format!("Vault write lock error: {e}"))?
            .insert(account_id.to_string(), token.to_string());
        Ok(())
    }

    /// Retrieves the stored refresh token for an account ID.
    pub fn get_token(&self, account_id: &str) -> Result<Option<String>, String> {
        Ok(self
            .tokens
            .read()
            .map_err(|e| format!("Vault read lock error: {e}"))?
            .get(account_id)
            .cloned())
    }

    /// Deletes the token associated with an account ID.
    pub fn delete_token(&self, account_id: &str) -> Result<bool, String> {
        let removed = self
            .tokens
            .write()
            .map_err(|e| format!("Vault write lock error: {e}"))?
            .remove(account_id);
        Ok(removed.is_some())
    }

    /// Checks if a token exists for the given account.
    pub fn contains(&self, account_id: &str) -> bool {
        self.tokens
            .read()
            .map(|m| m.contains_key(account_id))
            .unwrap_or(false)
    }

    /// Returns the number of tokens stored.
    pub fn len(&self) -> usize {
        self.tokens.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Checks if the vault is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all stored tokens.
    pub fn clear(&self) {
        if let Ok(mut lock) = self.tokens.write() {
            lock.clear();
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_mock_vault_store_retrieve_delete() {
        let vault = MockCredentialVault::new();
        assert!(vault.is_empty());

        vault.store_token("user_1", "refresh_secret_123").unwrap();
        assert_eq!(vault.len(), 1);
        assert!(vault.contains("user_1"));

        let retrieved = vault.get_token("user_1").unwrap();
        assert_eq!(retrieved, Some("refresh_secret_123".to_string()));

        let deleted = vault.delete_token("user_1").unwrap();
        assert!(deleted);
        assert_eq!(vault.get_token("user_1").unwrap(), None);
        assert!(vault.is_empty());
    }

    #[test]
    fn test_mock_vault_thread_safety() {
        let vault = MockCredentialVault::new();
        let mut handles = vec![];

        for i in 0..10 {
            let v = vault.clone();
            handles.push(std::thread::spawn(move || {
                let id = format!("account_{i}");
                let token = format!("token_{i}");
                v.store_token(&id, &token).unwrap();
                assert_eq!(v.get_token(&id).unwrap(), Some(token));
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(vault.len(), 10);
    }
}
