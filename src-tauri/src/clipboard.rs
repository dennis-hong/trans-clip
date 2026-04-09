//! Clipboard monitoring module for macOS
//!
//! This module monitors the system clipboard for changes and saves
//! copied text to the clipboard history database.

use crate::database::{ClipboardItemRow, Database};
use crate::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

static CLIPBOARD_MONITOR_RUNNING: OnceLock<Arc<AtomicBool>> = OnceLock::new();
const MAX_CLIPBOARD_HISTORY_BYTES: usize = 50_000;

/// Payload sent when clipboard content changes
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardChangedPayload {
    pub id: String,
    pub content: String,
    pub content_preview: String,
    pub copied_at: String,
    pub source_app: Option<String>,
}

/// Manages clipboard monitoring
pub struct ClipboardMonitor {
    app_handle: AppHandle,
    running: Arc<AtomicBool>,
    last_change_count: Arc<Mutex<i64>>,
    last_content_hash: Arc<Mutex<u64>>,
}

impl ClipboardMonitor {
    pub fn new(app_handle: AppHandle) -> Self {
        let running = CLIPBOARD_MONITOR_RUNNING
            .get_or_init(|| Arc::new(AtomicBool::new(false)))
            .clone();

        Self {
            app_handle,
            running,
            last_change_count: Arc::new(Mutex::new(-1)),
            last_content_hash: Arc::new(Mutex::new(0)),
        }
    }

    /// Start monitoring the clipboard for changes
    pub fn start(&self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        self.running.store(true, Ordering::SeqCst);
        log::info!("Starting clipboard monitor...");

        let app_handle = self.app_handle.clone();
        let running = self.running.clone();
        let last_change_count = self.last_change_count.clone();
        let last_content_hash = self.last_content_hash.clone();

        // Spawn monitoring task
        tauri::async_runtime::spawn(async move {
            Self::monitor_loop(app_handle, running, last_change_count, last_content_hash).await;
        });

        Ok(())
    }

