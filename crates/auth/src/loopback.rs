use crate::error::AuthError;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

/// Loopback HTTP listener for capturing OAuth 2.0 authorization codes on 127.0.0.1:0.
pub struct LoopbackListener {
    listener: TcpListener,
    port: u16,
}

impl LoopbackListener {
    /// Binds an ephemeral loopback server on 127.0.0.1:0.
    pub async fn bind() -> Result<Self, AuthError> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    /// Bound port number.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Redirect URI to supply to OAuth authorization request.
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }

    /// Waits for incoming authorization callback, verifies CSRF state token, and serves HTML response.
    pub async fn wait_for_callback(
        self,
        expected_state: &str,
        timeout: Duration,
    ) -> Result<String, AuthError> {
        tokio::time::timeout(timeout, self.handle_callback(expected_state))
            .await
            .map_err(|_| AuthError::LoopbackTimeout)?
    }

    async fn handle_callback(self, expected_state: &str) -> Result<String, AuthError> {
        let (mut socket, _) = self.listener.accept().await?;

        let mut buffer = [0u8; 4096];
        let bytes_read = socket.read(&mut buffer).await?;
        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

        // Extract first line: "GET /callback?code=...&state=... HTTP/1.1"
        let first_line = request_str
            .lines()
            .next()
            .ok_or_else(|| AuthError::InvalidTokenResponse("Empty HTTP request".to_string()))?;

        let mut parts = first_line.split_whitespace();
        let _method = parts.next();
        let raw_path = parts.next().unwrap_or("/");

        let dummy_url = format!("http://127.0.0.1:{}", self.port);
        let parsed_url = Url::parse(&format!("{}{}", dummy_url, raw_path))?;

        let mut code = None;
        let mut state = None;
        let mut error = None;

        for (k, v) in parsed_url.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                "error" => error = Some(v.into_owned()),
                _ => {}
            }
        }

        if let Some(err) = error {
            let error_html = format!(
                "HTTP/1.1 400 Bad Request\r\n\
                 Content-Type: text/html; charset=utf-8\r\n\
                 Connection: close\r\n\r\n\
                 <!DOCTYPE html><html><body style=\"font-family:system-ui;text-align:center;padding:50px;background:#18181b;color:#f87171;\">\
                 <h2>Authorization Failed</h2><p>{}</p></body></html>",
                err
            );
            let _ = socket.write_all(error_html.as_bytes()).await;
            return Err(AuthError::AccessDenied(err));
        }

        match state {
            Some(s) if s == expected_state => {
                // CSRF token verified
            }
            _ => {
                let mismatch_html =
                    "HTTP/1.1 400 Bad Request\r\n\
                     Content-Type: text/html; charset=utf-8\r\n\
                     Connection: close\r\n\r\n\
                     <!DOCTYPE html><html><body style=\"font-family:system-ui;text-align:center;padding:50px;background:#18181b;color:#f87171;\">\
                     <h2>Security Error</h2><p>CSRF state token mismatch. Authorization aborted.</p></body></html>";
                let _ = socket.write_all(mismatch_html.as_bytes()).await;
                return Err(AuthError::CsrfMismatch);
            }
        }

        let auth_code = code.ok_or_else(|| {
            AuthError::InvalidTokenResponse("Missing code parameter in authorization callback".to_string())
        })?;

        let success_html =
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/html; charset=utf-8\r\n\
             Connection: close\r\n\r\n\
             <!DOCTYPE html>\
             <html lang=\"en\">\
             <head>\
               <meta charset=\"utf-8\">\
               <title>FluxTube - Authorization Successful</title>\
             </head>\
             <body style=\"font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; text-align: center; padding: 60px 20px; background: #18181b; color: #f4f4f5;\">\
               <div style=\"max-width: 480px; margin: 0 auto; background: #27272a; padding: 32px; border-radius: 12px; box-shadow: 0 4px 12px rgba(0,0,0,0.5);\">\
                 <h1 style=\"color: #22c55e; margin-top: 0;\">&#10003; Authorization Successful!</h1>\
                 <p style=\"font-size: 1.1rem; color: #e4e4e7; margin: 16px 0;\">You have successfully authenticated with Google.</p>\
                 <p style=\"font-size: 0.95rem; color: #a1a1aa;\">You can safely close this browser window and return to FluxTube.</p>\
               </div>\
             </body>\
             </html>";

        let _ = socket.write_all(success_html.as_bytes()).await;
        let _ = socket.flush().await;

        Ok(auth_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_loopback_listener_ephemeral_port() {
        let listener = LoopbackListener::bind().await.unwrap();
        assert!(listener.port() > 1024);
        assert_eq!(
            listener.redirect_uri(),
            format!("http://127.0.0.1:{}/callback", listener.port())
        );
    }

    #[tokio::test]
    async fn test_loopback_callback_success() {
        let listener = LoopbackListener::bind().await.unwrap();
        let port = listener.port();
        let state = "csrf_test_state_12345";

        let handle = tokio::spawn(async move {
            listener
                .wait_for_callback(state, Duration::from_secs(5))
                .await
        });

        // Send simulated browser redirect request
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!(
                "http://127.0.0.1:{}/callback?code=mock_auth_code_xyz&state={}",
                port, state
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let body = resp.text().await.unwrap();
        assert!(body.contains("Authorization Successful"));

        let code = handle.await.unwrap().unwrap();
        assert_eq!(code, "mock_auth_code_xyz");
    }

    #[tokio::test]
    async fn test_loopback_csrf_mismatch() {
        let listener = LoopbackListener::bind().await.unwrap();
        let port = listener.port();

        let handle = tokio::spawn(async move {
            listener
                .wait_for_callback("legitimate_state", Duration::from_secs(5))
                .await
        });

        let client = reqwest::Client::new();
        let resp = client
            .get(&format!(
                "http://127.0.0.1:{}/callback?code=mock_code&state=tampered_state",
                port
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

        let result = handle.await.unwrap();
        match result {
            Err(AuthError::CsrfMismatch) => {}
            other => panic!("Expected CsrfMismatch, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_loopback_timeout() {
        let listener = LoopbackListener::bind().await.unwrap();
        let result = listener
            .wait_for_callback("csrf_token", Duration::from_millis(50))
            .await;

        match result {
            Err(AuthError::LoopbackTimeout) => {}
            other => panic!("Expected LoopbackTimeout, got: {:?}", other),
        }
    }
}
