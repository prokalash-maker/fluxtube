use crate::coordinator::TokenRefreshCoordinator;
use crate::error::AuthError;
use crate::loopback::LoopbackListener;
use crate::pkce::PkceSession;
use crate::token::{exchange_code_for_tokens, fetch_user_info};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use fluxtube_core::UserProfile;
use fluxtube_security::CredentialVault;
use fluxtube_storage::StorageManager;
use std::sync::Arc;
use std::time::Duration;

/// Authenticated user session info.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub profile: UserProfile,
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Google OAuth 2.0 PKCE authentication manager.
pub struct OAuthManager {
    vault: CredentialVault,
    storage: StorageManager,
    coordinator: Arc<TokenRefreshCoordinator>,
    http_client: reqwest::Client,
    client_id: String,
    client_secret: Option<String>,
    scopes: Vec<String>,
    token_endpoint: String,
    userinfo_endpoint: String,
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
        let coordinator = Arc::new(TokenRefreshCoordinator::new(
            vault.clone(),
            http_client.clone(),
            client_id.clone(),
            client_secret.clone(),
        ));

        Self {
            vault,
            storage,
            coordinator,
            http_client,
            client_id,
            client_secret,
            scopes,
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
            userinfo_endpoint: "https://www.googleapis.com/oauth2/v3/userinfo".to_string(),
        }
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
            client_secret,
            scopes,
            token_endpoint,
            userinfo_endpoint,
        }
    }

    /// Access coordinator reference.
    pub fn coordinator(&self) -> &Arc<TokenRefreshCoordinator> {
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

    /// Starts PKCE OAuth 2.0 desktop login flow.
    pub async fn start_pkce_login(&self) -> Result<AuthSession, AuthError> {
        let loopback = LoopbackListener::bind().await?;
        let redirect_uri = loopback.redirect_uri();

        let pkce = PkceSession::new();
        let scopes_slice: Vec<&str> = self.scopes.iter().map(|s| s.as_str()).collect();
        let auth_url = pkce.build_authorization_url(&self.client_id, &redirect_uri, &scopes_slice);

        tracing::info!(auth_url = %auth_url, "Opening system browser for Google OAuth");
        let _ = open::that(&auth_url);

        let code = loopback
            .wait_for_callback(&pkce.state, Duration::from_secs(120))
            .await?;

        let tokens = exchange_code_for_tokens(
            &self.http_client,
            &self.token_endpoint,
            &self.client_id,
            self.client_secret.as_deref(),
            &code,
            &pkce.code_verifier,
            &redirect_uri,
        )
        .await?;

        // Attempt to fetch profile info from userinfo endpoint
        let user_info = fetch_user_info(&self.http_client, &self.userinfo_endpoint, &tokens.access_token)
            .await
            .unwrap_or_else(|_| crate::token::GoogleUserInfo {
                sub: Some(format!("user_{}", &pkce.state[..8])),
                id: None,
                email: Some("user@fluxtube.app".to_string()),
                name: Some("FluxTube User".to_string()),
                picture: None,
            });

        let account_id = user_info
            .sub
            .or(user_info.id)
            .unwrap_or_else(|| format!("user_{}", Utc::now().timestamp()));

        let email = user_info.email.unwrap_or_else(|| format!("{}@gmail.com", account_id));
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

        // Commit refresh token to secure OS Keyring
        if let Some(ref rt) = tokens.refresh_token {
            self.vault.store_token(&account_id, rt)?;
        }

        // Cache valid token in coordinator
        self.coordinator.store_tokens(
            &account_id,
            tokens.access_token.clone(),
            tokens.expires_in,
            tokens.refresh_token.as_deref(),
        )?;

        // Save account metadata to local SQLite
        self.storage.save_account(&profile, true)?;

        let expires_at = Utc::now() + ChronoDuration::seconds(tokens.expires_in as i64);
        Ok(AuthSession {
            profile,
            access_token: tokens.access_token,
            expires_at,
        })
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
                // Guest mode: clear any currently active account flag
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

    /// Logs out an account by deleting credentials from vault and storage.
    pub async fn logout(&self, account_id: &str) -> Result<(), AuthError> {
        let _ = self.vault.delete_token(account_id);
        self.coordinator.invalidate_cache(account_id);
        self.storage.delete_account(account_id)?;
        Ok(())
    }
}
