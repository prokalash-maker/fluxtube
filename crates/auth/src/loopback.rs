use crate::error::AuthError;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use url::Url;

/// Loopback HTTP listener for capturing OAuth 2.0 authorization codes on `127.0.0.1:0`.
pub struct LoopbackListener {
    listener: TcpListener,
    port: u16,
}

impl LoopbackListener {
    /// Binds an ephemeral loopback server on `127.0.0.1:0`.
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

    /// Waits for incoming authorization callback, verifying state, redirect path, cancellation, and timeout.
    pub async fn wait_for_callback(
        self,
        expected_state: &str,
        timeout: Duration,
    ) -> Result<String, AuthError> {
        self.wait_for_callback_cancellable(expected_state, timeout, CancellationToken::new())
            .await
    }

    /// Cancellable variant used by desktop UI so pending browser auth can be aborted immediately.
    pub async fn wait_for_callback_cancellable(
        self,
        expected_state: &str,
        timeout: Duration,
        cancel: CancellationToken,
    ) -> Result<String, AuthError> {
        tokio::select! {
            _ = cancel.cancelled() => Err(AuthError::Cancelled),
            res = tokio::time::timeout(timeout, self.handle_callback(expected_state)) => {
                res.map_err(|_| AuthError::LoopbackTimeout)?
            }
        }
    }

    async fn handle_callback(self, expected_state: &str) -> Result<String, AuthError> {
        loop {
            let (mut socket, _) = self.listener.accept().await?;

            let mut buffer = [0u8; 4096];
            let bytes_read = socket.read(&mut buffer).await?;
            if bytes_read == 0 {
                continue;
            }
            let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

            let first_line = request_str
                .lines()
                .next()
                .ok_or_else(|| AuthError::InvalidTokenResponse("Empty HTTP request".to_string()))?;

            let mut parts = first_line.split_whitespace();
            let _method = parts.next();
            let raw_path = parts.next().unwrap_or("/");

            let dummy_url = format!("http://127.0.0.1:{}", self.port);
            let parsed_url = Url::parse(&format!("{}{}", dummy_url, raw_path))?;

            // Ignore browser favicon/preflight requests so the actual callback still succeeds
            if parsed_url.path() == "/favicon.ico" {
                let not_found = "HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n";
                let _ = socket.write_all(not_found.as_bytes()).await;
                continue;
            }

            if parsed_url.path() != "/callback" {
                let bad_path = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain; charset=utf-8\r\nConnection: close\r\n\r\nInvalid OAuth redirect path.";
                let _ = socket.write_all(bad_path.as_bytes()).await;
                return Err(AuthError::RedirectUriMismatch(format!(
                    "Unexpected callback path '{}' (expected '/callback')",
                    parsed_url.path()
                )));
            }

            let mut code = None;
            let mut state = None;
            let mut error = None;
            let mut error_description = None;

            for (k, v) in parsed_url.query_pairs() {
                match k.as_ref() {
                    "code" => code = Some(v.into_owned()),
                    "state" => state = Some(v.into_owned()),
                    "error" => error = Some(v.into_owned()),
                    "error_description" => error_description = Some(v.into_owned()),
                    _ => {}
                }
            }

            if let Some(err) = error {
                let detail = error_description.unwrap_or_else(|| err.clone());
                let error_html = format!(
                    "HTTP/1.1 400 Bad Request\r\n\
                     Content-Type: text/html; charset=utf-8\r\n\
                     Connection: close\r\n\r\n\
                     <!DOCTYPE html><html><body style=\"font-family:system-ui;text-align:center;padding:50px;background:#18181b;color:#f87171;\">\
                     <h2>Authorization Failed</h2><p>{}: {}</p></body></html>",
                    err, detail
                );
                let _ = socket.write_all(error_html.as_bytes()).await;
                let _ = socket.flush().await;
                return Err(if err == "redirect_uri_mismatch" {
                    AuthError::RedirectUriMismatch(detail)
                } else {
                    AuthError::AccessDenied(format!("{}: {}", err, detail))
                });
            }

            match state {
                Some(s) if s == expected_state => {}
                _ => {
                    let mismatch_html =
                        "HTTP/1.1 400 Bad Request\r\n\
                         Content-Type: text/html; charset=utf-8\r\n\
                         Connection: close\r\n\r\n\
                         <!DOCTYPE html><html><body style=\"font-family:system-ui;text-align:center;padding:50px;background:#18181b;color:#f87171;\">\
                         <h2>Security Error</h2><p>CSRF state token mismatch. Authorization aborted.</p></body></html>";
                    let _ = socket.write_all(mismatch_html.as_bytes()).await;
                    let _ = socket.flush().await;
                    return Err(AuthError::CsrfMismatch);
                }
            }

            let auth_code = code.ok_or_else(|| {
                AuthError::InvalidTokenResponse(
                    "Missing code parameter in authorization callback".to_string(),
                )
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

            return Ok(auth_code);
        }
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
        let state = "random_csrf_state_token_123";

        let server_task = tokio::spawn(async move {
            listener
                .wait_for_callback(state, Duration::from_secs(5))
                .await
        });

        let client = reqwest::Client::new();
        let resp = client
            .get(format!(
                "http://127.0.0.1:{}/callback?code=mock_auth_code_xyz&state={}",
                port, state
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let code = server_task.await.unwrap().unwrap();
        assert_eq!(code, "mock_auth_code_xyz");
    }

    #[tokio::test]
    async fn test_loopback_csrf_mismatch() {
        let listener = LoopbackListener::bind().await.unwrap();
        let port = listener.port();
        let expected_state = "valid_state";

        let server_task = tokio::spawn(async move {
            listener
                .wait_for_callback(expected_state, Duration::from_secs(5))
                .await
        });

        let client = reqwest::Client::new();
        let resp = client
            .get(format!(
                "http://127.0.0.1:{}/callback?code=mock_code&state=tampered_state",
                port
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), 400);
        let err = server_task.await.unwrap().unwrap_err();
        assert!(matches!(err, AuthError::CsrfMismatch));
    }

    #[tokio::test]
    async fn test_loopback_access_denied_and_redirect_mismatch() {
        let listener = LoopbackListener::bind().await.unwrap();
        let port = listener.port();
        let server_task = tokio::spawn(async move {
            listener
                .wait_for_callback("st", Duration::from_secs(5))
                .await
        });
        let client = reqwest::Client::new();
        let resp = client
            .get(format!(
                "http://127.0.0.1:{}/callback?error=access_denied&error_description=User%20cancelled",
                port
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 400);
        assert!(matches!(
            server_task.await.unwrap().unwrap_err(),
            AuthError::AccessDenied(_)
        ));
    }

    #[tokio::test]
    async fn test_loopback_cancellation() {
        let listener = LoopbackListener::bind().await.unwrap();
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(40)).await;
            cancel_clone.cancel();
        });
        let err = listener
            .wait_for_callback_cancellable("s", Duration::from_secs(10), cancel)
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::Cancelled));
    }

    #[tokio::test]
    async fn test_loopback_timeout() {
        let listener = LoopbackListener::bind().await.unwrap();
        let res = listener
            .wait_for_callback("state_x", Duration::from_millis(50))
            .await;
        assert!(matches!(res, Err(AuthError::LoopbackTimeout)));
    }
}
