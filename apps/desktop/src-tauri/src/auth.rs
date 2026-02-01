//! WorkOS authentication module
//!
//! Implements the device code flow for CLI authentication via WorkOS AuthKit.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

use crate::config::{save_credentials, Credentials};

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
pub async fn get_valid_token() -> Result<String, AuthError> {
    let credentials = crate::config::load_credentials()?;

    if !credentials.is_expired() {
        return Ok(credentials.access_token);
    }

    // Token expired, try to refresh
    tracing::info!("Access token expired, refreshing...");
    let client_id = get_client_id()?;
    let token = refresh_token(&client_id, &credentials.refresh_token).await?;

    // Save updated credentials
    save_token_as_credentials(&token)?;

    Ok(token.access_token)
}
