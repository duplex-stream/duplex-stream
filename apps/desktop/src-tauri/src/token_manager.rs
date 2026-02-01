//! Token Manager module for background token refresh
//!
//! Manages access token lifecycle, automatically refreshing tokens before they expire.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::auth::{get_client_id, refresh_token, AuthError};
use crate::config::SecureTokenStorage;

/// Interval for checking token expiry (30 seconds)
const CHECK_INTERVAL_SECS: u64 = 30;

/// Refresh token this many seconds before expiration
const REFRESH_BUFFER_SECS: u64 = 60;

/// Token Manager state
pub struct TokenManager {
    storage: SecureTokenStorage,
    /// Whether the manager is running
    running: Arc<RwLock<bool>>,
}

impl TokenManager {
    /// Create a new TokenManager
    pub fn new() -> Self {
        Self {
            storage: SecureTokenStorage::new(),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get the current access token if available and valid
    pub fn get_access_token(&self) -> Option<String> {
        self.storage.get_tokens().ok().map(|t| t.access_token)
    }

    /// Check if we have valid tokens
    pub fn is_authenticated(&self) -> bool {
        self.storage.get_tokens().is_ok()
    }

    /// Store new tokens
    pub fn store_tokens(&self, access_token: String, refresh_token: String, expires_at: u64) -> Result<(), crate::config::ConfigError> {
        self.storage.store_tokens(access_token, refresh_token, expires_at)
    }

    /// Clear all tokens (logout)
    pub fn clear_tokens(&self) -> Result<(), crate::config::ConfigError> {
        self.storage.clear_tokens()
    }

    /// Start the background refresh task
    ///
    /// This spawns a tokio task that periodically checks token expiry
    /// and refreshes tokens before they expire.
    pub fn start_background_refresh(&self) -> tokio::task::JoinHandle<()> {
        let storage = self.storage.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            // Mark as running
            {
                let mut r = running.write().await;
                *r = true;
            }

            let mut check_interval = interval(Duration::from_secs(CHECK_INTERVAL_SECS));

            loop {
                check_interval.tick().await;

                // Check if we should stop
                {
                    let r = running.read().await;
                    if !*r {
                        tracing::info!("Token manager stopping");
                        break;
                    }
                }

                // Check if we have tokens and need to refresh
                match storage.get_tokens() {
                    Ok(token_data) => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        // Check if token will expire within the buffer period
                        if token_data.expires_at <= now + REFRESH_BUFFER_SECS {
                            tracing::info!("Token expiring soon, refreshing...");

                            match Self::do_refresh(&storage, &token_data.refresh_token).await {
                                Ok(()) => {
                                    tracing::info!("Token refreshed successfully");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to refresh token: {}", e);
                                    // Don't clear tokens on refresh failure - they might still work
                                    // or the user might want to try again
                                }
                            }
                        } else {
                            let remaining = token_data.expires_at - now;
                            tracing::debug!(
                                "Token still valid for {} seconds",
                                remaining
                            );
                        }
                    }
                    Err(e) => {
                        tracing::debug!("No tokens to refresh: {}", e);
                    }
                }
            }
        })
    }

    /// Stop the background refresh task
    pub async fn stop(&self) {
        let mut r = self.running.write().await;
        *r = false;
    }

    /// Perform a token refresh
    async fn do_refresh(storage: &SecureTokenStorage, refresh_token_str: &str) -> Result<(), AuthError> {
        let client_id = get_client_id()?;

        let token_response = refresh_token(&client_id, refresh_token_str).await?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expires_at = now + token_response.expires_in;

        storage.store_tokens(
            token_response.access_token,
            token_response.refresh_token,
            expires_at,
        ).map_err(|e| AuthError::Config(e))?;

        Ok(())
    }
}

impl Default for TokenManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TokenManager {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            running: self.running.clone(),
        }
    }
}

/// Shared token manager type for use across the application
pub type SharedTokenManager = Arc<TokenManager>;

/// Create a new shared token manager
pub fn create_shared_manager() -> SharedTokenManager {
    Arc::new(TokenManager::new())
}
