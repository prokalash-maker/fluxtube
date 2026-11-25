pub mod config;
pub mod coordinator;
pub mod device;
pub mod error;
pub mod loopback;
pub mod manager;
pub mod pkce;
pub mod token;

pub use config::{
    default_google_scopes, mask_identifier, DesktopOAuthConfig, GOOGLE_AUTH_ENDPOINT,
    GOOGLE_REVOKE_ENDPOINT, GOOGLE_TOKEN_ENDPOINT, GOOGLE_USERINFO_ENDPOINT, YOUTUBE_API_BASE,
    YOUTUBE_SCOPE_FORCE_SSL, YOUTUBE_SCOPE_READONLY,
};
pub use coordinator::{CachedToken, TokenManager, TokenRefreshCoordinator};
pub use device::{
    DeviceCodeResponse, DeviceTokenPoll, GOOGLE_DEVICE_CODE_ENDPOINT,
    GOOGLE_DEVICE_VERIFICATION_URL,
};
pub use error::AuthError;
pub use loopback::LoopbackListener;
pub use manager::{AuthSession, DevicePollOutcome, OAuthManager, SessionStatus};
pub use pkce::PkceSession;
pub use token::{
    revoke_google_token, validate_id_token_claims, GoogleUserInfo, OidcIdTokenClaims,
    TokenExchangeResponse, TokenRefreshResponse,
};
