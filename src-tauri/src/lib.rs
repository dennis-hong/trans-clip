mod clipboard;
mod commands;
mod database;
mod hotkey;
mod keychain;
mod translate;

use database::Database;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent,
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
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize database
            let db_path = app_handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir")
                .join("transclip.db");

            // Ensure directory exists
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let db = tauri::async_runtime::block_on(async {
                Database::new(&db_path).await.expect("Failed to initialize database")
            });

            app.manage(AppState {
                db: Arc::new(Mutex::new(db)),
            });

            // Create tray menu
            let quit_item = MenuItem::with_id(app, "quit", "Quit TransClip", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show History", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // Create tray icon - simple 32x32 white square
            // 32x32 pixels * 4 bytes (RGBA) = 4096 bytes
            let icon = Image::new_owned(vec![255, 255, 255, 255].repeat(32 * 32), 32, 32);

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            hotkey::show_window_at_position(&window);
                        }
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
                        }
                    }
                })
                .build(app)?;

            // Initialize popup position from settings
            {
                let state = app.state::<AppState>();
                let db = tauri::async_runtime::block_on(state.db.lock());
                if let Ok(settings) = tauri::async_runtime::block_on(db.get_settings()) {
                    hotkey::set_popup_position(&settings.popup_position);
                }
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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Translation commands
            commands::translate,
            // Clipboard commands
            commands::get_clipboard_history,
            commands::delete_clipboard_item,
            commands::clear_clipboard_history,
            commands::toggle_pin_clipboard_item,
            commands::set_clipboard,
            commands::paste_text,
            // Glossary commands
            commands::get_glossary_entries,
            commands::add_glossary_entry,
            commands::update_glossary_entry,
            commands::delete_glossary_entry,
            commands::import_glossary,
            commands::export_glossary,
            // Settings commands
            commands::get_settings,
            commands::update_settings,
            commands::get_api_key,
            commands::set_api_key,
            commands::delete_api_key,
            // System commands
            commands::check_accessibility_permission,
            commands::request_accessibility_permission,
            commands::show_translation_popup,
            commands::hide_translation_popup,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                RunEvent::ExitRequested { api, .. } => {
                    // Prevent the app from exiting when all windows are closed
                    // This allows it to remain as a menu bar app
                    api.prevent_exit();
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
