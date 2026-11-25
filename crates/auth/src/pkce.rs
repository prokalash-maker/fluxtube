use crate::config::GOOGLE_AUTH_ENDPOINT;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope,
};
use sha2::{Digest, Sha256};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

/// Session holding PKCE code verifier, S256 challenge, and CSRF state token.
#[derive(Debug, Clone)]
pub struct PkceSession {
    pub code_verifier: String,
    pub code_challenge: String,
    pub state: String,
}

impl Default for PkceSession {
    fn default() -> Self {
        Self::new()
    }
}

impl PkceSession {
    /// Generates a new PKCE session using `oauth2`'s cryptographically secure random generators.
    pub fn new() -> Self {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let csrf = CsrfToken::new_random();

        Self {
            code_verifier: verifier.secret().clone(),
            code_challenge: challenge.as_str().to_string(),
            state: csrf.secret().clone(),
        }
    }

    /// Computes the RFC 7636 S256 challenge from a code verifier.
    pub fn compute_s256_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }

    /// Builds the official Google OAuth 2.0 authorization URL for desktop PKCE via `oauth2::BasicClient`.
    pub fn build_authorization_url(
        &self,
        client_id: &str,
        redirect_uri: &str,
        scopes: &[&str],
    ) -> String {
        self.build_authorization_url_with_endpoint(
            GOOGLE_AUTH_ENDPOINT,
            client_id,
            redirect_uri,
            scopes,
        )
    }

    /// Builds an authorization URL using a custom auth endpoint (for Desktop JSON or test mocks).
    pub fn build_authorization_url_with_endpoint(
        &self,
        auth_endpoint: &str,
        client_id: &str,
        redirect_uri: &str,
        scopes: &[&str],
    ) -> String {
        let auth_url = AuthUrl::new(auth_endpoint.to_string())
            .unwrap_or_else(|_| AuthUrl::new(GOOGLE_AUTH_ENDPOINT.to_string()).unwrap());
        let redirect = RedirectUrl::new(redirect_uri.to_string())
            .expect("Valid loopback redirect URI");

        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            None,
            auth_url,
            None,
        )
        .set_redirect_uri(redirect);

        let challenge = PkceCodeChallenge::from_code_verifier_sha256(
            &oauth2::PkceCodeVerifier::new(self.code_verifier.clone()),
        );
        let csrf = CsrfToken::new(self.state.clone());

        let mut req = client
            .authorize_url(|| csrf)
            .set_pkce_challenge(challenge)
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent");

        for s in scopes {
            req = req.add_scope(Scope::new((*s).to_string()));
        }

        let (url, _state) = req.url();
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_rfc7636_test_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

        let computed_challenge = PkceSession::compute_s256_challenge(verifier);
        assert_eq!(computed_challenge, expected_challenge);
    }

    #[test]
    fn test_pkce_session_generation() {
        let session = PkceSession::new();

        assert!(session.code_verifier.len() >= 43 && session.code_verifier.len() <= 128);
        assert_eq!(session.code_challenge.len(), 43);
        assert!(!session.state.is_empty());

        let url = session.build_authorization_url(
            "mock-client-id.apps.googleusercontent.com",
            "http://127.0.0.1:8080/callback",
            &["https://www.googleapis.com/auth/youtube.readonly"],
        );

        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("prompt=consent"));
        assert!(url.contains(&format!("state={}", session.state)));
    }
}
