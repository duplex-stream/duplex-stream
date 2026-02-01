mod claude_code;

pub use claude_code::ClaudeCodeParser;

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Unsupported file format")]
    UnsupportedFormat,
}

/// Represents a discovered conversation file
#[derive(Debug, Clone)]
pub struct ConversationFile {
    /// Path to the conversation file
    pub path: PathBuf,
    /// Session ID if available
    pub session_id: Option<String>,
    /// Project path this conversation belongs to
    pub project_path: Option<PathBuf>,
}

/// Represents a parsed conversation ready for sync
#[derive(Debug, Clone)]
pub struct Conversation {
    /// Source file path
    pub source_path: PathBuf,
    /// Source type (e.g., "claude-code")
    pub source: String,
    /// Session ID if available
    pub session_id: Option<String>,
    /// Project path this conversation belongs to
    pub project_path: Option<PathBuf>,
    /// Raw content to upload
    pub content: String,
}

/// Trait for conversation parsers
pub trait ConversationParser: Send + Sync {
    /// Parser name (e.g., "claude-code")
    fn name(&self) -> &str;

    /// Check if this parser can handle the given directory
    fn detect(&self, path: &Path) -> bool;

    /// Discover all conversation files in the given directory
    fn discover(&self, path: &Path) -> Vec<ConversationFile>;

    /// Parse a conversation file
    fn parse(&self, file: &Path) -> Result<Conversation, ParserError>;

    /// Glob patterns to watch for changes (e.g., ["*.jsonl"])
    fn watch_patterns(&self) -> Vec<&str>;
}

/// Registry of available parsers
pub struct ParserRegistry {
    parsers: Vec<Box<dyn ConversationParser>>,
}

impl ParserRegistry {
    /// Create a new registry with default parsers
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: Vec::new(),
        };

        // Register built-in parsers
        registry.register(Box::new(ClaudeCodeParser::new()));

        registry
    }

    /// Register a new parser
    pub fn register(&mut self, parser: Box<dyn ConversationParser>) {
        tracing::debug!("Registered parser: {}", parser.name());
        self.parsers.push(parser);
    }

    /// Get a parser by name
    pub fn get(&self, name: &str) -> Option<&dyn ConversationParser> {
        self.parsers
            .iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }

    /// Get all registered parsers
    pub fn all(&self) -> impl Iterator<Item = &dyn ConversationParser> {
        self.parsers.iter().map(|p| p.as_ref())
    }

    /// Find a parser that can handle the given path
    pub fn detect(&self, path: &Path) -> Option<&dyn ConversationParser> {
        self.parsers.iter().find(|p| p.detect(path)).map(|p| p.as_ref())
    }

    /// Get enabled parsers based on config
    pub fn get_enabled(&self, enabled_names: &[String]) -> Vec<&dyn ConversationParser> {
        enabled_names
            .iter()
            .filter_map(|name| self.get(name))
            .collect()
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
