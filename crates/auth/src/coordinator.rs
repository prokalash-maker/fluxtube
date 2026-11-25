use crate::config::GOOGLE_TOKEN_ENDPOINT;
use crate::error::AuthError;
use crate::token::refresh_access_token_http;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use fluxtube_security::CredentialVault;
use parking_lot::{Mutex, RwLock};
use secrecy::{ExposeSecret, SecretString};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct CachedToken {
    access_token: SecretString,
    pub expires_at: DateTime<Utc>,
}

impl std::fmt::Debug for CachedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedToken")
            .field("access_token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl CachedToken {
    pub fn new(access_token: String, expires_at: DateTime<Utc>) -> Self {
        Self {
            access_token: SecretString::new(access_token),
            expires_at,
        }
    }

    pub fn expose_access_token(&self) -> &str {
        self.access_token.expose_secret()
    }
}

/// Centralized token coordinator handling single-flight refresh, HTTP 401 recovery,
/// `invalid_grant` / revocation state tracking, and zeroized in-memory token storage.
pub struct TokenRefreshCoordinator {
    vault: CredentialVault,
    account_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    cached_tokens: RwLock<HashMap<String, CachedToken>>,
    last_refresh_at: RwLock<HashMap<String, DateTime<Utc>>>,
    reauth_required: RwLock<HashSet<String>>,
    http_client: reqwest::Client,
    client_id: String,
    client_secret: Option<SecretString>,
    token_endpoint: String,
}

