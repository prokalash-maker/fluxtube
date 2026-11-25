use crate::error::AuthError;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

pub const GOOGLE_AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";
pub const GOOGLE_USERINFO_ENDPOINT: &str = "https://openidconnect.googleapis.com/v1/userinfo";
pub const YOUTUBE_API_BASE: &str = "https://www.googleapis.com/youtube/v3/";

/// Official YouTube Data API v3 scopes matching the methods implemented by FluxTube.
pub const YOUTUBE_SCOPE_READONLY: &str = "https://www.googleapis.com/auth/youtube.readonly";
pub const YOUTUBE_SCOPE_FORCE_SSL: &str = "https://www.googleapis.com/auth/youtube.force-ssl";

/// Returns minimal scopes required for identity + read/write YouTube features.
pub fn default_google_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "profile".to_string(),
        "email".to_string(),
        // youtube.force-ssl covers read + commentThreads.insert + rate + subscriptions + playlists
        YOUTUBE_SCOPE_FORCE_SSL.to_string(),
    ]
}

#[derive(Debug, Deserialize)]
struct GoogleCredentialsFile {
    installed: Option<InstalledClientJson>,
    web: Option<serde_json::Value>,
    #[serde(rename = "type")]
    account_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstalledClientJson {
    client_id: String,
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    auth_uri: Option<String>,
    #[serde(default)]
    token_uri: Option<String>,
    #[serde(default)]
    client_secret: Option<String>,
    #[serde(default)]
    redirect_uris: Vec<String>,
}

/// Validated Google Cloud Console Desktop OAuth 2.0 client configuration.
#[derive(Clone)]
pub struct DesktopOAuthConfig {
    pub client_id: String,
    pub client_secret: Option<SecretString>,
    pub project_id: Option<String>,
    pub auth_uri: String,
    pub token_uri: String,
    pub redirect_uris: Vec<String>,
}

impl std::fmt::Debug for DesktopOAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesktopOAuthConfig")
            .field("client_id", &self.masked_client_id())
            .field("has_client_secret", &self.client_secret.is_some())
            .field("project_id", &self.project_id)
            .field("auth_uri", &self.auth_uri)
            .field("token_uri", &self.token_uri)
            .field("redirect_uris", &self.redirect_uris)
            .finish()
    }
}

impl DesktopOAuthConfig {
    /// Parses a Google Cloud Console `client_secret_*.json` (Desktop application format).
    pub fn from_google_json_str(json_str: &str) -> Result<Self, AuthError> {
        let parsed: GoogleCredentialsFile = serde_json::from_str(json_str)
            .map_err(|e| AuthError::InvalidConfig(format!("Invalid OAuth JSON file: {}", e)))?;

        if parsed.account_type.as_deref() == Some("service_account") {
            return Err(AuthError::InvalidConfig(
                "Service Account JSON is prohibited for user sign-in. Export an OAuth 2.0 Client ID -> Desktop app JSON instead.".to_string(),
            ));
        }

        let installed = parsed.installed.ok_or_else(|| {
            if parsed.web.is_some() {
                AuthError::InvalidConfig(
                    "Found 'web' OAuth config. Create a 'Desktop app' ('installed') OAuth Client ID in Google Cloud Console.".to_string(),
                )
            } else {
                AuthError::InvalidConfig(
                    "Missing top-level 'installed' object in Google OAuth JSON.".to_string(),
                )
            }
        })?;

        let client_id = installed.client_id.trim().to_string();
        if client_id.is_empty() {
            return Err(AuthError::InvalidConfig(
                "Google OAuth JSON has empty 'client_id'.".to_string(),
            ));
        }

        let auth_uri = installed
            .auth_uri
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| GOOGLE_AUTH_ENDPOINT.to_string());
        let token_uri = installed
            .token_uri
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| GOOGLE_TOKEN_ENDPOINT.to_string());

        validate_https_url("auth_uri", &auth_uri)?;
        validate_https_url("token_uri", &token_uri)?;

        let client_secret = installed
            .client_secret
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(SecretString::new);

        Ok(Self {
            client_id,
            client_secret,
            project_id: installed.project_id,
            auth_uri,
            token_uri,
            redirect_uris: installed.redirect_uris,
        })
    }

    pub fn from_file(path: &std::path::Path) -> Result<Self, AuthError> {
        let raw = std::fs::read_to_string(path)?;
        Self::from_google_json_str(&raw)
    }

    pub fn client_secret_exposed(&self) -> Option<&str> {
        self.client_secret
            .as_ref()
            .map(|s| s.expose_secret().as_str())
    }

    pub fn masked_client_id(&self) -> String {
        mask_identifier(&self.client_id)
    }
}

fn validate_https_url(field: &str, raw: &str) -> Result<(), AuthError> {
    let parsed = Url::parse(raw)
        .map_err(|e| AuthError::InvalidConfig(format!("Invalid {} URL: {}", field, e)))?;
    if parsed.scheme() != "https" {
        return Err(AuthError::InvalidConfig(format!(
            "{} must use HTTPS (got '{}')",
            field,
            parsed.scheme()
        )));
    }
    Ok(())
}

/// Masks a Client ID or API key for UI display and diagnostics without exposing secrets.
pub fn mask_identifier(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "Not configured".to_string();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 12 {
        return "***configured***".to_string();
    }
    let head: String = chars[..6].iter().collect();
    let tail: String = chars[chars.len() - 6..].iter().collect();
    format!("{}...{}", head, tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_desktop_google_json() {
        let json = r#"{
            "installed": {
                "client_id": "123456-abcdef.apps.googleusercontent.com",
                "project_id": "fluxtube-test",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "client_secret": "GOCSPX-test-secret",
                "redirect_uris": ["http://localhost"]
            }
        }"#;

        let cfg = DesktopOAuthConfig::from_google_json_str(json).unwrap();
        assert_eq!(cfg.client_id, "123456-abcdef.apps.googleusercontent.com");
        assert_eq!(cfg.client_secret_exposed(), Some("GOCSPX-test-secret"));
        assert_eq!(cfg.project_id.as_deref(), Some("fluxtube-test"));
        assert!(format!("{:?}", cfg).contains("123456...nt.com"));
        assert!(!format!("{:?}", cfg).contains("GOCSPX-test-secret"));
    }

    #[test]
    fn test_reject_service_account_and_web_json() {
        let sa = r#"{"type": "service_account", "project_id": "x"}"#;
        assert!(DesktopOAuthConfig::from_google_json_str(sa).is_err());

        let web = r#"{"web": {"client_id": "abc"}}"#;
        assert!(DesktopOAuthConfig::from_google_json_str(web).is_err());
    }
}
