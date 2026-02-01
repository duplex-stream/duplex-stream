// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::{Parser, Subcommand};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod auth;
mod config;
mod db;
mod parsers;
mod sync;
mod watcher;

#[derive(Parser)]
#[command(name = "duplex")]
#[command(about = "Duplex Stream - Sync coding agent conversations")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authentication commands
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Sync conversations now
    Sync,
    /// Run as desktop app (default)
    Run,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Log in with device code flow
    Login,
    /// Log out and clear credentials
    Logout,
    /// Show current auth status
    Status,
}

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("duplex=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Auth { action }) => {
            // Create a tokio runtime for async auth operations
            let rt = tokio::runtime::Runtime::new().unwrap();

            match action {
                AuthAction::Login => {
                    if let Err(e) = rt.block_on(auth::login()) {
                        eprintln!("Login failed: {}", e);
                        std::process::exit(1);
                    }
                }
                AuthAction::Logout => {
                    if let Err(e) = auth::logout() {
                        eprintln!("Logout failed: {}", e);
                        std::process::exit(1);
                    }
                }
                AuthAction::Status => {
                    if let Err(e) = auth::status() {
                        eprintln!("Failed to check status: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Some(Commands::Sync) => {
            println!("Syncing conversations...");
            // TODO: Trigger sync
            println!("Sync not yet implemented");
        }
        Some(Commands::Run) | None => {
            // Run as desktop app with system tray
            run_desktop_app();
        }
    }
}

fn run_desktop_app() {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
        Emitter, Listener,
    };

    tracing::info!("Starting Duplex Stream desktop app");

    // Load configuration
    let app_config = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load config: {}", e);
            config::Config::default()
        }
    };

    // Create parser registry
    let registry = Arc::new(parsers::ParserRegistry::new());

    // Create file watcher with configured debounce duration
    let debounce_secs = app_config.sync.debounce_seconds;
    let mut file_watcher = match watcher::FileWatcher::new(Duration::from_secs(debounce_secs)) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to create file watcher: {}", e);
            return;
        }
    };

    // Discover and watch directories
    let watch_count = match watcher::discover_and_watch(&mut file_watcher, &registry, &app_config) {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to discover directories: {}", e);
            0
        }
    };

    // Create sync engine
    // Load API URL from env or use default
    let api_url = std::env::var("DUPLEX_API_URL")
        .unwrap_or_else(|_| "http://localhost:8787".to_string());

    // Try to load access token from credentials, fall back to env var
    let access_token = config::get_access_token()
        .ok()
        .or_else(|| std::env::var("DUPLEX_ACCESS_TOKEN").ok());

    if access_token.is_none() {
        tracing::warn!("No authentication credentials found. Run 'duplex auth login' to authenticate.");
    }

    let sync_engine = match sync::create_shared_engine(api_url, access_token, registry.clone()) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to create sync engine: {}", e);
            return;
        }
    };

    // Wrap watcher in Arc<Mutex> for sharing with event handler thread
    let file_watcher = Arc::new(Mutex::new(file_watcher));
    let file_watcher_clone = file_watcher.clone();
    let sync_engine_clone = sync_engine.clone();

    // Start background thread to handle file change events
    std::thread::spawn(move || {
        // Create a tokio runtime for async operations
        let rt = tokio::runtime::Runtime::new().unwrap();

        loop {
            let event = {
                let watcher = file_watcher_clone.lock().unwrap();
                watcher.try_recv()
            };

            if let Some(event) = event {
                tracing::info!(
                    "File changed: {:?} (parser: {})",
                    event.path,
                    event.parser_name
                );

                // Queue for sync
                {
                    let mut engine = sync_engine_clone.lock().unwrap();
                    if let Err(e) = engine.handle_file_change(event) {
                        tracing::error!("Failed to queue file for sync: {}", e);
                    }
                }

                // Process the queue
                rt.block_on(async {
                    let mut engine = sync_engine_clone.lock().unwrap();
                    if let Err(e) = engine.process_all().await {
                        tracing::error!("Failed to process sync queue: {}", e);
                    }
                });
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(move |app| {
            // Register deep-link scheme (needed for dev mode)
            #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register("duplex") {
                    tracing::error!("Failed to register deep-link: {}", e);
                } else {
                    tracing::info!("Registered duplex:// URL scheme");
                }
            }

            // Handle deep link events
            let app_handle = app.handle().clone();
            app.listen("deep-link://new-url", move |event| {
                let payload = event.payload();
                tracing::info!("Received deep link: {:?}", payload);
                // Parse the URL to extract token
                if let Ok(url) = url::Url::parse(payload) {
                    if url.scheme() == "duplex" && url.host_str() == Some("auth") {
                        // Extract token from query params
                        if let Some(token) = url.query_pairs().find(|(k, _)| k == "token").map(|(_, v)| v.to_string()) {
                            tracing::info!("Received auth token from deep link");
                            // Store the token in keyring
                            if let Err(e) = store_token_in_keyring(&token) {
                                tracing::error!("Failed to store token in keyring: {}", e);
                            } else {
                                tracing::info!("Token stored successfully");
                                // Emit event to trigger menu refresh
                                let _ = app_handle.emit("auth-state-changed", true);
                            }
                        }
                    }
                }
            });

            // Check auth state for menu
            let is_authenticated = get_token_from_keyring().is_some();

            // Build the tray menu
            let status_text = format!(
                "Watching {} project{}",
                watch_count,
                if watch_count == 1 { "" } else { "s" }
            );
            let status = MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
            let auth_status = if is_authenticated {
                MenuItem::with_id(app, "auth_status", "✓ Signed In", false, None::<&str>)?
            } else {
                MenuItem::with_id(app, "auth_status", "○ Not Signed In", false, None::<&str>)?
            };
            let auth_action = if is_authenticated {
                MenuItem::with_id(app, "auth_action", "Sign Out", true, None::<&str>)?
            } else {
                MenuItem::with_id(app, "auth_action", "Sign In...", true, None::<&str>)?
            };
            let sync_now = MenuItem::with_id(app, "sync_now", "Sync Now", is_authenticated, None::<&str>)?;
            let separator = MenuItem::with_id(app, "sep1", "---", false, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&status, &auth_status, &auth_action, &sync_now, &separator, &settings, &quit])?;

            // Create the tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "auth_action" => {
                        // Check current auth state
                        if get_token_from_keyring().is_some() {
                            // Sign out
                            tracing::info!("Signing out...");
                            if let Err(e) = clear_keyring_token() {
                                tracing::error!("Failed to sign out: {}", e);
                            } else {
                                tracing::info!("Signed out successfully");
                                // Emit event to trigger menu refresh
                                let _ = app.emit("auth-state-changed", false);
                            }
                        } else {
                            // Sign in - open browser
                            tracing::info!("Opening browser for sign in...");
                            if let Err(e) = open_auth_browser() {
                                tracing::error!("Failed to open browser: {}", e);
                            }
                        }
                    }
                    "sync_now" => {
                        tracing::info!("Sync Now clicked");
                        // TODO: Trigger sync
                    }
                    "settings" => {
                        tracing::info!("Settings clicked");
                        if let Err(e) = open_config_in_editor() {
                            tracing::error!("Failed to open config: {}", e);
                        }
                    }
                    "quit" => {
                        tracing::info!("Quit clicked");
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            tracing::info!("System tray initialized, watching {} directories", watch_count);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Store access token in system keyring
fn store_token_in_keyring(token: &str) -> Result<(), Box<dyn std::error::Error>> {
    let entry = keyring::Entry::new("duplex-stream", "access_token")?;
    entry.set_password(token)?;
    Ok(())
}

/// Get access token from keyring
fn get_token_from_keyring() -> Option<String> {
    let entry = keyring::Entry::new("duplex-stream", "access_token").ok()?;
    entry.get_password().ok()
}

/// Clear token from keyring
fn clear_keyring_token() -> Result<(), Box<dyn std::error::Error>> {
    let entry = keyring::Entry::new("duplex-stream", "access_token")?;
    entry.delete_credential()?;
    Ok(())
}

/// Open browser for web authentication
fn open_auth_browser() -> Result<(), Box<dyn std::error::Error>> {
    let default_url = if cfg!(debug_assertions) {
        "http://localhost:5173"
    } else {
        "https://app.duplex.stream"
    };
    let auth_url = std::env::var("DUPLEX_WEB_URL").unwrap_or_else(|_| default_url.to_string());
    let full_url = format!("{}/auth/desktop", auth_url);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&full_url)
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&full_url)
            .spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", &full_url])
            .spawn()?;
    }

    Ok(())
}

fn open_config_in_editor() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = config::get_config_path()?;

    // Try to open with default editor
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-t")
            .arg(&config_path)
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&config_path)
            .spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("notepad")
            .arg(&config_path)
            .spawn()?;
    }

    Ok(())
}