pub type TokenManager = TokenRefreshCoordinator;

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
            GOOGLE_TOKEN_ENDPOINT.to_string(),
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
            last_refresh_at: RwLock::new(HashMap::new()),
            reauth_required: RwLock::new(HashSet::new()),
            http_client,
            client_id,
            client_secret: client_secret
                .filter(|s| !s.trim().is_empty())
                .map(SecretString::new),
            token_endpoint,
        }
    }

    /// Returns whether an account requires interactive re-authorization (e.g. `invalid_grant`).
    pub fn is_reauth_required(&self, account_id: &str) -> bool {
        self.reauth_required.read().contains(account_id)
    }

    /// Returns timestamp of the last successful token refresh for the given account.
    pub fn last_refresh_at(&self, account_id: &str) -> Option<DateTime<Utc>> {
        self.last_refresh_at.read().get(account_id).copied()
    }

    /// Retrieves a valid access token for the given account.
    /// If the cached token is valid for at least 60 seconds, returns immediately.
    /// If expired or absent, performs single-flight refresh without stampedes or infinite loops.
    pub async fn get_valid_access_token(&self, account_id: &str) -> Result<String, AuthError> {
        if self.is_reauth_required(account_id) {
            return Err(AuthError::InvalidGrant(
                "Session expired or revoked; re-authorization required".to_string(),
            ));
        }

        let now = Utc::now();
        let safety_margin = ChronoDuration::seconds(60);

        // Fast path: cached and still valid
        {
            let cache = self.cached_tokens.read();
            if let Some(cached) = cache.get(account_id) {
                if cached.expires_at > now + safety_margin {
                    return Ok(cached.expose_access_token().to_string());
                }
            }
        }

        // Slow path: acquire account-specific lock
        let lock = self.account_lock(account_id);
        let _guard = lock.lock().await;

        if self.is_reauth_required(account_id) {
            return Err(AuthError::InvalidGrant(
                "Session expired or revoked; re-authorization required".to_string(),
            ));
        }

        // Double check cache after acquiring lock
        {
            let cache = self.cached_tokens.read();
            if let Some(cached) = cache.get(account_id) {
                if cached.expires_at > Utc::now() + safety_margin {
                    return Ok(cached.expose_access_token().to_string());
                }
            }
        }

        self.refresh_internal(account_id).await
    }

    /// Handles HTTP 401 responses by invalidating the in-memory access token and performing
    /// a single coordinated refresh attempt without entering a refresh loop.
    pub async fn handle_unauthorized(&self, account_id: &str) -> Result<String, AuthError> {
        self.invalidate_cache(account_id);
        self.force_refresh(account_id).await
    }

    /// Forces a refresh of the access token regardless of cache.
    pub async fn force_refresh(&self, account_id: &str) -> Result<String, AuthError> {
        if self.is_reauth_required(account_id) {
            return Err(AuthError::InvalidGrant(
                "Session expired or revoked; re-authorization required".to_string(),
            ));
        }
        let lock = self.account_lock(account_id);
        let _guard = lock.lock().await;
        if self.is_reauth_required(account_id) {
            return Err(AuthError::InvalidGrant(
                "Session expired or revoked; re-authorization required".to_string(),
            ));
        }
        self.refresh_internal(account_id).await
    }

    /// Stores newly obtained tokens into memory cache and OS secure vault.
    pub fn store_tokens(
        &self,
        account_id: &str,
        access_token: String,
        expires_in: u64,
        refresh_token: Option<&str>,
    ) -> Result<(), AuthError> {
        let now = Utc::now();
        let expires_at = now + ChronoDuration::seconds(expires_in as i64);
        {
            let mut cache = self.cached_tokens.write();
            cache.insert(
                account_id.to_string(),
                CachedToken::new(access_token, expires_at),
            );
        }
        {
            self.reauth_required.write().remove(account_id);
            self.last_refresh_at.write().insert(account_id.to_string(), now);
        }

        if let Some(rt) = refresh_token {
            self.vault.store_token(account_id, rt)?;
        }

        Ok(())
    }

    /// Reads currently cached access token (if any) for revocation on sign-out.
    pub fn peek_cached_token(&self, account_id: &str) -> Option<String> {
        self.cached_tokens
            .read()
            .get(account_id)
            .map(|t| t.expose_access_token().to_string())
    }

    /// Invalidates in-memory cached token for an account.
    pub fn invalidate_cache(&self, account_id: &str) {
        let mut cache = self.cached_tokens.write();
        cache.remove(account_id);
    }

    /// Clears all in-memory token caches and reauth flags.
    pub fn clear_all_cache(&self) {
        self.cached_tokens.write().clear();
        self.reauth_required.write().clear();
        self.last_refresh_at.write().clear();
    }

    fn account_lock(&self, account_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = self.account_locks.lock();
        locks
            .entry(account_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }

    async fn refresh_internal(&self, account_id: &str) -> Result<String, AuthError> {
        let refresh_token = self
            .vault
            .get_token(account_id)?
            .ok_or_else(|| AuthError::NoRefreshToken(account_id.to_string()))?;

        let secret = self
            .client_secret
            .as_ref()
            .map(|s| s.expose_secret().as_str());

        let resp = match refresh_access_token_http(
            &self.http_client,
            &self.token_endpoint,
            &self.client_id,
            secret,
            &refresh_token,
        )
        .await
        {
            Ok(r) => r,
            Err(err @ AuthError::InvalidGrant(_)) | Err(err @ AuthError::Unauthorized(_)) => {
                // Stop retry loops cleanly until the user re-authorizes
                self.invalidate_cache(account_id);
                self.reauth_required.write().insert(account_id.to_string());
                return Err(err);
            }
            Err(e) => return Err(e),
        };

        let now = Utc::now();
        let expires_at = now + ChronoDuration::seconds(resp.expires_in as i64);
        let access_token = resp.access_token.clone();

        {
            let mut cache = self.cached_tokens.write();
            cache.insert(
                account_id.to_string(),
                CachedToken::new(access_token.clone(), expires_at),
            );
        }
        {
            self.reauth_required.write().remove(account_id);
            self.last_refresh_at.write().insert(account_id.to_string(), now);
        }

        if let Some(new_rt) = resp.refresh_token {
            self.vault.store_token(account_id, &new_rt)?;
        }

        Ok(access_token)
    }
}
