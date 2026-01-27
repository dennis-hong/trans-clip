use super::types::{PermissionStatus, Position};
use tauri::Manager;

#[tauri::command]
pub async fn check_accessibility_permission() -> Result<PermissionStatus, String> {
    let granted = crate::hotkey::check_accessibility_permission();
    Ok(PermissionStatus { granted })
}

#[tauri::command]
pub async fn request_accessibility_permission() -> Result<(), String> {
    crate::hotkey::request_accessibility_permission();
    Ok(())
}

#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|e| format!("Failed to open settings: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn show_translation_popup(
    app: tauri::AppHandle,
    _text: String,
    _position: Option<Position>,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        crate::hotkey::show_window_at_position(&window);
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_translation_popup(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
