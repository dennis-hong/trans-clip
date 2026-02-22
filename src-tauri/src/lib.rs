#![allow(unexpected_cfgs)]

mod clipboard;
mod commands;
mod database;
mod hotkey;
mod keychain;
mod prompts;
mod utils;

use database::Database;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, RunEvent, WindowEvent,
};

pub struct AppState {
    pub db: Database,
}

fn show_main_window_and_emit(app: &tauri::AppHandle, event_name: &str) {
    if let Some(window) = app.get_webview_window("main") {
        hotkey::show_window_at_position(&window);
        if let Err(err) = window.emit(event_name, ()) {
            log::warn!("Failed to emit '{}' event: {}", event_name, err);
        }
    }
}

fn initialize_database(app_handle: &tauri::AppHandle) -> Result<Database, String> {
    let primary_path = match app_handle.path().app_data_dir() {
        Ok(dir) => dir.join("transclip.db"),
        Err(err) => {
            let fallback = std::env::temp_dir().join("trans-clip").join("transclip.db");
            log::warn!(
                "Failed to resolve app_data_dir ({}). Falling back to {}",
                err,
                fallback.display()
            );
            fallback
        }
    };

    if let Some(parent) = primary_path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            log::warn!(
                "Failed to create database directory {}: {}",
                parent.display(),
                err
            );
        }
    }

    match tauri::async_runtime::block_on(async { Database::new(&primary_path).await }) {
        Ok(db) => {
            log::info!("Database initialized at {}", primary_path.display());
            Ok(db)
        }
        Err(primary_err) => {
            let fallback_path = std::env::temp_dir().join("trans-clip").join("transclip.db");

            if fallback_path == primary_path {
                return Err(format!(
                    "Failed to initialize database at {}: {}",
                    primary_path.display(),
                    primary_err
                ));
            }

            if let Some(parent) = fallback_path.parent() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    format!(
                        "Failed to create fallback DB dir {}: {}",
                        parent.display(),
                        err
                    )
                })?;
            }

            match tauri::async_runtime::block_on(async { Database::new(&fallback_path).await }) {
                Ok(db) => {
                    log::warn!(
                        "Primary DB init failed ({}). Using fallback path {}",
                        primary_err,
                        fallback_path.display()
                    );
                    Ok(db)
                }
                Err(fallback_err) => Err(format!(
                    "Database initialization failed. primary={} ({}) fallback={} ({})",
                    primary_path.display(),
                    primary_err,
                    fallback_path.display(),
                    fallback_err
                )),
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger for debug output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("TransClip starting...");

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();

            let db = initialize_database(&app_handle)?;
            app.manage(AppState { db });

            // macOS app menu (top menu bar) - add manual update action.
            #[cfg(target_os = "macos")]
            {
                if let Err(err) = (|| -> Result<(), String> {
                    let app_menu = Menu::default(app.handle()).map_err(|e| e.to_string())?;
                    let app_submenu = app_menu
                        .items()
                        .map_err(|e| e.to_string())?
                        .into_iter()
                        .find_map(|item| item.as_submenu().cloned());

                    if let Some(app_submenu) = app_submenu {
                        let separator_before =
                            PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
                        let check_updates_app_item = MenuItem::with_id(
                            app,
                            "check_updates_app",
                            "Check for Updates...",
                            true,
                            None::<&str>,
                        )
                        .map_err(|e| e.to_string())?;
                        let separator_after =
                            PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

                        app_submenu
                            .insert_items(
                                &[&separator_before, &check_updates_app_item, &separator_after],
                                1,
                            )
                            .map_err(|e| e.to_string())?;
                    }

                    app.set_menu(app_menu).map_err(|e| e.to_string())?;
                    Ok(())
                })() {
                    log::error!("Failed to initialize app menu: {}", err);
                }

                app.on_menu_event(|app, event| {
                    if event.id().as_ref() == "check_updates_app" {
                        show_main_window_and_emit(app, "check_for_updates");
                    }
                });
            }

            if let Err(err) = (|| -> Result<(), String> {
                // Create tray menu
                let quit_item = MenuItem::with_id(app, "quit", "Quit TransClip", true, None::<&str>)
                    .map_err(|e| e.to_string())?;
                let show_item =
                    MenuItem::with_id(app, "show", "Show History", true, None::<&str>)
                        .map_err(|e| e.to_string())?;
                let settings_item = MenuItem::with_id(
                    app,
                    "settings",
                    "Settings...",
                    true,
                    None::<&str>,
                )
                .map_err(|e| e.to_string())?;
                let check_updates_item = MenuItem::with_id(
                    app,
                    "check_updates_tray",
                    "Check for Updates...",
                    true,
                    None::<&str>,
                )
                .map_err(|e| e.to_string())?;
                let feedback_item = MenuItem::with_id(
                    app,
                    "feedback",
                    "Report Bug / Feedback",
                    true,
                    None::<&str>,
                )
                .map_err(|e| e.to_string())?;
                let menu = Menu::with_items(
                    app,
                    &[
                        &show_item,
                        &settings_item,
                        &check_updates_item,
                        &feedback_item,
                        &quit_item,
                    ],
                )
                .map_err(|e| e.to_string())?;

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
                    .ok_or_else(|| "Failed to load tray icon from resources".to_string())?;

                let _tray = TrayIconBuilder::new()
                    .icon(icon)
                    .menu(&menu)
                    .show_menu_on_left_click(true)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "quit" => {
                            hotkey::stop_hotkey_monitor();
                            clipboard::stop_global_monitor();
                            app.exit(0);
                        }
                        "show" => {
                            show_main_window_and_emit(app, "show_history");
                        }
                        "settings" => {
                            show_main_window_and_emit(app, "open_settings");
                        }
                        "check_updates_tray" => {
                            show_main_window_and_emit(app, "check_for_updates");
                        }
                        "feedback" => {
                            // Open GitHub issues page in default browser
                            if let Err(err) =
                                open::that("https://github.com/dennis-hong/trans-clip/issues")
                            {
                                log::warn!("Failed to open feedback page: {}", err);
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
                                // Emit event to open history view
                                if let Err(err) = window.emit("show_history", ()) {
                                    log::warn!("Failed to emit show_history from tray click: {}", err);
                                }
                            }
                        }
                    })
                    .build(app)
                    .map_err(|e| e.to_string())?;

                Ok(())
            })() {
                log::error!("Tray initialization failed: {}", err);
            }

            // Initialize popup position from settings and migrate legacy API key only when needed.
            // Also load hotkey interval from settings for startup.
            let startup_hotkey_interval_ms = {
                let state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    let db = &state.db;
                    let mut interval_ms = 500_u64;

                    if let Ok(settings) = db.get_settings().await {
                        if settings.double_press_interval > 0 {
                            interval_ms = (settings.double_press_interval as u64).clamp(200, 1000);
                        }
                    }

                    // Only touch Keychain if a legacy SQLite key still exists.
                    match db.get_api_key().await {
                        Ok(Some(key)) if !key.is_empty() => {
                            keychain::migrate_api_key_from_db(db).await;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            log::warn!("Failed to check legacy API key before migration: {}", e);
                        }
                    }

                    interval_ms
                })
            };

            // Initialize and start hotkey monitoring (Cmd+C+C detection)
            let hotkey_handle = app_handle.clone();
            let hotkey_manager = hotkey::HotkeyManager::new(hotkey_handle, startup_hotkey_interval_ms);

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
            commands::clipboard::get_clipboard_item,
            commands::clipboard::delete_clipboard_item,
            commands::clipboard::clear_clipboard_history,
            commands::clipboard::toggle_pin_clipboard_item,
            commands::clipboard::create_clipboard_item,
            commands::clipboard::update_clipboard_item,
            commands::clipboard::set_clipboard,
            commands::clipboard::hide_and_paste_text,
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
            commands::system::start_hotkey_monitor,
            commands::system::open_accessibility_settings,
            commands::system::show_translation_popup,
            commands::system::hide_translation_popup,
            commands::system::open_feedback_page,
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
        ]);

    let app = match builder.build(tauri::generate_context!()) {
        Ok(app) => app,
        Err(e) => {
            log::error!("Failed to build Tauri application: {}", e);
            return;
        }
    };

    app.run(|app_handle, event| match event {
        RunEvent::ExitRequested { code, api, .. } => {
            if code.is_none() {
                // Prevent the app from exiting when all windows are closed
                // This allows it to remain as a menu bar app
                api.prevent_exit();
            } else {
                hotkey::stop_hotkey_monitor();
                clipboard::stop_global_monitor();
            }
        }
        RunEvent::Exit => {
            hotkey::stop_hotkey_monitor();
            clipboard::stop_global_monitor();
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
                    if let Err(err) = window.hide() {
                        log::warn!("Failed to hide main window on close request: {}", err);
                    }
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
    });
}
