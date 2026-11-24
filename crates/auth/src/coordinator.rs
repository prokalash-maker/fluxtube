use crate::error::AuthError;
use crate::token::refresh_access_token_http;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use fluxtube_security::CredentialVault;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

pub struct TokenRefreshCoordinator {
    vault: CredentialVault,
    account_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    cached_tokens: RwLock<HashMap<String, CachedToken>>,
    http_client: reqwest::Client,
    client_id: String,
    client_secret: Option<String>,
    token_endpoint: String,
}

impl TokenRefreshCoordinator {
    pub fn new(
        vault: CredentialVault,
        http_client: reqwest::Client,
        client_id: String,
        client_secret: Option<String>,
    ) -> Self {
        Self::with_endpoint(
            vault,
            http_client,
            client_id,
            client_secret,
            "https://oauth2.googleapis.com/token".to_string(),
        )
    }

    pub fn with_endpoint(
        vault: CredentialVault,
        http_client: reqwest::Client,
        client_id: String,
        client_secret: Option<String>,
        token_endpoint: String,
    ) -> Self {
        Self {
            vault,
            account_locks: Mutex::new(HashMap::new()),
            cached_tokens: RwLock::new(HashMap::new()),
            http_client,
            client_id,
            client_secret,
            token_endpoint,
        }
    }

    /// Retrieves a valid access token for the given account.
    /// If the cached token is valid for at least 60 seconds, returns immediately.
    /// If expired or absent, performs coordinated refresh without stampedes.
    pub async fn get_valid_access_token(&self, account_id: &str) -> Result<String, AuthError> {
        let now = Utc::now();
        let safety_margin = ChronoDuration::seconds(60);

        // Fast path: cached and still valid
        {
            let cache = self.cached_tokens.read();
            if let Some(cached) = cache.get(account_id) {
                if cached.expires_at > now + safety_margin {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        // Slow path: acquire account-specific lock
        let lock = {
            let mut locks = self.account_locks.lock();
            locks
                .entry(account_id.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };

        let _guard = lock.lock().await;

        // Double check cache after acquiring lock
        {
            let cache = self.cached_tokens.read();
            if let Some(cached) = cache.get(account_id) {
                if cached.expires_at > Utc::now() + safety_margin {
                    return Ok(cached.access_token.clone());
                }
            }
        }

        self.refresh_internal(account_id).await
    }

    /// Forces a refresh of the access token regardless of cache.
    pub async fn force_refresh(&self, account_id: &str) -> Result<String, AuthError> {
        let lock = {
            let mut locks = self.account_locks.lock();
            locks
                .entry(account_id.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };

        let _guard = lock.lock().await;
        self.refresh_internal(account_id).await
    }

    /// Stores newly obtained tokens into cache and vault.
    pub fn store_tokens(
        &self,
        account_id: &str,
        access_token: String,
        expires_in: u64,
        refresh_token: Option<&str>,
    ) -> Result<(), AuthError> {
        let expires_at = Utc::now() + ChronoDuration::seconds(expires_in as i64);
        {
            let mut cache = self.cached_tokens.write();
            cache.insert(
                account_id.to_string(),
                CachedToken {
                    access_token,
                    expires_at,
                },
            );
        }

        if let Some(rt) = refresh_token {
            self.vault.store_token(account_id, rt)?;
        }

        Ok(())
    }

    /// Invalidates in-memory cached token for an account.
    pub fn invalidate_cache(&self, account_id: &str) {
        let mut cache = self.cached_tokens.write();
        cache.remove(account_id);
    }

    /// Clears all in-memory token caches.
    pub fn clear_all_cache(&self) {
        let mut cache = self.cached_tokens.write();
        cache.clear();
    }

    async fn refresh_internal(&self, account_id: &str) -> Result<String, AuthError> {
        let refresh_token = self
            .vault
            .get_token(account_id)?
            .ok_or_else(|| AuthError::NoRefreshToken(account_id.to_string()))?;

        let resp = refresh_access_token_http(
            &self.http_client,
            &self.token_endpoint,
            &self.client_id,
            self.client_secret.as_deref(),
            &refresh_token,
        )
        .await?;

        let expires_at = Utc::now() + ChronoDuration::seconds(resp.expires_in as i64);
        let access_token = resp.access_token.clone();

        {
            let mut cache = self.cached_tokens.write();
            cache.insert(
                account_id.to_string(),
                CachedToken {
                    access_token: access_token.clone(),
                    expires_at,
                },
            );
        }

        if let Some(new_rt) = resp.refresh_token {
            self.vault.store_token(account_id, &new_rt)?;
        }

        Ok(access_token)
    }
}
