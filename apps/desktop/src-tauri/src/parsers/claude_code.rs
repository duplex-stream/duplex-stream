use super::{Conversation, ConversationFile, ConversationParser, ParserError};
use std::path::{Path, PathBuf};

/// Parser for Claude Code conversation files
pub struct ClaudeCodeParser {
    /// Base directory for Claude Code projects
    base_dir: PathBuf,
}

impl ClaudeCodeParser {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .map(|h| h.join(".claude").join("projects"))
            .unwrap_or_else(|| PathBuf::from("~/.claude/projects"));

        Self { base_dir }
    }

    /// Get the default Claude Code projects directory
    pub fn default_projects_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude").join("projects"))
    }

    /// Extract project path from the encoded directory name
    fn decode_project_path(encoded: &str) -> Option<PathBuf> {
        // Claude Code encodes paths like "-Users-name-project" for "/Users/name/project"
        if encoded.starts_with('-') {
            let path = encoded.replace('-', "/");
            Some(PathBuf::from(path))
        } else {
            None
        }
    }

    /// Extract session ID from filename
    fn extract_session_id(filename: &str) -> Option<String> {
        // Session files are like "abc123-def456-789.jsonl" (UUID format)
        if filename.ends_with(".jsonl") {
            let name = filename.trim_end_matches(".jsonl");
            // Basic UUID validation (36 chars with hyphens)
            if name.len() == 36 && name.chars().filter(|c| *c == '-').count() == 4 {
                return Some(name.to_string());
            }
        }
        None
    }
}

impl Default for ClaudeCodeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationParser for ClaudeCodeParser {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn detect(&self, path: &Path) -> bool {
        // Check if this looks like a Claude Code projects directory
        if path == self.base_dir {
            return true;
        }

        // Check if this is a project directory inside the base dir
        if let Some(parent) = path.parent() {
            if parent == self.base_dir {
                return true;
            }
        }

        // Check for .jsonl files that look like Claude Code sessions
        if path.is_file() && path.extension().map_or(false, |e| e == "jsonl") {
            // Check if parent directory looks like a Claude Code project dir
            if let Some(parent) = path.parent() {
                if let Some(parent_parent) = parent.parent() {
                    if parent_parent == self.base_dir {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn discover(&self, path: &Path) -> Vec<ConversationFile> {
        let mut files = Vec::new();

        let search_dir = if path == self.base_dir {
            path.to_path_buf()
        } else if path.is_dir() {
            path.to_path_buf()
        } else if path.is_file() {
            // If given a file, just return that file
            if let Some(session_id) = Self::extract_session_id(
                path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            ) {
                let project_path = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .and_then(Self::decode_project_path);

                files.push(ConversationFile {
                    path: path.to_path_buf(),
                    session_id: Some(session_id),
                    project_path,
                });
            }
            return files;
        } else {
            return files;
        };

        // Walk the directory structure
        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.is_dir() {
                    // This is a project directory
                    let project_name = entry_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    let project_path = Self::decode_project_path(project_name);

                    // Look for session files in this project
                    if let Ok(session_entries) = std::fs::read_dir(&entry_path) {
                        for session_entry in session_entries.flatten() {
                            let session_path = session_entry.path();
                            if session_path.is_file() {
                                if let Some(filename) =
                                    session_path.file_name().and_then(|n| n.to_str())
                                {
                                    if let Some(session_id) = Self::extract_session_id(filename) {
                                        files.push(ConversationFile {
                                            path: session_path,
                                            session_id: Some(session_id),
                                            project_path: project_path.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                } else if entry_path.is_file() {
                    // Check if this is a session file
                    if let Some(filename) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if let Some(session_id) = Self::extract_session_id(filename) {
                            let project_path = search_dir
                                .file_name()
                                .and_then(|n| n.to_str())
                                .and_then(Self::decode_project_path);

                            files.push(ConversationFile {
                                path: entry_path,
                                session_id: Some(session_id),
                                project_path,
                            });
                        }
                    }
                }
            }
        }

        files
    }

    fn parse(&self, file: &Path) -> Result<Conversation, ParserError> {
        // Read the raw content - we send the full JSONL to the API for processing
        let content = std::fs::read_to_string(file)?;

        let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let session_id = Self::extract_session_id(filename);

        let project_path = file
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .and_then(Self::decode_project_path);

        Ok(Conversation {
            source_path: file.to_path_buf(),
            source: self.name().to_string(),
            session_id,
            project_path,
            content,
        })
    }

    fn watch_patterns(&self) -> Vec<&str> {
        vec!["*.jsonl"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_project_path() {
        assert_eq!(
            ClaudeCodeParser::decode_project_path("-Users-test-project"),
            Some(PathBuf::from("/Users/test/project"))
        );

        assert_eq!(ClaudeCodeParser::decode_project_path("normaldir"), None);
    }

    #[test]
    fn test_extract_session_id() {
        assert_eq!(
            ClaudeCodeParser::extract_session_id("a1b2c3d4-e5f6-7890-abcd-ef1234567890.jsonl"),
            Some("a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string())
        );

        assert_eq!(ClaudeCodeParser::extract_session_id("not-a-uuid.jsonl"), None);
        assert_eq!(ClaudeCodeParser::extract_session_id("file.txt"), None);
    }
}
