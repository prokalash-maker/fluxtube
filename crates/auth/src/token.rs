use crate::error::AuthError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
    pub id_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: Option<String>,
    pub id: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorPayload {
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OidcIdTokenClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub exp: i64,
}

fn map_oauth_token_error(status: reqwest::StatusCode, body: &str) -> AuthError {
    if let Ok(payload) = serde_json::from_str::<OAuthErrorPayload>(body) {
        let err_code = payload.error.unwrap_or_default();
        let desc = payload
            .error_description
            .unwrap_or_else(|| err_code.clone());
        return match err_code.as_str() {
            "invalid_grant" => AuthError::InvalidGrant(desc),
            "invalid_client" | "unauthorized_client" => AuthError::InvalidConfig(desc),
            "redirect_uri_mismatch" => AuthError::RedirectUriMismatch(desc),
            "access_denied" => AuthError::AccessDenied(desc),
            _ if status.as_u16() == 401 => AuthError::Unauthorized(desc),
            _ => AuthError::InvalidTokenResponse(format!("{}: {}", err_code, desc)),
        };
    }

    if status.as_u16() == 401 {
        AuthError::Unauthorized(format!("HTTP 401: {}", body))
    } else {
        AuthError::InvalidTokenResponse(format!("HTTP {}: {}", status.as_u16(), body))
    }
}

/// Validates mandatory OIDC ID token claims (`iss`, `aud`, `exp`, `sub`) after TLS exchange.
/// Note: user identity is always fetched over TLS from `openidconnect.googleapis.com/v1/userinfo`,
/// and this validates claim consistency rather than trusting an unverified token alone.
pub fn validate_id_token_claims(
    id_token: &str,
    expected_client_id: &str,
    expected_sub: Option<&str>,
) -> Result<OidcIdTokenClaims, AuthError> {
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidIdToken(
            "Malformed JWT structure".to_string(),
        ));
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| AuthError::InvalidIdToken(format!("Invalid base64 payload: {}", e)))?;
    let claims: OidcIdTokenClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| AuthError::InvalidIdToken(format!("Missing mandatory claims: {}", e)))?;

    if claims.iss != "https://accounts.google.com" && claims.iss != "accounts.google.com" {
        return Err(AuthError::InvalidIdToken(format!(
            "Untrusted issuer '{}'",
            claims.iss
        )));
    }
    if claims.aud != expected_client_id {
        return Err(AuthError::InvalidIdToken(
            "ID token audience ('aud') does not match configured Client ID".to_string(),
        ));
    }
    if claims.exp <= Utc::now().timestamp() {
        return Err(AuthError::InvalidIdToken("ID token is expired".to_string()));
    }
    if let Some(sub) = expected_sub {
        if claims.sub != sub {
            return Err(AuthError::InvalidIdToken(
                "ID token 'sub' does not match authenticated UserInfo profile".to_string(),
            ));
        }
    }

    Ok(claims)
}

/// Exchanges an authorization code + PKCE verifier for access and refresh tokens.
pub async fn exchange_code_for_tokens(
    client: &reqwest::Client,
    token_endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenExchangeResponse, AuthError> {
    let mut params = vec![
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", code_verifier),
        ("redirect_uri", redirect_uri),
    ];

    if let Some(secret) = client_secret.filter(|s| !s.trim().is_empty()) {
        params.push(("client_secret", secret));
    }

    let resp = client.post(token_endpoint).form(&params).send().await?;
    let status = resp.status();

    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(map_oauth_token_error(status, &err_text));
    }

    let tokens: TokenExchangeResponse = resp.json().await.map_err(|e| {
        AuthError::InvalidTokenResponse(format!("Failed to parse token response: {}", e))
    })?;

    Ok(tokens)
}

/// Refreshes an expired access token using the stored refresh token.
pub async fn refresh_access_token_http(
    client: &reqwest::Client,
    token_endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> Result<TokenRefreshResponse, AuthError> {
    let mut params = vec![
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    if let Some(secret) = client_secret.filter(|s| !s.trim().is_empty()) {
        params.push(("client_secret", secret));
    }

    let resp = client.post(token_endpoint).form(&params).send().await?;
    let status = resp.status();

    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(map_oauth_token_error(status, &err_text));
    }

    let tokens: TokenRefreshResponse = resp.json().await.map_err(|e| {
        AuthError::InvalidTokenResponse(format!("Failed to parse refresh response: {}", e))
    })?;

    Ok(tokens)
}

/// Revokes an active Google OAuth refresh or access token via `https://oauth2.googleapis.com/revoke`.
pub async fn revoke_google_token(
    client: &reqwest::Client,
    revoke_endpoint: &str,
    token: &str,
) -> Result<(), AuthError> {
    let resp = client
        .post(revoke_endpoint)
        .form(&[("token", token)])
        .send()
        .await?;

    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }

    let body = resp.text().await.unwrap_or_default();
    // Google returns 400 invalid_token if already revoked — treat that cleanly when logging out
    if body.contains("invalid_token") {
        return Ok(());
    }
    Err(map_oauth_token_error(status, &body))
}

/// Fetches user profile from Google UserInfo endpoint with bearer access token.
pub async fn fetch_user_info(
    client: &reqwest::Client,
    userinfo_endpoint: &str,
    access_token: &str,
) -> Result<GoogleUserInfo, AuthError> {
    let resp = client
        .get(userinfo_endpoint)
        .bearer_auth(access_token)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(map_oauth_token_error(status, &err_text));
    }

    let info: GoogleUserInfo = resp.json().await.map_err(|e| {
        AuthError::InvalidTokenResponse(format!("Failed to parse userinfo response: {}", e))
    })?;

    Ok(info)
}