    /// Stop monitoring
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        log::info!("Clipboard monitor stopped");
    }

    async fn monitor_loop(
        app_handle: AppHandle,
        running: Arc<AtomicBool>,
        last_change_count: Arc<Mutex<i64>>,
        last_content_hash: Arc<Mutex<u64>>,
    ) {
        log::info!("Clipboard monitor loop started");

        // Poll interval tuned for lower CPU impact while keeping UX responsive.
        let poll_interval = Duration::from_millis(700);

        while running.load(Ordering::SeqCst) {
            // Check for clipboard changes
            match Self::check_clipboard_change(&last_change_count, &last_content_hash).await {
                Ok(Some(text)) => {
                    // Clipboard changed with new text
                    if let Err(e) = Self::handle_clipboard_change(&app_handle, text).await {
                        log::error!("Failed to handle clipboard change: {}", e);
                    }
                }
                Ok(None) => {
                    // No change or not text content
                }
                Err(e) => {
                    log::error!("Error checking clipboard: {}", e);
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        log::info!("Clipboard monitor loop ended");
    }

    /// Check if clipboard has changed and return the new text if so
    async fn check_clipboard_change(
        last_change_count: &Arc<Mutex<i64>>,
        last_content_hash: &Arc<Mutex<u64>>,
    ) -> Result<Option<String>, String> {
        #[cfg(target_os = "macos")]
        {
            use objc::runtime::Object;
            use objc::{class, msg_send, sel, sel_impl};
            use std::process::Command;

            let change_count = unsafe {
                let pasteboard: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
                if pasteboard.is_null() {
                    return Err("Failed to access NSPasteboard".to_string());
                }
                let count: i64 = msg_send![pasteboard, changeCount];
                count
            };

            let mut last_count = last_change_count.lock().await;
            if *last_count == change_count {
                return Ok(None);
            }
            *last_count = change_count;
            drop(last_count);

            // Read clipboard text with explicit UTF-8 environment.
            let content_output = Command::new("pbpaste")
                .env("LANG", "en_US.UTF-8")
                .env("LC_ALL", "en_US.UTF-8")
                .output()
                .map_err(|e| format!("Failed to get clipboard content: {}", e))?;

            if !content_output.status.success() {
                return Ok(None); // Clipboard might contain non-text data
            }

            // Properly handle UTF-8 encoding for Korean and other languages
            let text = match String::from_utf8(content_output.stdout.clone()) {
                Ok(s) => s,
                Err(_) => {
                    // Fallback: try to decode as UTF-8 with replacement
                    String::from_utf8_lossy(&content_output.stdout).to_string()
                }
            };

            // Skip empty content
            if text.trim().is_empty() {
                return Ok(None);
            }

            // Calculate hash of content to detect changes
            let content_hash = Self::hash_string(&text);

            let mut last_hash = last_content_hash.lock().await;
            if *last_hash != content_hash {
                *last_hash = content_hash;
            }
            Ok(Some(text))
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (last_change_count, last_content_hash);
            Err("Clipboard monitoring only supported on macOS".to_string())
        }
    }

    /// Simple string hash function
    fn hash_string(s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Handle a clipboard change event
    async fn handle_clipboard_change(app_handle: &AppHandle, text: String) -> Result<(), String> {
        log::info!("Clipboard changed: {} chars", text.len());

        if text.len() > MAX_CLIPBOARD_HISTORY_BYTES {
            log::warn!(
                "Skipping clipboard history save for oversized payload ({} bytes > {} bytes)",
                text.len(),
                MAX_CLIPBOARD_HISTORY_BYTES
            );
            return Ok(());
        }

        // Get app state
        let state = app_handle
            .try_state::<AppState>()
            .ok_or("Failed to get app state")?;

        let source_app = Self::get_frontmost_app();
        let db = &state.db;

        // Get current settings for max history count
        let settings = db.get_settings().await.map_err(|e| e.to_string())?;

        // Check if this content already exists
        let existing = Self::find_existing_item(db, &text).await?;

        let item = if let Some(existing_id) = existing {
            // Update existing item's timestamp
            db.update_clipboard_item_timestamp(&existing_id)
                .await
                .map_err(|e| e.to_string())?;

            // Return the updated item info
            ClipboardChangedPayload {
                id: existing_id,
                content: text.clone(),
                content_preview: Self::create_preview(&text),
                copied_at: chrono::Utc::now().to_rfc3339(),
                source_app: source_app.clone(),
            }
        } else {
            // Create new clipboard item
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let preview = Self::create_preview(&text);
            let char_count = text.chars().count() as i32;
            let word_count = text.split_whitespace().count() as i32;
            let item_row = ClipboardItemRow {
                id: id.clone(),
                content: text.clone(),
                content_preview: preview.clone(),
                copied_at: now.clone(),
                source_app: source_app.clone(),
                is_pinned: 0,
                character_count: Some(char_count),
                word_count: Some(word_count),
                updated_at: None,
            };

            db.insert_clipboard_item(&item_row)
                .await
                .map_err(|e| e.to_string())?;

            ClipboardChangedPayload {
                id,
                content: text,
                content_preview: preview,
                copied_at: now,
                source_app,
            }
        };

        // Cleanup old items if needed
        db.cleanup_old_clipboard_items(settings.max_history_count)
            .await
            .map_err(|e| e.to_string())?;

        // Emit event to frontend
        app_handle
            .emit("clipboard_changed", item.clone())
            .map_err(|e| e.to_string())?;

        log::info!("Clipboard item saved and event emitted: {}", item.id);

        Ok(())
    }

    /// Find existing clipboard item with same content
    async fn find_existing_item(db: &Database, content: &str) -> Result<Option<String>, String> {
        db.find_clipboard_item_by_content(content)
            .await
            .map_err(|e| e.to_string())
    }

    /// Create a preview of the text (first 100 chars, single line)
    fn create_preview(text: &str) -> String {
        let preview: String = text
            .chars()
            .take(100)
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();

        if text.chars().count() > 100 {
            format!("{}...", preview.trim())
        } else {
            preview.trim().to_string()
        }
    }

    /// Get the frontmost application name (best effort)
    fn get_frontmost_app() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            let output = Command::new("osascript")
                .env("LANG", "en_US.UTF-8")
                .env("LC_ALL", "en_US.UTF-8")
                .arg("-e")
                .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
                .output()
                .ok()?;

            if output.status.success() {
                let app_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !app_name.is_empty() {
                    return Some(app_name);
                }
            }

            None
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }
}

pub fn stop_global_monitor() {
    if let Some(running) = CLIPBOARD_MONITOR_RUNNING.get() {
        running.store(false, Ordering::SeqCst);
        log::info!("Clipboard monitor stop requested");
    }
}
