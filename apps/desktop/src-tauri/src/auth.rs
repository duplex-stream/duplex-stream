//! WorkOS authentication module
//!
//! Implements authentication flows for CLI and desktop via WorkOS AuthKit:
//! - Device code flow for CLI authentication
//! - PKCE OAuth flow for desktop authentication

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

use crate::config::{save_credentials, Credentials, SecureTokenStorage};
use crate::oauth::{LoopbackServer, OAuthError, PkceChallenge};

/// WorkOS API base URL
const WORKOS_API_URL: &str = "https://api.workos.com";

/// Default WorkOS client ID - can be overridden by env var
const DEFAULT_CLIENT_ID: &str = ""; // Set this to your WorkOS client ID

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Device code flow expired")]
    DeviceCodeExpired,
    #[error("Authorization pending")]
    AuthorizationPending,
    #[error("Authorization denied")]
    AuthorizationDenied,
    #[error("API error: {0}")]
    Api(String),
    #[error("Config error: {0}")]
    Config(#[from] crate::config::ConfigError),
    #[error("WorkOS client ID not configured")]
    ClientIdNotConfigured,
    #[error("OAuth error: {0}")]
    OAuth(#[from] OAuthError),
    #[error("OAuth flow not started")]
    OAuthNotStarted,
}

/// Response from the device authorization endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// User info from WorkOS
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkOSUser {
    pub id: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Token response from WorkOS authentication
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: WorkOSUser,
    #[serde(default)]
    pub organization_id: Option<String>,
}

/// Error response from WorkOS
#[derive(Debug, Deserialize)]
struct WorkOSError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

/// Get the WorkOS client ID from environment or default
pub fn get_client_id() -> Result<String, AuthError> {
    // First try environment variable
    if let Ok(client_id) = std::env::var("WORKOS_CLIENT_ID") {
        if !client_id.is_empty() {
            return Ok(client_id);
        }
    }

    // Fall back to compiled-in default
    if !DEFAULT_CLIENT_ID.is_empty() {
        return Ok(DEFAULT_CLIENT_ID.to_string());
    }

    Err(AuthError::ClientIdNotConfigured)
}

/// Start the device code authorization flow
pub async fn start_device_flow(client_id: &str) -> Result<DeviceCodeResponse, AuthError> {
    let client = Client::new();

    let response = client
        .post(format!("{}/user_management/authorize/device", WORKOS_API_URL))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("client_id={}", client_id))
        .send()
        .await?;

    if !response.status().is_success() {
        let error: WorkOSError = response.json().await?;
        return Err(AuthError::Api(format!(
            "{}: {}",
            error.error,
            error.error_description.unwrap_or_default()
        )));
    }

    let device_response: DeviceCodeResponse = response.json().await?;
    Ok(device_response)
}

/// Poll for authentication completion
pub async fn poll_for_token(
    client_id: &str,
    device_code: &str,
    interval: u64,
    timeout: Duration,
) -> Result<TokenResponse, AuthError> {
    let client = Client::new();
    let start = std::time::Instant::now();

    loop {
        // Check for timeout
        if start.elapsed() >= timeout {
            return Err(AuthError::DeviceCodeExpired);
        }

        // Wait the specified interval before polling
        tokio::time::sleep(Duration::from_secs(interval)).await;

        let response = client
            .post(format!("{}/user_management/authenticate", WORKOS_API_URL))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!(
                "client_id={}&grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code={}",
                client_id, device_code
            ))
            .send()
            .await?;

        if response.status().is_success() {
            let token_response: TokenResponse = response.json().await?;
            return Ok(token_response);
        }

        // Check error type
        let error: WorkOSError = response.json().await?;
        match error.error.as_str() {
            "authorization_pending" => {
                // User hasn't completed auth yet, continue polling
                continue;
            }
            "slow_down" => {
                // We're polling too fast, increase interval
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            "expired_token" => {
                return Err(AuthError::DeviceCodeExpired);
            }
            "access_denied" => {
                return Err(AuthError::AuthorizationDenied);
            }
            _ => {
                return Err(AuthError::Api(format!(
                    "{}: {}",
                    error.error,
                    error.error_description.unwrap_or_default()
                )));
            }
        }
    }
}

