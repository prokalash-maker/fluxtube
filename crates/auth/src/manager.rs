use crate::config::{
    mask_identifier, DesktopOAuthConfig, GOOGLE_AUTH_ENDPOINT, GOOGLE_REVOKE_ENDPOINT,
    GOOGLE_TOKEN_ENDPOINT, GOOGLE_USERINFO_ENDPOINT,
};
use crate::coordinator::{TokenManager, TokenRefreshCoordinator};
use crate::device::{
    poll_device_token_once, request_device_code as http_request_device_code, DeviceCodeResponse,
    DeviceTokenPoll, GOOGLE_DEVICE_CODE_ENDPOINT,
};
use crate::error::AuthError;
use crate::loopback::LoopbackListener;
use crate::pkce::PkceSession;
use crate::token::{
    exchange_code_for_tokens, fetch_user_info, revoke_google_token, validate_id_token_claims,
    TokenExchangeResponse,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use fluxtube_core::UserProfile;
use fluxtube_security::CredentialVault;
use fluxtube_storage::StorageManager;
use parking_lot::RwLock;
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// High-level status of the current Google OAuth session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    Guest,
    Authenticated,
    ReauthRequired(String),
}

/// Authenticated user session info (access token is kept only in memory).
#[derive(Clone)]
pub struct AuthSession {
    pub profile: UserProfile,
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub granted_scopes: Vec<String>,
}

impl std::fmt::Debug for AuthSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthSession")
            .field("profile_id", &self.profile.id)
            .field("access_token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("granted_scopes", &self.granted_scopes)
            .finish()
    }
}

/// Non-blocking result from polling a pending TV/Device OAuth authorization.
#[derive(Debug, Clone)]
pub enum DevicePollOutcome {
    Pending,
    SlowDown,
    Authorized(AuthSession),
}

/// Google OAuth 2.0 PKCE + Device Flow authentication manager.
pub struct OAuthManager {
    vault: CredentialVault,
    storage: StorageManager,
    coordinator: Arc<TokenManager>,
    http_client: reqwest::Client,
    client_id: String,
    client_secret: Option<SecretString>,
    scopes: Vec<String>,
    auth_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    revoke_endpoint: String,
    device_code_endpoint: String,
    granted_scopes: RwLock<HashMap<String, Vec<String>>>,
}

impl OAuthManager {
    pub fn new(
        vault: CredentialVault,
        storage: StorageManager,
        http_client: reqwest::Client,
        client_id: String,
        client_secret: Option<String>,
        scopes: Vec<String>,
    ) -> Self {
        Self::with_endpoints(
            vault,
            storage,
            http_client,
            client_id,
            client_secret,
            scopes,
            GOOGLE_TOKEN_ENDPOINT.to_string(),
            GOOGLE_USERINFO_ENDPOINT.to_string(),
        )
    }

    pub fn from_desktop_config(
        vault: CredentialVault,
        storage: StorageManager,
        http_client: reqwest::Client,
        cfg: &DesktopOAuthConfig,
        scopes: Vec<String>,
    ) -> Self {
        let mut mgr = Self::with_endpoints(
            vault,
            storage,
            http_client,
            cfg.client_id.clone(),
            cfg.client_secret_exposed().map(|s| s.to_string()),
            scopes,
            cfg.token_uri.clone(),
            GOOGLE_USERINFO_ENDPOINT.to_string(),
        );
        mgr.auth_endpoint = cfg.auth_uri.clone();
        mgr
    }

    pub fn with_endpoints(
        vault: CredentialVault,
        storage: StorageManager,
        http_client: reqwest::Client,
        client_id: String,
        client_secret: Option<String>,
        scopes: Vec<String>,
        token_endpoint: String,
        userinfo_endpoint: String,
    ) -> Self {
        let coordinator = Arc::new(TokenRefreshCoordinator::with_endpoint(
            vault.clone(),
            http_client.clone(),
            client_id.clone(),
            client_secret.clone(),
            token_endpoint.clone(),
        ));

        Self {
            vault,
            storage,
            coordinator,
            http_client,
            client_id,
            client_secret: client_secret
                .filter(|s| !s.trim().is_empty())
                .map(SecretString::new),
            scopes,
            auth_endpoint: GOOGLE_AUTH_ENDPOINT.to_string(),
            token_endpoint,
            userinfo_endpoint,
            revoke_endpoint: GOOGLE_REVOKE_ENDPOINT.to_string(),
            device_code_endpoint: GOOGLE_DEVICE_CODE_ENDPOINT.to_string(),
            granted_scopes: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_device_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.device_code_endpoint = endpoint.into();
        self
    }

    pub fn with_revoke_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.revoke_endpoint = endpoint.into();
        self
    }

