use reqwest::Client;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use thiserror::Error;

use crate::auth;
use crate::db::{Database, SyncState, SyncStatus};
use crate::parsers::{Conversation, ConversationParser, ParserRegistry};
use crate::watcher::FileChangeEvent;

#[derive(Error, Debug)]
pub enum SyncError {
    #[error("Database error: {0}")]
    Database(#[from] crate::db::DatabaseError),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Parser error: {0}")]
    Parser(#[from] crate::parsers::ParserError),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No parser found for: {0}")]
    NoParser(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Authentication error: {0}")]
    Auth(#[from] crate::auth::AuthError),
    #[error("Not authenticated - run 'duplex auth login'")]
    NotAuthenticated,
}

/// Item in the sync queue
#[derive(Debug, Clone)]
pub struct SyncItem {
    pub path: PathBuf,
    pub parser_name: String,
    pub content_hash: String,
}

/// Response from the extraction API
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionResponse {
    pub workflow_id: String,
    pub status: String,
}

/// Engine that manages syncing conversations to the API
pub struct SyncEngine {
    /// HTTP client for API requests
    client: Client,
    /// API base URL
    api_url: String,
    /// Access token for authentication
    access_token: Option<String>,
    /// Queue of items to sync
    queue: VecDeque<SyncItem>,
    /// Database for sync state
    db: Database,
    /// Parser registry
    registry: Arc<ParserRegistry>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(
        api_url: String,
        access_token: Option<String>,
        registry: Arc<ParserRegistry>,
    ) -> Result<Self, SyncError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let db = Database::open()?;

        Ok(Self {
            client,
            api_url,
            access_token,
            queue: VecDeque::new(),
            db,
            registry,
        })
    }

    /// Handle a file change event
    pub fn handle_file_change(&mut self, event: FileChangeEvent) -> Result<(), SyncError> {
        let path = &event.path;

        // Read file content
        let content = std::fs::read_to_string(path)?;

        // Compute content hash
        let content_hash = compute_hash(&content);

        // Check if we need to sync (content changed since last sync)
        if let Some(existing) = self.db.get_sync_state(&path.to_string_lossy())? {
            if existing.content_hash == content_hash {
                tracing::debug!("File unchanged, skipping: {:?}", path);
                return Ok(());
            }
        }

        // Add to queue
        let item = SyncItem {
            path: path.clone(),
            parser_name: event.parser_name,
            content_hash,
        };

        // Update database with pending status
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.db.upsert_sync_state(&SyncState {
            file_path: path.to_string_lossy().to_string(),
            content_hash: item.content_hash.clone(),
            last_synced_at: None,
            last_modified_at: now,
            workflow_id: None,
            status: SyncStatus::Pending,
        })?;

        self.queue.push_back(item);
        tracing::info!("Queued for sync: {:?}", path);

        Ok(())
    }

    /// Process the next item in the queue
    pub async fn process_next(&mut self) -> Result<Option<String>, SyncError> {
        let item = match self.queue.pop_front() {
            Some(i) => i,
            None => return Ok(None),
        };

        tracing::info!("Syncing: {:?}", item.path);

        // Mark as syncing
        self.db.mark_syncing(&item.path.to_string_lossy())?;

        // Get parser and parse the file
        let parser = self
            .registry
            .get(&item.parser_name)
            .ok_or_else(|| SyncError::NoParser(item.parser_name.clone()))?;

        let conversation = parser.parse(&item.path)?;

        // Upload to API
        match self.upload_conversation(&conversation).await {
            Ok(response) => {
                self.db
                    .mark_complete(&item.path.to_string_lossy(), &response.workflow_id)?;
                tracing::info!(
                    "Sync complete: {:?} -> workflow {}",
                    item.path,
                    response.workflow_id
                );
                Ok(Some(response.workflow_id))
            }
            Err(e) => {
                self.db
                    .update_status(&item.path.to_string_lossy(), SyncStatus::Error)?;
                tracing::error!("Sync failed: {:?} - {}", item.path, e);
                Err(e)
            }
        }
    }

    /// Get a valid access token, with auto-refresh
    async fn get_token(&self) -> Result<Option<String>, SyncError> {
        // First try to get a valid token from auth system (with auto-refresh)
        match auth::get_valid_token().await {
            Ok(token) => return Ok(Some(token)),
            Err(auth::AuthError::Config(crate::config::ConfigError::NotAuthenticated)) => {
                // Not logged in - fall back to initial token if provided
            }
            Err(auth::AuthError::ClientIdNotConfigured) => {
                // WorkOS not configured - fall back to initial token
                tracing::debug!("WorkOS client ID not configured, using fallback token");
            }
            Err(e) => {
                // Other auth errors (e.g., refresh failed)
                tracing::warn!("Failed to get valid token: {}", e);
            }
        }

        // Fall back to the initial token passed at construction
        Ok(self.access_token.clone())
    }

    /// Upload a conversation to the API
    async fn upload_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<ExtractionResponse, SyncError> {
        let url = format!("{}/extraction/conversations/extract", self.api_url);

        let mut request = self.client.post(&url).json(&serde_json::json!({
            "content": conversation.content,
            "sourcePath": conversation.source_path.to_string_lossy(),
            "source": conversation.source,
            "workspaceId": "default",
        }));

        // Add auth header if available (with auto-refresh)
        if let Some(token) = self.get_token().await? {
            request = request.bearer_auth(token);
        } else {
            tracing::warn!("No authentication token available, request may fail");
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Provide helpful message for auth errors
            if status.as_u16() == 401 {
                return Err(SyncError::NotAuthenticated);
            }

            return Err(SyncError::Api(format!("{}: {}", status, body)));
        }

        let extraction_response: ExtractionResponse = response.json().await?;
        Ok(extraction_response)
    }

    /// Process all items in the queue
    pub async fn process_all(&mut self) -> Result<usize, SyncError> {
        let mut count = 0;
        while !self.queue.is_empty() {
            match self.process_next().await {
                Ok(Some(_)) => count += 1,
                Ok(None) => break,
                Err(e) => {
                    tracing::error!("Error processing sync item: {}", e);
                    // Continue with next item
                }
            }
        }
        Ok(count)
    }

    /// Get the number of items in the queue
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Get sync status counts from the database
    pub fn get_status_counts(&self) -> Result<crate::db::StatusCounts, SyncError> {
        Ok(self.db.get_status_counts()?)
    }
}

/// Compute SHA-256 hash of content
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Shared sync engine wrapped in Arc<Mutex>
pub type SharedSyncEngine = Arc<Mutex<SyncEngine>>;

/// Create a shared sync engine
pub fn create_shared_engine(
    api_url: String,
    access_token: Option<String>,
    registry: Arc<ParserRegistry>,
) -> Result<SharedSyncEngine, SyncError> {
    let engine = SyncEngine::new(api_url, access_token, registry)?;
    Ok(Arc::new(Mutex::new(engine)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash("hello world");
        let hash2 = compute_hash("hello world");
        let hash3 = compute_hash("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex chars
    }
}
