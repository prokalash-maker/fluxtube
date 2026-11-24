use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use zeroize::Zeroizing;

#[derive(thiserror::Error, Debug)]
pub enum SecurityError {
    #[error("OS Keyring platform failure: {0}")]
    PlatformFailure(String),

    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("Lock error: {0}")]
    Lock(String),
}

#[derive(Clone)]
enum VaultBackend {
    Native { service_name: String },
    InMemory(Arc<RwLock<HashMap<String, Zeroizing<String>>>>),
}

#[derive(Clone)]
pub struct CredentialVault {
    backend: VaultBackend,
}

impl CredentialVault {
    pub fn new(service_name: &str) -> Self {
        Self {
            backend: VaultBackend::Native {
                service_name: service_name.to_string(),
            },
        }
    }

    pub fn in_memory() -> Self {
        Self {
            backend: VaultBackend::InMemory(Arc::new(RwLock::new(HashMap::new()))),
        }
    }

    pub fn auto(service_name: &str) -> Self {
        if std::env::var("FLUXTUBE_MOCK_VAULT").is_ok() || std::env::var("CI").is_ok() {
            tracing::info!("Initializing CredentialVault in InMemory mock mode for CI/test");
            Self::in_memory()
        } else {
            Self::new(service_name)
        }
    }

    pub fn is_in_memory(&self) -> bool {
        matches!(self.backend, VaultBackend::InMemory(_))
    }

    pub fn store_token(&self, account_id: &str, refresh_token: &str) -> Result<(), SecurityError> {
        let key = format!("account:{}", account_id);
        match &self.backend {
            VaultBackend::Native { service_name } => {
                let entry = keyring::Entry::new(service_name, &key)?;
                entry.set_password(refresh_token)?;
                Ok(())
            }
            VaultBackend::InMemory(store) => {
                let mut map = store.write();
                map.insert(key, Zeroizing::new(refresh_token.to_string()));
                Ok(())
            }
        }
    }

    pub fn get_token(&self, account_id: &str) -> Result<Option<String>, SecurityError> {
        let key = format!("account:{}", account_id);
        match &self.backend {
            VaultBackend::Native { service_name } => {
                let entry = keyring::Entry::new(service_name, &key)?;
                match entry.get_password() {
                    Ok(pass) => Ok(Some(pass)),
                    Err(keyring::Error::NoEntry) => Ok(None),
                    Err(e) => Err(SecurityError::Keyring(e)),
                }
            }
            VaultBackend::InMemory(store) => {
                let map = store.read();
                Ok(map.get(&key).map(|v| v.as_str().to_string()))
            }
        }
    }

    pub fn delete_token(&self, account_id: &str) -> Result<(), SecurityError> {
        let key = format!("account:{}", account_id);
        match &self.backend {
            VaultBackend::Native { service_name } => {
                let entry = keyring::Entry::new(service_name, &key)?;
                match entry.delete_credential() {
                    Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
                    Err(e) => Err(SecurityError::Keyring(e)),
                }
            }
            VaultBackend::InMemory(store) => {
                let mut map = store.write();
                map.remove(&key);
                Ok(())
            }
        }
    }

    pub fn wipe_all(&self) -> Result<(), SecurityError> {
        match &self.backend {
            VaultBackend::Native { .. } => {
                // OS keyrings require account identifiers to delete entries
                Ok(())
            }
            VaultBackend::InMemory(store) => {
                let mut map = store.write();
                map.clear();
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_vault_crud() {
        let vault = CredentialVault::in_memory();
        assert!(vault.is_in_memory());

        // Initially empty
        assert_eq!(vault.get_token("acc_1").unwrap(), None);

        // Store token
        vault.store_token("acc_1", "refresh_token_12345").unwrap();
        assert_eq!(
            vault.get_token("acc_1").unwrap(),
            Some("refresh_token_12345".to_string())
        );

        // Update token
        vault.store_token("acc_1", "refresh_token_updated").unwrap();
        assert_eq!(
            vault.get_token("acc_1").unwrap(),
            Some("refresh_token_updated".to_string())
        );

        // Delete token
        vault.delete_token("acc_1").unwrap();
        assert_eq!(vault.get_token("acc_1").unwrap(), None);

        // Idempotent delete (deleting non-existent should succeed)
        vault.delete_token("acc_1").unwrap();
    }

    #[test]
    fn test_in_memory_wipe_all() {
        let vault = CredentialVault::in_memory();
        vault.store_token("u1", "token1").unwrap();
        vault.store_token("u2", "token2").unwrap();

        assert_eq!(vault.get_token("u1").unwrap(), Some("token1".to_string()));
        assert_eq!(vault.get_token("u2").unwrap(), Some("token2".to_string()));

        vault.wipe_all().unwrap();

        assert_eq!(vault.get_token("u1").unwrap(), None);
        assert_eq!(vault.get_token("u2").unwrap(), None);
    }

    #[test]
    fn test_auto_vault_mode_env() {
        std::env::set_var("FLUXTUBE_MOCK_VAULT", "1");
        let vault = CredentialVault::auto("fluxtube_test");
        assert!(vault.is_in_memory());
    }
}