/// Refresh an access token using a refresh token
pub async fn refresh_token(client_id: &str, refresh_token: &str) -> Result<TokenResponse, AuthError> {
    let client = Client::new();

    let response = client
        .post(format!("{}/user_management/authenticate", WORKOS_API_URL))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "client_id={}&grant_type=refresh_token&refresh_token={}",
            client_id, refresh_token
        ))
        .send()
        .await?;

    if !response.status().is_success() {
        let error: WorkOSError = response.json().await?;
        return Err(AuthError::Api(format!(
            "{}: {}",
            error.error,
            error.error_description.unwrap_or_default()
        )));
    }

    let token_response: TokenResponse = response.json().await?;
    Ok(token_response)
}

/// Convert a TokenResponse to Credentials and save
pub fn save_token_as_credentials(token: &TokenResponse) -> Result<(), AuthError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let credentials = Credentials {
        access_token: token.access_token.clone(),
        refresh_token: token.refresh_token.clone(),
        expires_at: now + token.expires_in,
        user_id: token.user.id.clone(),
        email: token.user.email.clone(),
        org_id: token.organization_id.clone(),
    };

    save_credentials(&credentials)?;
    Ok(())
}

/// Run the complete login flow
pub async fn login() -> Result<(), AuthError> {
    let client_id = get_client_id()?;

    // Start device flow
    println!("Initiating device code flow...\n");
    let device_response = start_device_flow(&client_id).await?;

    // Display instructions to user
    println!("To authenticate, visit:");
    println!("  {}\n", device_response.verification_uri_complete);
    println!("Or go to {} and enter code:", device_response.verification_uri);
    println!("  {}\n", device_response.user_code);
    println!("Waiting for authentication (expires in {}s)...", device_response.expires_in);

    // Poll for completion
    let timeout = Duration::from_secs(device_response.expires_in);
    let token = poll_for_token(
        &client_id,
        &device_response.device_code,
        device_response.interval,
        timeout,
    )
    .await?;

    // Save credentials
    save_token_as_credentials(&token)?;

    println!("\nSuccessfully logged in as {}", token.user.email.unwrap_or_else(|| token.user.id.clone()));
    if let Some(org_id) = &token.organization_id {
        println!("Organization: {}", org_id);
    }

    Ok(())
}

/// Logout by deleting credentials
pub fn logout() -> Result<(), AuthError> {
    crate::config::delete_credentials()?;
    println!("Logged out successfully");
    Ok(())
}

/// Check and display auth status
pub fn status() -> Result<(), AuthError> {
    match crate::config::load_credentials() {
        Ok(credentials) => {
            println!("Logged in as: {}", credentials.user_id);
            if let Some(email) = &credentials.email {
                println!("Email: {}", email);
            }
            if let Some(org_id) = &credentials.org_id {
                println!("Organization: {}", org_id);
            }
            if credentials.is_expired() {
                println!("Status: Token expired (refresh on next sync)");
            } else {
                let remaining = credentials.expires_at.saturating_sub(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );
                println!("Status: Authenticated (expires in {}s)", remaining);
            }
            Ok(())
        }
        Err(crate::config::ConfigError::NotAuthenticated) => {
            println!("Not logged in");
            println!("Run 'duplex auth login' to authenticate");
            Ok(())
        }
        Err(e) => Err(AuthError::Config(e)),
    }
}

/// Get a valid access token, refreshing if needed
/// First checks credentials.json, then falls back to simple .token file
pub async fn get_valid_token() -> Result<String, AuthError> {
    // Try loading full credentials (has expiry/refresh capability)
    match crate::config::load_credentials() {
        Ok(credentials) => {
            if !credentials.is_expired() {
                return Ok(credentials.access_token);
            }

            // Token expired, try to refresh
            tracing::info!("Access token expired, refreshing...");
            let client_id = get_client_id()?;
            let token = refresh_token(&client_id, &credentials.refresh_token).await?;

            // Save updated credentials
            save_token_as_credentials(&token)?;

            return Ok(token.access_token);
        }
        Err(crate::config::ConfigError::NotAuthenticated) => {
            // No credentials.json, fall through to check token file
        }
        Err(e) => {
            return Err(AuthError::Config(e));
        }
    }

    // Fall back to simple token file (from desktop auth flow)
    // This token doesn't have refresh capability, but it's better than nothing
    match crate::config::get_access_token() {
        Ok(token) => {
            tracing::debug!("Using token from simple token file");
            Ok(token)
        }
        Err(e) => Err(AuthError::Config(e)),
    }
}

// ============================================================================
// Desktop OAuth Flow (PKCE)
// ============================================================================

