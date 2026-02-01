use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind, Debouncer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

use crate::parsers::{ConversationParser, ParserRegistry};

#[derive(Error, Debug)]
pub enum WatcherError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),
}

/// Event emitted when a file is ready to sync
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Path to the changed file
    pub path: PathBuf,
    /// Name of the parser that handles this file
    pub parser_name: String,
}

/// Manages file watching for conversation files
pub struct FileWatcher {
    /// The debouncer that wraps the watcher
    debouncer: Debouncer<RecommendedWatcher>,
    /// Map of watched directories to their parser names
    watched_dirs: Arc<Mutex<HashMap<PathBuf, String>>>,
    /// Receiver for file change events
    event_rx: Receiver<FileChangeEvent>,
    /// Sender for file change events (kept for internal use)
    _event_tx: Sender<FileChangeEvent>,
}

impl FileWatcher {
    /// Create a new file watcher with the given debounce duration
    pub fn new(debounce_duration: Duration) -> Result<Self, WatcherError> {
        let (event_tx, event_rx) = channel();
        let watched_dirs: Arc<Mutex<HashMap<PathBuf, String>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let watched_dirs_clone = watched_dirs.clone();
        let event_tx_clone = event_tx.clone();

        // Create the debouncer with our event handler
        let debouncer = new_debouncer(
            debounce_duration,
            move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                match res {
                    Ok(events) => {
                        for event in events {
                            if event.kind == DebouncedEventKind::Any {
                                let path = &event.path;

                                // Check if this file is in a watched directory
                                if let Some(parser_name) =
                                    find_parser_for_path(path, &watched_dirs_clone)
                                {
                                    // Only care about .jsonl files for now
                                    if path.extension().map_or(false, |e| e == "jsonl") {
                                        let event = FileChangeEvent {
                                            path: path.clone(),
                                            parser_name,
                                        };

                                        if let Err(e) = event_tx_clone.send(event) {
                                            tracing::error!("Failed to send file change event: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Watch error: {:?}", e);
                    }
                }
            },
        )?;

        Ok(Self {
            debouncer,
            watched_dirs,
            event_rx,
            _event_tx: event_tx,
        })
    }

    /// Watch a directory with the given parser
    pub fn watch(&mut self, path: &Path, parser_name: &str) -> Result<(), WatcherError> {
        if !path.exists() {
            return Err(WatcherError::PathNotFound(path.to_path_buf()));
        }

        // Add to watcher
        self.debouncer
            .watcher()
            .watch(path, RecursiveMode::Recursive)?;

        // Track the directory and its parser
        let mut dirs = self.watched_dirs.lock().unwrap();
        dirs.insert(path.to_path_buf(), parser_name.to_string());

        tracing::info!("Watching {:?} with parser '{}'", path, parser_name);
        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: &Path) -> Result<(), WatcherError> {
        self.debouncer.watcher().unwatch(path)?;

        let mut dirs = self.watched_dirs.lock().unwrap();
        dirs.remove(path);

        tracing::info!("Stopped watching {:?}", path);
        Ok(())
    }

    /// Get the number of watched directories
    pub fn watched_count(&self) -> usize {
        self.watched_dirs.lock().unwrap().len()
    }

    /// Get the receiver for file change events
    pub fn events(&self) -> &Receiver<FileChangeEvent> {
        &self.event_rx
    }

    /// Try to receive a file change event (non-blocking)
    pub fn try_recv(&self) -> Option<FileChangeEvent> {
        self.event_rx.try_recv().ok()
    }
}

/// Find the parser name for a given file path
fn find_parser_for_path(path: &Path, watched_dirs: &Arc<Mutex<HashMap<PathBuf, String>>>) -> Option<String> {
    let dirs = watched_dirs.lock().unwrap();

    for (watched_path, parser_name) in dirs.iter() {
        if path.starts_with(watched_path) {
            return Some(parser_name.clone());
        }
    }

    None
}

/// Discover and watch all known conversation directories
pub fn discover_and_watch(
    watcher: &mut FileWatcher,
    registry: &ParserRegistry,
    config: &crate::config::Config,
) -> Result<usize, WatcherError> {
    let mut count = 0;

    // Auto-discover known locations if enabled
    if config.discovery.auto_discover {
        // Claude Code projects directory
        if let Some(claude_projects) = crate::parsers::ClaudeCodeParser::default_projects_dir() {
            if claude_projects.exists() {
                if let Some(parser) = registry.get("claude-code") {
                    watcher.watch(&claude_projects, parser.name())?;
                    count += 1;
                }
            } else {
                tracing::debug!("Claude Code projects directory not found: {:?}", claude_projects);
            }
        }
    }

    // Watch additional configured paths
    for path_str in &config.discovery.additional_paths {
        let path = expand_path(path_str);
        if path.exists() {
            // Try to detect which parser to use
            if let Some(parser) = registry.detect(&path) {
                watcher.watch(&path, parser.name())?;
                count += 1;
            } else {
                tracing::warn!("No parser found for path: {:?}", path);
            }
        } else {
            tracing::warn!("Configured path does not exist: {:?}", path);
        }
    }

    tracing::info!("Discovered and watching {} directories", count);
    Ok(count)
}

/// Expand ~ to home directory
fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_expand_path() {
        let expanded = expand_path("~/test/path");
        assert!(expanded.to_string_lossy().contains("test/path"));
        assert!(!expanded.to_string_lossy().starts_with("~"));

        let absolute = expand_path("/absolute/path");
        assert_eq!(absolute, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new(Duration::from_secs(1));
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_directory() {
        let dir = tempdir().unwrap();
        let mut watcher = FileWatcher::new(Duration::from_secs(1)).unwrap();

        let result = watcher.watch(dir.path(), "test-parser");
        assert!(result.is_ok());
        assert_eq!(watcher.watched_count(), 1);
    }
}
