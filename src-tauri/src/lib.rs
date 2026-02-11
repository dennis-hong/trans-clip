mod clipboard;
mod commands;
mod database;
mod hotkey;
mod keychain;
mod prompts;
mod utils;

use database::Database;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, RunEvent, WindowEvent,
};
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger for debug output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("TransClip starting...");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize database
            let db_path = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?
                .join("transclip.db");

            // Ensure directory exists
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let db = tauri::async_runtime::block_on(async {
                Database::new(&db_path).await
            })
            .map_err(|e| format!("Failed to initialize database: {}", e))?;

            app.manage(AppState {
                db: Arc::new(Mutex::new(db)),
            });

            // Create tray menu
            let quit_item = MenuItem::with_id(app, "quit", "Quit TransClip", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show History", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
            let feedback_item = MenuItem::with_id(app, "feedback", "Report Bug / Feedback", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &settings_item, &feedback_item, &quit_item])?;

            // Load tray icon from file
            let icon = app
                .path()
                .resource_dir()
                .ok()
                .map(|dir| dir.join("icons/32x32.png"))
                .and_then(|path| Image::from_path(&path).ok())
                .or_else(|| {
                    // Fallback to embedded icon if file not found
                    Image::from_bytes(include_bytes!("../icons/32x32.png")).ok()
                })
                .ok_or("Failed to load tray icon from file or embedded resource")?;

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            hotkey::show_window_at_position(&window);
                            // Emit event to open history view
                            let _ = window.emit("show_history", ());
                        }
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            hotkey::show_window_at_position(&window);
                            // Emit event to open settings view
                            let _ = window.emit("open_settings", ());
                        }
                    }
                    "feedback" => {
                        // Open GitHub issues page in default browser
                        let _ = open::that("https://github.com/dennis-hong/trans-clip/issues");
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            hotkey::show_window_at_position(&window);
                            // Emit event to open history view
                            let _ = window.emit("show_history", ());
                        }
                    }
                })
                .build(app)?;

            // Initialize popup position from settings and migrate API key to Keychain
            {
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    let db = state.db.lock().await;
                    if let Ok(settings) = db.get_settings().await {
                        hotkey::set_popup_position(&settings.popup_position);
                    }
                    // Migrate API key from SQLite to Keychain (backward compat)
                    keychain::migrate_api_key_from_db(&db).await;
                });
            }

            // Initialize and start hotkey monitoring (Cmd+C+C detection)
            let hotkey_handle = app_handle.clone();
            let hotkey_manager = hotkey::HotkeyManager::new(hotkey_handle, 500);

            // Check accessibility permission before starting
            if hotkey::check_accessibility_permission() {
                if let Err(e) = hotkey_manager.start() {
                    log::error!("Failed to start hotkey manager: {}", e);
                } else {
                    log::info!("Hotkey manager started successfully");
                }
            } else {
                log::warn!("Accessibility permission not granted. Cmd+C+C hotkey will not work.");
                log::info!("Please grant accessibility permission in System Settings > Privacy & Security > Accessibility");
            }

            // Initialize and start clipboard monitoring
            let clipboard_handle = app_handle.clone();
            let clipboard_monitor = clipboard::ClipboardMonitor::new(clipboard_handle);
            
            if let Err(e) = clipboard_monitor.start() {
                log::error!("Failed to start clipboard monitor: {}", e);
            } else {
                log::info!("Clipboard monitor started successfully");
            }

            // Show window on startup
            if let Some(window) = app.get_webview_window("main") {
                hotkey::show_window_at_position(&window);
                log::info!("Window shown on startup");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Translation commands
            commands::translate::translate,
            commands::translate::get_cached_translation,
            commands::translate::translate_stream,
            // Polish commands
            commands::polish::polish,
            commands::polish::polish_stream,
            // Clipboard commands
            commands::clipboard::get_clipboard_history,
            commands::clipboard::delete_clipboard_item,
            commands::clipboard::clear_clipboard_history,
            commands::clipboard::toggle_pin_clipboard_item,
            commands::clipboard::create_clipboard_item,
            commands::clipboard::update_clipboard_item,
            commands::clipboard::set_clipboard,
            commands::clipboard::paste_text,
            // Glossary commands
            commands::glossary::get_glossary_entries,
            commands::glossary::add_glossary_entry,
            commands::glossary::update_glossary_entry,
            commands::glossary::delete_glossary_entry,
            commands::glossary::import_glossary,
            commands::glossary::export_glossary,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_api_key,
            commands::settings::set_api_key,
            commands::settings::delete_api_key,
            // System commands
            commands::system::check_accessibility_permission,
            commands::system::request_accessibility_permission,
            commands::system::open_accessibility_settings,
            commands::system::show_translation_popup,
            commands::system::hide_translation_popup,
            // Window management commands
            commands::window::get_monitors,
            commands::window::get_current_monitor_index,
            commands::window::get_window_position,
            commands::window::set_window_position,
            commands::window::set_window_size,
            commands::window::move_to_monitor,
            commands::window::toggle_always_on_top,
            commands::window::snap_to_bottom,
            commands::window::snap_to_edge,
            commands::window::set_drawer_collapsed,
            commands::window::set_drawer_mode,
            commands::window::get_current_monitor_info,
            commands::window::save_window_width_for_monitor,
            commands::window::open_postit_editor,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log::error!("Failed to build Tauri application: {}", e);
            panic!("Failed to build Tauri application: {}", e);
        })
        .run(|app_handle, event| {
            match event {
                RunEvent::ExitRequested { api, .. } => {
                    // Prevent the app from exiting when all windows are closed
                    // This allows it to remain as a menu bar app
                    api.prevent_exit();
                }
                RunEvent::WindowEvent {
                    label,
                    event: WindowEvent::CloseRequested { api, .. },
                    ..
                } => {
                    // When close button is clicked, hide the window instead of destroying it
                    if label == "main" {
                        api.prevent_close();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                }
                RunEvent::Reopen { .. } => {
                    // Handle dock icon click - show the main window
                    if let Some(window) = app_handle.get_webview_window("main") {
                        hotkey::show_window_at_position(&window);
                    }
                }
                _ => {}
            }
        });
}