/// Desktop OAuth flow using PKCE and loopback server
///
/// This implements a secure OAuth 2.0 Authorization Code flow with PKCE,
/// using a local loopback server to receive the callback.
pub struct DesktopOAuthFlow {
    /// PKCE challenge for this flow
    pkce: PkceChallenge,
    /// Loopback server for receiving the callback
    server: Option<LoopbackServer>,
    /// The authorization URL to open in the browser
    auth_url: Option<String>,
    /// Secure token storage
    storage: SecureTokenStorage,
}

impl DesktopOAuthFlow {
    /// Create a new desktop OAuth flow
    pub fn new() -> Self {
        Self {
            pkce: PkceChallenge::generate(),
            server: None,
            auth_url: None,
            storage: SecureTokenStorage::new(),
        }
    }

    /// Start the OAuth flow
    ///
    /// This starts the loopback server and generates the authorization URL.
    /// Call `get_auth_url()` to get the URL to open in the browser.
    pub async fn start(&mut self) -> Result<(), AuthError> {
        let client_id = get_client_id()?;

        // Start the loopback server
        let server = LoopbackServer::start().await?;
        let redirect_uri = server.redirect_uri();

        // Build the authorization URL
        // WorkOS uses /user_management/authorize for OAuth flows
        let auth_url = format!(
            "{}/user_management/authorize?client_id={}&redirect_uri={}&response_type=code&code_challenge={}&code_challenge_method=S256",
            WORKOS_API_URL,
            urlencoding::encode(&client_id),
            urlencoding::encode(&redirect_uri),
            urlencoding::encode(&self.pkce.challenge),
        );

        self.auth_url = Some(auth_url);
        self.server = Some(server);

        tracing::info!("OAuth flow started, waiting for callback on loopback server");
        Ok(())
    }

    /// Get the authorization URL to open in the browser
    pub fn get_auth_url(&self) -> Option<&str> {
        self.auth_url.as_deref()
    }

    /// Complete the OAuth flow
    ///
    /// This waits for the callback, exchanges the code for tokens,
    /// and stores them in the keyring.
    pub async fn complete(self) -> Result<TokenResponse, AuthError> {
        let server = self.server.ok_or(AuthError::OAuthNotStarted)?;

        // Wait for the callback
        let callback = server.wait_for_callback().await?;
        tracing::info!("Received authorization code from callback");

        // Exchange the code for tokens
        let client_id = get_client_id()?;
        let token = exchange_code_for_token(
            &client_id,
            &callback.code,
            &self.pkce.verifier,
        ).await?;

        // Store tokens in keyring
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = now + token.expires_in;

        self.storage.store_tokens(
            token.access_token.clone(),
            token.refresh_token.clone(),
            expires_at,
        )?;

        tracing::info!("OAuth flow completed successfully");
        Ok(token)
    }
}

impl Default for DesktopOAuthFlow {
    fn default() -> Self {
        Self::new()
    }
}

/// Exchange an authorization code for tokens using PKCE
async fn exchange_code_for_token(
    client_id: &str,
    code: &str,
    code_verifier: &str,
) -> Result<TokenResponse, AuthError> {
    let client = Client::new();

    let response = client
        .post(format!("{}/user_management/authenticate", WORKOS_API_URL))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!(
            "client_id={}&grant_type=authorization_code&code={}&code_verifier={}",
            urlencoding::encode(client_id),
            urlencoding::encode(code),
            urlencoding::encode(code_verifier),
        ))
        .send()
        .await?;

    if !response.status().is_success() {
        let error: WorkOSError = response.json().await?;
        return Err(AuthError::Api(format!(
            "{}: {}",
            error.error,
            error.error_description.unwrap_or_default()
        )));
    }

    let token_response: TokenResponse = response.json().await?;
    Ok(token_response)
}

/// Run the complete desktop OAuth login flow
///
/// This is a convenience function that starts the flow, opens the browser,
/// waits for completion, and returns the result.
pub async fn desktop_login() -> Result<TokenResponse, AuthError> {
    let mut flow = DesktopOAuthFlow::new();

    // Start the flow
    flow.start().await?;

    // Get the auth URL
    let auth_url = flow.get_auth_url().ok_or(AuthError::OAuthNotStarted)?;
    tracing::info!("Opening browser for authentication...");

    // Open the browser
    open_browser(auth_url)?;

    // Wait for completion
    flow.complete().await
}

/// Open a URL in the default browser
fn open_browser(url: &str) -> Result<(), AuthError> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| AuthError::Api(format!("Failed to open browser: {}", e)))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| AuthError::Api(format!("Failed to open browser: {}", e)))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .map_err(|e| AuthError::Api(format!("Failed to open browser: {}", e)))?;
    }

    Ok(())
}
