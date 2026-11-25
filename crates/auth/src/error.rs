use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("No active account is currently logged in")]
    NoActiveAccount,

    #[error("No refresh token stored for account {0}")]
    NoRefreshToken(String),

    #[error("OAuth configuration is invalid or missing: {0}")]
    InvalidConfig(String),

    #[error("OAuth authorization was cancelled")]
    Cancelled,

    #[error("OAuth authorization denied: {0}")]
    AccessDenied(String),

    #[error("Redirect URI mismatch: {0}")]
    RedirectUriMismatch(String),

    #[error("CSRF state token mismatch")]
    CsrfMismatch,

    #[error("Loopback listener timed out waiting for authorization callback")]
    LoopbackTimeout,

    #[error("Google OAuth Client ID is not configured")]
    MissingClientId,

    #[error("Device activation code expired; please request a new code")]
    DeviceCodeExpired,

    #[error("Refresh token expired or access revoked (invalid_grant): {0}")]
    InvalidGrant(String),

    #[error("Unauthorized (HTTP 401): {0}")]
    Unauthorized(String),

    #[error("OpenID Connect ID token validation failed: {0}")]
    InvalidIdToken(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Security credential vault error: {0}")]
    Security(#[from] fluxtube_security::SecurityError),

    #[error("Database storage error: {0}")]
    Storage(#[from] fluxtube_storage::StorageError),

    #[error("Invalid token response: {0}")]
    InvalidTokenResponse(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
}
