use crate::error::AuthError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub token_type: Option<String>,
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

/// Exchanges an authorization code for access and refresh tokens.
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

    if let Some(secret) = client_secret {
        params.push(("client_secret", secret));
    }

    let resp = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AuthError::InvalidTokenResponse(format!(
            "Token exchange failed: {}",
            err_text
        )));
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

    if let Some(secret) = client_secret {
        params.push(("client_secret", secret));
    }

    let resp = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AuthError::InvalidTokenResponse(format!(
            "Token refresh failed: {}",
            err_text
        )));
    }

    let tokens: TokenRefreshResponse = resp.json().await.map_err(|e| {
        AuthError::InvalidTokenResponse(format!("Failed to parse refresh response: {}", e))
    })?;

    Ok(tokens)
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

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AuthError::InvalidTokenResponse(format!(
            "UserInfo fetch failed: {}",
            err_text
        )));
    }

    let info: GoogleUserInfo = resp.json().await.map_err(|e| {
        AuthError::InvalidTokenResponse(format!("Failed to parse userinfo response: {}", e))
    })?;

    Ok(info)
}
