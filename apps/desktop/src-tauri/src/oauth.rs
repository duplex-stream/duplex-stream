//! PKCE OAuth module for desktop authentication
//!
//! Implements PKCE (Proof Key for Code Exchange) flow with a loopback HTTP server
//! to securely receive the authorization code.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("Failed to bind to loopback address: {0}")]
    BindError(#[from] std::io::Error),
    #[error("Failed to receive authorization code")]
    CodeReceiveError,
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),
    #[error("Server error: {0}")]
    ServerError(String),
}

/// PKCE challenge for OAuth 2.0 authorization
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    /// The code verifier (random string, base64url encoded)
    pub verifier: String,
    /// The code challenge (SHA256 hash of verifier, base64url encoded)
    pub challenge: String,
}

impl PkceChallenge {
    /// Generate a new PKCE challenge pair
    ///
    /// Creates a cryptographically random code verifier (32 bytes, base64url encoded)
    /// and derives the code challenge using SHA256.
    pub fn generate() -> Self {
        // Generate 32 random bytes for the verifier
        let mut verifier_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut verifier_bytes);

        // Base64url encode the verifier (no padding)
        let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

        // Create the challenge: SHA256 hash of verifier, base64url encoded
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge_hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(challenge_hash);

        Self { verifier, challenge }
    }
}

/// Result from the loopback callback server
pub struct CallbackResult {
    /// The authorization code received from the OAuth provider
    pub code: String,
    /// The state parameter (if any) for CSRF verification
    pub state: Option<String>,
}

/// Loopback HTTP server for receiving OAuth callbacks
pub struct LoopbackServer {
    /// The port the server is listening on
    pub port: u16,
    /// Channel to receive the callback result
    result_rx: oneshot::Receiver<Result<CallbackResult, OAuthError>>,
    /// Shutdown signal sender
    _shutdown_tx: oneshot::Sender<()>,
}

impl LoopbackServer {
    /// Start a new loopback server on a random available port
    ///
    /// The server listens for a single callback request at /callback,
    /// extracts the authorization code, and shuts down.
    pub async fn start() -> Result<Self, OAuthError> {
        // Bind to localhost on a random available port
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let listener = TcpListener::bind(addr).await?;
        let port = listener.local_addr()?.port();

        tracing::info!("OAuth callback server listening on 127.0.0.1:{}", port);

        // Create channels for communication
        let (result_tx, result_rx) = oneshot::channel();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

        // Wrap the result sender in Arc for sharing
        let result_tx = Arc::new(tokio::sync::Mutex::new(Some(result_tx)));

        // Spawn the server task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = &mut shutdown_rx => {
                        tracing::debug!("OAuth callback server received shutdown signal");
                        break;
                    }
                    // Accept new connections
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, _)) => {
                                let result_tx = result_tx.clone();
                                let io = TokioIo::new(stream);

                                tokio::spawn(async move {
                                    let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                                        let result_tx = result_tx.clone();
                                        async move {
                                            handle_callback(req, result_tx).await
                                        }
                                    });

                                    if let Err(e) = http1::Builder::new()
                                        .serve_connection(io, service)
                                        .await
                                    {
                                        tracing::error!("Error serving connection: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                tracing::error!("Error accepting connection: {}", e);
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            port,
            result_rx,
            _shutdown_tx: shutdown_tx,
        })
    }

    /// Get the redirect URI for this server
    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }

    /// Wait for the callback and return the authorization code
    ///
    /// This consumes the server, which will shut down after receiving the callback.
    pub async fn wait_for_callback(self) -> Result<CallbackResult, OAuthError> {
        self.result_rx.await.map_err(|_| OAuthError::CodeReceiveError)?
    }
}

/// Handle an incoming callback request
async fn handle_callback(
    req: Request<hyper::body::Incoming>,
    result_tx: Arc<tokio::sync::Mutex<Option<oneshot::Sender<Result<CallbackResult, OAuthError>>>>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path();

    // Only handle /callback path
    if !path.starts_with("/callback") {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap());
    }

    // Parse query parameters
    let query = req.uri().query().unwrap_or("");
    let params: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    // Check for error response
    if let Some(error) = params.get("error") {
        let error_desc = params.get("error_description")
            .map(|s| s.as_str())
            .unwrap_or("Unknown error");

        tracing::error!("OAuth error: {} - {}", error, error_desc);

        // Send error result
        if let Some(tx) = result_tx.lock().await.take() {
            let _ = tx.send(Err(OAuthError::AuthorizationFailed(format!("{}: {}", error, error_desc))));
        }

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html")
            .body(Full::new(Bytes::from(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>Authentication Failed</title></head>
<body style="font-family: system-ui; text-align: center; padding: 50px;">
<h1>Authentication Failed</h1>
<p>{}: {}</p>
<p>You can close this window.</p>
</body>
</html>"#,
                error, error_desc
            ))))
            .unwrap());
    }

    // Extract authorization code
    let code = params.get("code").cloned();
    let state = params.get("state").cloned();

    if let Some(code) = code {
        tracing::info!("Received authorization code");

        // Send success result
        if let Some(tx) = result_tx.lock().await.take() {
            let _ = tx.send(Ok(CallbackResult { code, state }));
        }

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/html")
            .body(Full::new(Bytes::from(
                r#"<!DOCTYPE html>
<html>
<head><title>Authentication Successful</title></head>
<body style="font-family: system-ui; text-align: center; padding: 50px;">
<h1>Authentication Successful!</h1>
<p>You can close this window and return to the app.</p>
<script>window.close();</script>
</body>
</html>"#
            )))
            .unwrap());
    }

    // No code parameter
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "text/html")
        .body(Full::new(Bytes::from(
            r#"<!DOCTYPE html>
<html>
<head><title>Invalid Callback</title></head>
<body style="font-family: system-ui; text-align: center; padding: 50px;">
<h1>Invalid Callback</h1>
<p>No authorization code received.</p>
</body>
</html>"#
        )))
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_challenge_generation() {
        let pkce = PkceChallenge::generate();

        // Verifier should be 43 characters (32 bytes base64url encoded)
        assert_eq!(pkce.verifier.len(), 43);

        // Challenge should be 43 characters (32 bytes SHA256 hash base64url encoded)
        assert_eq!(pkce.challenge.len(), 43);

        // Generate another one to verify randomness
        let pkce2 = PkceChallenge::generate();
        assert_ne!(pkce.verifier, pkce2.verifier);
        assert_ne!(pkce.challenge, pkce2.challenge);
    }

    #[test]
    fn test_pkce_challenge_derivation() {
        // Verify that the challenge is correctly derived from the verifier
        let pkce = PkceChallenge::generate();

        let mut hasher = Sha256::new();
        hasher.update(pkce.verifier.as_bytes());
        let expected_hash = hasher.finalize();
        let expected_challenge = URL_SAFE_NO_PAD.encode(expected_hash);

        assert_eq!(pkce.challenge, expected_challenge);
    }
}
