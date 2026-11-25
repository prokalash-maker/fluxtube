use crate::error::AuthError;
use crate::token::TokenExchangeResponse;
use serde::{Deserialize, Serialize};

pub const GOOGLE_DEVICE_CODE_ENDPOINT: &str = "https://oauth2.googleapis.com/device/code";
pub const GOOGLE_DEVICE_VERIFICATION_URL: &str = "https://www.google.com/device";

/// Response from Google OAuth 2.0 Device Authorization endpoint (`POST /device/code`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    #[serde(alias = "verification_uri")]
    pub verification_url: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
struct OAuthErrorBody {
    error: String,
    error_description: Option<String>,
}

/// Result of a single non-blocking poll against the OAuth token endpoint for a device code.
#[derive(Debug, Clone)]
pub enum DeviceTokenPoll {
    Pending,
    SlowDown,
    Tokens(TokenExchangeResponse),
}

/// Requests a user activation code and device verification URL for TV/Device OAuth 2.0 flow.
pub async fn request_device_code(
    client: &reqwest::Client,
    device_endpoint: &str,
    client_id: &str,
    scopes: &[&str],
) -> Result<DeviceCodeResponse, AuthError> {
    if client_id.trim().is_empty() {
        return Err(AuthError::MissingClientId);
    }

    let scope_str = scopes.join(" ");
    let params = [("client_id", client_id), ("scope", scope_str.as_str())];

    let resp = client.post(device_endpoint).form(&params).send().await?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AuthError::InvalidTokenResponse(format!(
            "Device code request failed: {}",
            err_text
        )));
    }

    let parsed: DeviceCodeResponse = resp.json().await.map_err(|e| {
        AuthError::InvalidTokenResponse(format!("Failed to parse device code response: {}", e))
    })?;

    Ok(parsed)
}

/// Performs one RFC 8628 token-exchange poll attempt for a pending `device_code`.
pub async fn poll_device_token_once(
    client: &reqwest::Client,
    token_endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    device_code: &str,
) -> Result<DeviceTokenPoll, AuthError> {
    let mut params = vec![
        ("client_id", client_id),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ];

    if let Some(secret) = client_secret.filter(|s| !s.is_empty()) {
        params.push(("client_secret", secret));
    }

    let resp = client.post(token_endpoint).form(&params).send().await?;
    let status = resp.status();
    let bytes = resp.bytes().await?;

    if status.is_success() {
        let tokens: TokenExchangeResponse = serde_json::from_slice(&bytes).map_err(|e| {
            AuthError::InvalidTokenResponse(format!("Failed to parse device token response: {}", e))
        })?;
        return Ok(DeviceTokenPoll::Tokens(tokens));
    }

    if let Ok(err_body) = serde_json::from_slice::<OAuthErrorBody>(&bytes) {
        return match err_body.error.as_str() {
            "authorization_pending" => Ok(DeviceTokenPoll::Pending),
            "slow_down" => Ok(DeviceTokenPoll::SlowDown),
            "expired_token" => Err(AuthError::DeviceCodeExpired),
            "access_denied" => Err(AuthError::AccessDenied(
                err_body
                    .error_description
                    .unwrap_or_else(|| "User denied access".to_string()),
            )),
            other => Err(AuthError::InvalidTokenResponse(format!(
                "{}: {}",
                other,
                err_body.error_description.unwrap_or_default()
            ))),
        };
    }

    let text = String::from_utf8_lossy(&bytes);
    Err(AuthError::InvalidTokenResponse(format!(
        "Device token poll HTTP {}: {}",
        status, text
    )))
}