    pub fn with_auth_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.auth_endpoint = endpoint.into();
        self
    }

    /// Access centralized `TokenManager` reference.
    pub fn coordinator(&self) -> &Arc<TokenManager> {
        &self.coordinator
    }

    /// Access credential vault reference.
    pub fn vault(&self) -> &CredentialVault {
        &self.vault
    }

    /// Access storage manager reference.
    pub fn storage(&self) -> &StorageManager {
        &self.storage
    }

    pub fn is_configured(&self) -> bool {
        !self.client_id.trim().is_empty()
    }

    pub fn masked_client_id(&self) -> String {
        mask_identifier(&self.client_id)
    }

    pub fn configured_scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn granted_scopes_for(&self, account_id: &str) -> Vec<String> {
        if let Some(s) = self.granted_scopes.read().get(account_id) {
            return s.clone();
        }
        if let Ok(Some(raw)) = self
            .storage
            .get_setting(&format!("auth_scopes:{}", account_id))
        {
            return raw.split_whitespace().map(|s| s.to_string()).collect();
        }
        self.scopes.clone()
    }

    /// Starts PKCE OAuth 2.0 desktop login flow in system browser.
    pub async fn start_pkce_login(&self) -> Result<AuthSession, AuthError> {
        self.start_pkce_login_cancellable(CancellationToken::new())
            .await
    }

    /// Starts cancellable PKCE OAuth 2.0 desktop login flow in system browser.
    pub async fn start_pkce_login_cancellable(
        &self,
        cancel: CancellationToken,
    ) -> Result<AuthSession, AuthError> {
        if !self.is_configured() {
            return Err(AuthError::MissingClientId);
        }

        let loopback = LoopbackListener::bind().await?;
        let redirect_uri = loopback.redirect_uri();

        let pkce = PkceSession::new();
        let scopes_slice: Vec<&str> = self.scopes.iter().map(|s| s.as_str()).collect();
        let auth_url = pkce.build_authorization_url_with_endpoint(
            &self.auth_endpoint,
            &self.client_id,
            &redirect_uri,
            &scopes_slice,
        );

        tracing::info!(redirect_uri = %redirect_uri, "Opening system browser for Google OAuth PKCE");
        open::that(&auth_url).map_err(|e| AuthError::Io(std::io::Error::other(e)))?;

        let code = loopback
            .wait_for_callback_cancellable(&pkce.state, Duration::from_secs(180), cancel)
            .await?;

        let secret = self
            .client_secret
            .as_ref()
            .map(|s| s.expose_secret().as_str());

        let tokens = exchange_code_for_tokens(
            &self.http_client,
            &self.token_endpoint,
            &self.client_id,
            secret,
            &code,
            &pkce.code_verifier,
            &redirect_uri,
        )
        .await?;

        self.finalize_token_session(tokens, &pkce.state[..8]).await
    }

    /// Requests a Google OAuth 2.0 Device Code for TV / Limited-Input verification (`google.com/device`).
    pub async fn request_device_code(&self) -> Result<DeviceCodeResponse, AuthError> {
        if !self.is_configured() {
            return Err(AuthError::MissingClientId);
        }
        let scopes_slice: Vec<&str> = self.scopes.iter().map(|s| s.as_str()).collect();
        http_request_device_code(
            &self.http_client,
            &self.device_code_endpoint,
            &self.client_id,
            &scopes_slice,
        )
        .await
    }

    /// Polls the token endpoint once for a pending `device_code`.
    pub async fn poll_device_token(&self, device_code: &str) -> Result<DevicePollOutcome, AuthError> {
        let secret = self
            .client_secret
            .as_ref()
            .map(|s| s.expose_secret().as_str());
        match poll_device_token_once(
            &self.http_client,
            &self.token_endpoint,
            &self.client_id,
            secret,
            device_code,
        )
        .await?
        {
            DeviceTokenPoll::Pending => Ok(DevicePollOutcome::Pending),
            DeviceTokenPoll::SlowDown => Ok(DevicePollOutcome::SlowDown),
            DeviceTokenPoll::Tokens(tokens) => {
                let fallback_tag = &device_code[..device_code.len().min(8)];
                let session = self.finalize_token_session(tokens, fallback_tag).await?;
                Ok(DevicePollOutcome::Authorized(session))
            }
        }
    }

    async fn finalize_token_session(
        &self,
        tokens: TokenExchangeResponse,
        fallback_tag: &str,
    ) -> Result<AuthSession, AuthError> {
        let user_info = fetch_user_info(
            &self.http_client,
            &self.userinfo_endpoint,
            &tokens.access_token,
        )
        .await
        .unwrap_or_else(|_| crate::token::GoogleUserInfo {
            sub: Some(format!("user_{}", fallback_tag)),
            id: None,
            email: Some("user@fluxtube.app".to_string()),
            name: Some("FluxTube User".to_string()),
            picture: None,
        });

        let account_id = user_info
            .sub
            .or(user_info.id)
            .unwrap_or_else(|| format!("user_{}", Utc::now().timestamp()));

        // Cross-check OIDC ID token claims when provided by Google
        if let Some(ref id_tok) = tokens.id_token {
            if let Err(e) = validate_id_token_claims(id_tok, &self.client_id, Some(&account_id)) {
                tracing::warn!(error = %e, "ID token claim validation failed");
                return Err(e);
            }
        }

        let email = user_info
            .email
            .unwrap_or_else(|| format!("{}@gmail.com", account_id));
        let name = user_info.name.unwrap_or_else(|| "YouTube User".to_string());
        let picture = user_info.picture;

        let profile = UserProfile {
            id: account_id.clone(),
            email,
            name,
            picture,
            is_active: true,
            created_at: Utc::now(),
        };

        let granted_scopes: Vec<String> = tokens
            .scope
            .as_deref()
            .map(|s| s.split_whitespace().map(|x| x.to_string()).collect())
            .filter(|v: &Vec<String>| !v.is_empty())
            .unwrap_or_else(|| self.scopes.clone());

        self.granted_scopes
            .write()
            .insert(account_id.clone(), granted_scopes.clone());
        let _ = self.storage.set_setting(
            &format!("auth_scopes:{}", account_id),
            &granted_scopes.join(" "),
        );

        // Save refresh token securely in OS Keychain / Credential Manager, access token only in RAM
        self.coordinator.store_tokens(
            &account_id,
            tokens.access_token.clone(),
            tokens.expires_in,
            tokens.refresh_token.as_deref(),
        )?;

        self.storage.save_account(&profile, true)?;

        let expires_at = Utc::now() + ChronoDuration::seconds(tokens.expires_in as i64);
        Ok(AuthSession {
            profile,
            access_token: tokens.access_token,
            expires_at,
            granted_scopes,
        })
    }

    /// Restores an existing session on app startup using its OS vault refresh token (without browser prompt).
    pub async fn validate_and_restore_session(
        &self,
        account_id: &str,
    ) -> Result<String, AuthError> {
        self.coordinator.get_valid_access_token(account_id).await
    }

    /// Recovers once from an HTTP 401 by refreshing the access token.
    pub async fn handle_unauthorized(&self, account_id: &str) -> Result<String, AuthError> {
        self.coordinator.handle_unauthorized(account_id).await
    }

    /// Returns currently active account profile, if any.
    pub fn get_active_account(&self) -> Result<Option<UserProfile>, AuthError> {
        self.storage.get_active_account().map_err(AuthError::Storage)
    }

    /// Lists all saved user account profiles.
    pub fn list_accounts(&self) -> Result<Vec<UserProfile>, AuthError> {
        self.storage.get_all_accounts().map_err(AuthError::Storage)
    }

    /// Switches the active account. Pass None to enter Guest mode.
    pub fn switch_account(&self, account_id: Option<&str>) -> Result<(), AuthError> {
        match account_id {
            Some(id) => {
                self.storage.set_active_account(id).map_err(AuthError::Storage)?;
            }
            None => {
                if let Some(mut active) = self.storage.get_active_account()? {
                    active.is_active = false;
                    self.storage.save_account(&active, false)?;
                }
            }
        }
        Ok(())
    }

    /// Checks if the client is currently in guest mode (no active account).
    pub fn is_guest(&self) -> Result<bool, AuthError> {
        Ok(self.get_active_account()?.is_none())
    }

    /// Retrieves valid access token for current active account. Returns Ok(None) in guest mode.
    pub async fn get_active_access_token(&self) -> Result<Option<String>, AuthError> {
        if let Some(account) = self.get_active_account()? {
            let token = self.coordinator.get_valid_access_token(&account.id).await?;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    /// Refreshes access token for a specific account.
    pub async fn refresh_access_token(&self, account_id: &str) -> Result<String, AuthError> {
        self.coordinator.force_refresh(account_id).await
    }

    /// Logs out an account locally by deleting credentials from vault, memory cache, and SQLite.
    pub async fn logout(&self, account_id: &str) -> Result<(), AuthError> {
        let _ = self.vault.delete_token(account_id);
        self.coordinator.invalidate_cache(account_id);
        self.granted_scopes.write().remove(account_id);
        self.storage.delete_account(account_id)?;
        Ok(())
    }

    /// Revokes access remotely at `https://oauth2.googleapis.com/revoke` and logs out locally.
    pub async fn revoke_and_logout(&self, account_id: &str) -> Result<(), AuthError> {
        let token_to_revoke = self
            .vault
            .get_token(account_id)
            .ok()
            .flatten()
            .or_else(|| self.coordinator.peek_cached_token(account_id));

        if let Some(tok) = token_to_revoke {
            revoke_google_token(&self.http_client, &self.revoke_endpoint, &tok).await?;
        }

        self.logout(account_id).await
    }
}
