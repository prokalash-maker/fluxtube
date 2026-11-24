use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use url::Url;

/// Session holding PKCE code verifier, challenge, and CSRF state token.
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
    /// Generates a new PKCE session with high-entropy code verifier, S256 challenge, and CSRF state.
    pub fn new() -> Self {
        let mut verifier_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut verifier_bytes);
        let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

        let code_challenge = Self::compute_s256_challenge(&code_verifier);

        let mut state_bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut state_bytes);
        let state = URL_SAFE_NO_PAD.encode(state_bytes);

        Self {
            code_verifier,
            code_challenge,
            state,
        }
    }

    /// Computes the RFC 7636 S256 challenge from a code verifier.
    pub fn compute_s256_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }

    /// Builds the Google OAuth 2.0 authorization URL for desktop PKCE.
    pub fn build_authorization_url(
        &self,
        client_id: &str,
        redirect_uri: &str,
        scopes: &[&str],
    ) -> String {
        let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
            .expect("Valid Google OAuth base URL");

        let scope_str = scopes.join(" ");

        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &scope_str)
            .append_pair("code_challenge", &self.code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &self.state)
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");

        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_rfc7636_test_vector() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

        let computed_challenge = PkceSession::compute_s256_challenge(verifier);
        assert_eq!(computed_challenge, expected_challenge);
    }

    #[test]
    fn test_pkce_session_generation() {
        let session = PkceSession::new();

        // Verifier length must be between 43 and 128 characters per RFC 7636 §4.1
        assert!(session.code_verifier.len() >= 43 && session.code_verifier.len() <= 128);
        assert_eq!(session.code_challenge.len(), 43); // 32 bytes base64url no-pad is 43 chars
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
