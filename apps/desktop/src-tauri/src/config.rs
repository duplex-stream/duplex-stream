use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to determine config directory")]
    NoConfigDir,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Not authenticated")]
    NotAuthenticated,
    #[error("Token expired")]
    TokenExpired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub parsers: ParsersConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    #[serde(default = "default_debounce_seconds")]
    pub debounce_seconds: u64,
    #[serde(default = "default_true")]
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryConfig {
    #[serde(default = "default_true")]
    pub auto_discover: bool,
    #[serde(default)]
    pub additional_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsersConfig {
    #[serde(default = "default_enabled_parsers")]
    pub enabled: Vec<String>,
}

fn default_debounce_seconds() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

fn default_enabled_parsers() -> Vec<String> {
    vec!["claude-code".to_string()]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sync: SyncConfig::default(),
            discovery: DiscoveryConfig::default(),
            parsers: ParsersConfig::default(),
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            debounce_seconds: default_debounce_seconds(),
            auto_start: true,
        }
    }
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            auto_discover: true,
            additional_paths: vec![],
        }
    }
}

impl Default for ParsersConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled_parsers(),
        }
    }
}

/// Get the config directory path
pub fn get_config_dir() -> Result<PathBuf, ConfigError> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Use ~/.config/duplex on Linux and macOS
        if let Some(home) = dirs::home_dir() {
            return Ok(home.join(".config").join("duplex"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Use AppData on Windows
        if let Some(config) = dirs::config_dir() {
            return Ok(config.join("duplex"));
        }
    }

    Err(ConfigError::NoConfigDir)
}

/// Get the config file path
pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    Ok(get_config_dir()?.join("config.jsonc"))
}

/// Get the credentials file path
pub fn get_credentials_path() -> Result<PathBuf, ConfigError> {
    Ok(get_config_dir()?.join("credentials.json"))
}

/// Get the database file path
pub fn get_database_path() -> Result<PathBuf, ConfigError> {
    Ok(get_config_dir()?.join("sync.db"))
}

/// Load config from file, creating default if it doesn't exist
pub fn load_config() -> Result<Config, ConfigError> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        // Create config directory and default config
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_config = Config::default();
        let json = serde_json::to_string_pretty(&default_config)?;

        // Add a comment at the top
        let jsonc = format!(
            "// Duplex Stream configuration\n// See https://duplex.app/docs/config for options\n{}",
            json
        );

        std::fs::write(&config_path, jsonc)?;
        tracing::info!("Created default config at {:?}", config_path);

        return Ok(default_config);
    }

    // Read and parse config (strip comments first)
    let content = std::fs::read_to_string(&config_path)?;
    let json = json_comments::StripComments::new(content.as_bytes());
    let config: Config = serde_json::from_reader(json)?;

    tracing::debug!("Loaded config from {:?}", config_path);
    Ok(config)
}

/// Stored authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64, // Unix timestamp
    pub user_id: String,
    pub email: Option<String>,
    pub org_id: Option<String>,
}

impl Credentials {
    /// Check if the access token is expired (with 60s buffer)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.expires_at <= now + 60
    }
}

/// Load credentials from the credentials file
pub fn load_credentials() -> Result<Credentials, ConfigError> {
    let creds_path = get_credentials_path()?;

    if !creds_path.exists() {
        return Err(ConfigError::NotAuthenticated);
    }

    let content = std::fs::read_to_string(&creds_path)?;
    let credentials: Credentials = serde_json::from_str(&content)?;

    tracing::debug!("Loaded credentials for user {}", credentials.user_id);
    Ok(credentials)
}

/// Save credentials to the credentials file
pub fn save_credentials(credentials: &Credentials) -> Result<(), ConfigError> {
    let creds_path = get_credentials_path()?;

    // Ensure the directory exists
    if let Some(parent) = creds_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(credentials)?;
    std::fs::write(&creds_path, json)?;

    tracing::info!("Saved credentials to {:?}", creds_path);
    Ok(())
}

/// Delete the credentials file (logout)
pub fn delete_credentials() -> Result<(), ConfigError> {
    let creds_path = get_credentials_path()?;

    if creds_path.exists() {
        std::fs::remove_file(&creds_path)?;
        tracing::info!("Deleted credentials from {:?}", creds_path);
    }

    Ok(())
}

/// Get a valid access token, checking expiry
pub fn get_access_token() -> Result<String, ConfigError> {
    let credentials = load_credentials()?;

    if credentials.is_expired() {
        return Err(ConfigError::TokenExpired);
    }

    Ok(credentials.access_token)
}
