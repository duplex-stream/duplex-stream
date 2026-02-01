pub mod auth;
pub mod config;
pub mod db;
pub mod parsers;
pub mod sync;
pub mod watcher;

// Re-export for Tauri
pub use config::Config;
pub use db::Database;
pub use sync::SyncEngine;
pub use watcher::FileWatcher;
