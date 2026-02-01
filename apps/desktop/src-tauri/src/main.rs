// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::{Parser, Subcommand};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod auth;
mod config;
mod db;
mod oauth;
mod parsers;
mod sync;
mod token_manager;
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
        Emitter, Listener, Manager,
    };

    tracing::info!("Starting Duplex Stream desktop app");

    // Initialize secure token storage and migrate legacy tokens
    let token_storage = config::SecureTokenStorage::new();
    match token_storage.migrate_from_legacy() {
        Ok(true) => tracing::info!("Migrated legacy token to keyring"),
        Ok(false) => tracing::debug!("No legacy token to migrate"),
        Err(e) => tracing::warn!("Failed to migrate legacy token: {}", e),
    }

    // Create token manager
    let token_manager = token_manager::create_shared_manager();

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

    // Try to load access token from keyring, fall back to env var
    let access_token = token_manager.get_access_token()
        .or_else(|| config::get_access_token().ok())
        .or_else(|| std::env::var("DUPLEX_ACCESS_TOKEN").ok());

    if access_token.is_none() {
        tracing::warn!("No authentication credentials found. Sign in via the menu bar.");
    }

    // Start background token refresh in a separate thread with persistent runtime
    let token_manager_for_refresh = token_manager.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let _ = token_manager_for_refresh.start_background_refresh().await;
        });
    });

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
    let sync_engine_for_menu = sync_engine.clone();

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
            // Hide dock icon on macOS (menubar-only app)
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
                unsafe {
                    let app = NSApp();
                    app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
                }
                tracing::info!("Set app to accessory mode (no dock icon)");
            }

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

            // Handle deep link events (keeping for future use, but not for auth tokens)
            app.listen("deep-link://new-url", move |event| {
                let payload = event.payload();
                tracing::info!("Received deep link payload: {:?}", payload);

                // Payload is a JSON array of URLs
                let urls: Vec<String> = match serde_json::from_str(payload) {
                    Ok(urls) => urls,
                    Err(e) => {
                        tracing::error!("Failed to parse deep link payload as JSON: {}", e);
                        return;
                    }
                };

                for url_str in urls {
                    tracing::info!("Processing deep link URL: {}", url_str);
                    if let Ok(url) = url::Url::parse(&url_str) {
                        // Handle other deep links here if needed
                        // Auth is now handled via PKCE loopback server
                        tracing::debug!("Deep link received: scheme={}, host={:?}, path={}",
                            url.scheme(), url.host_str(), url.path());
                    }
                }
            });

            // Build initial menu
            let menu = build_tray_menu(app, watch_count)?;

            // Create the tray icon
            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "auth_action" => {
                        // Check current auth state using keyring
                        let storage = config::SecureTokenStorage::new();
                        if storage.has_tokens() {
                            // Sign out
                            tracing::info!("Signing out...");
                            if let Err(e) = storage.clear_tokens() {
                                tracing::error!("Failed to sign out: {}", e);
                            } else {
                                tracing::info!("Signed out successfully");
                                // Emit event to trigger menu refresh
                                let _ = app.emit("auth-state-changed", false);
                            }
                        } else {
                            // Sign in using PKCE OAuth flow
                            tracing::info!("Starting OAuth sign in flow...");
                            let app_handle = app.clone();
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                rt.block_on(async {
                                    match auth::desktop_login().await {
                                        Ok(token) => {
                                            tracing::info!(
                                                "Sign in successful for {}",
                                                token.user.email.as_deref().unwrap_or(&token.user.id)
                                            );
                                            // Emit event to trigger menu refresh
                                            let _ = app_handle.emit("auth-state-changed", true);
                                        }
                                        Err(e) => {
                                            tracing::error!("Sign in failed: {}", e);
                                        }
                                    }
                                });
                            });
                        }
                    }
                    "sync_now" => {
                        tracing::info!("Sync Now clicked");
                        let sync_engine = sync_engine_for_menu.clone();
                        std::thread::spawn(move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async {
                                let mut engine = sync_engine.lock().unwrap();
                                match engine.process_all().await {
                                    Ok(count) => {
                                        tracing::info!("Sync completed: {} items processed", count);
                                    }
                                    Err(e) => {
                                        tracing::error!("Sync failed: {}", e);
                                    }
                                }
                            });
                        });
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

            // Listen for auth state changes to update menu
            let tray_id = tray.id().clone();
            let app_handle = app.handle().clone();
            app.listen("auth-state-changed", move |_event| {
                tracing::info!("Auth state changed, updating menu...");

                // Clone handles for the spawned thread
                let app_handle = app_handle.clone();
                let tray_id = tray_id.clone();

                // Delay menu update to avoid interfering with current menu interaction
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(100));

                    // Rebuild the menu with new auth state
                    if let Some(tray) = app_handle.tray_by_id(&tray_id) {
                        let storage = config::SecureTokenStorage::new();
                        let is_authenticated = storage.has_tokens();
                        tracing::info!("is_authenticated = {}", is_authenticated);

                        // Update menu items
                        let auth_status_text = if is_authenticated { "✓ Signed In" } else { "○ Not Signed In" };
                        let auth_action_text = if is_authenticated { "Sign Out" } else { "Sign In..." };
                        tracing::info!("Setting menu: auth_status='{}', auth_action='{}'", auth_status_text, auth_action_text);

                        // Create new menu
                        if let Ok(menu) = Menu::with_items(&app_handle, &[
                            &MenuItem::with_id(&app_handle, "status", format!("Watching {} project(s)", watch_count), false, None::<&str>).unwrap(),
                            &MenuItem::with_id(&app_handle, "auth_status", auth_status_text, false, None::<&str>).unwrap(),
                            &MenuItem::with_id(&app_handle, "auth_action", auth_action_text, true, None::<&str>).unwrap(),
                            &MenuItem::with_id(&app_handle, "sync_now", "Sync Now", is_authenticated, None::<&str>).unwrap(),
                            &MenuItem::with_id(&app_handle, "sep1", "---", false, None::<&str>).unwrap(),
                            &MenuItem::with_id(&app_handle, "settings", "Settings...", true, None::<&str>).unwrap(),
                            &MenuItem::with_id(&app_handle, "quit", "Quit", true, None::<&str>).unwrap(),
                        ]) {
                            let _ = tray.set_menu(Some(menu));
                            tracing::info!("Menu updated successfully");
                        }
                    }
                });
            });

            tracing::info!("System tray initialized, watching {} directories", watch_count);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

/// Build the tray menu based on current auth state
fn build_tray_menu(app: &tauri::App, watch_count: usize) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};

    let storage = config::SecureTokenStorage::new();
    let is_authenticated = storage.has_tokens();

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

    Ok(Menu::with_items(app, &[&status, &auth_status, &auth_action, &sync_now, &separator, &settings, &quit])?)
}
