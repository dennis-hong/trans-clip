use crate::utils::monitor::{
    calculate_adaptive_width, generate_monitor_key, get_logical_bounds, sort_monitors_by_position,
};
use crate::AppState;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    LazyLock, Mutex,
};
use tauri::{Emitter, Manager, Monitor, State};

use super::types::{CurrentMonitorInfo, MonitorInfo, SnapEdge, SnapResult, WindowPosition};

// Store the last valid monitor index to preserve position across hide/show cycles
static LAST_MONITOR_INDEX: AtomicUsize = AtomicUsize::new(0);
static LAST_MONITOR_STATE: LazyLock<Mutex<Option<LastMonitorState>>> =
    LazyLock::new(|| Mutex::new(None));
const POSTIT_EDITOR_LABEL: &str = "postit-editor";

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PostItEditorOpenPayload {
    mode: String,
    item_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LastMonitorState {
    signature: String,
    monitor_key: String,
    position_x: i32,
    position_y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MonitorSelectionCandidate {
    signature: String,
    monitor_key: String,
    position_x: i32,
    position_y: i32,
}

fn generate_monitor_signature(monitor: &Monitor) -> String {
    let size = monitor.size();
    let name = monitor
        .name()
        .map(|value| value.to_string())
        .unwrap_or_default();
    format!(
        "{}|{}x{}@{:.2}",
        name,
        size.width,
        size.height,
        monitor.scale_factor()
    )
}

fn build_monitor_candidate(monitor: &Monitor) -> MonitorSelectionCandidate {
    let size = monitor.size();
    let position = monitor.position();
    MonitorSelectionCandidate {
        signature: generate_monitor_signature(monitor),
        monitor_key: generate_monitor_key(size.width, size.height, monitor.scale_factor()),
        position_x: position.x,
        position_y: position.y,
    }
}

fn resolve_saved_monitor_index(
    saved_state: Option<&LastMonitorState>,
    candidates: &[MonitorSelectionCandidate],
) -> Option<usize> {
    let saved_state = saved_state?;

    let mut best_match = None;
    let mut best_distance = i32::MAX;

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.signature == saved_state.signature {
            let distance = (candidate.position_x - saved_state.position_x).abs()
                + (candidate.position_y - saved_state.position_y).abs();

            if distance < best_distance {
                best_distance = distance;
                best_match = Some(index);
            }
        }
    }

    if best_match.is_some() {
        return best_match;
    }

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.monitor_key != saved_state.monitor_key {
            continue;
        }

        let distance = (candidate.position_x - saved_state.position_x).abs()
            + (candidate.position_y - saved_state.position_y).abs();

        if distance < best_distance {
            best_distance = distance;
            best_match = Some(index);
        }
    }

    best_match
}

fn get_last_monitor_state() -> Option<LastMonitorState> {
    match LAST_MONITOR_STATE.lock() {
        Ok(state) => state.clone(),
        Err(err) => {
            log::warn!("Failed to read last-used monitor state: {}", err);
            None
        }
    }
}

fn remember_last_monitor(index: usize, monitor: &Monitor) {
    LAST_MONITOR_INDEX.store(index, Ordering::SeqCst);

    let size = monitor.size();
    let position = monitor.position();
    let state = LastMonitorState {
        signature: generate_monitor_signature(monitor),
        monitor_key: generate_monitor_key(size.width, size.height, monitor.scale_factor()),
        position_x: position.x,
        position_y: position.y,
    };

    match LAST_MONITOR_STATE.lock() {
        Ok(mut saved_state) => {
            *saved_state = Some(state.clone());
        }
        Err(err) => {
            log::warn!("Failed to update last-used monitor state: {}", err);
        }
    }

    log::info!(
        "Updated last-used monitor: index={}, signature={}, monitor_key={}, position=({}, {})",
        index,
        state.signature,
        state.monitor_key,
        state.position_x,
        state.position_y
    );
}

fn find_monitor_index_from_cursor(
    app: &tauri::AppHandle,
    sorted_monitors: &[&Monitor],
) -> Option<usize> {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        // Get current mouse position
        let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            Ok(s) => s,
            Err(_) => {
                log::warn!("Failed to create event source for cursor position");
                return None;
            }
        };
        let event = match CGEvent::new(source) {
            Ok(e) => e,
            Err(_) => {
                log::warn!("Failed to create event for cursor position");
                return None;
            }
        };
        let cursor_pos = event.location();
        let cursor_x = cursor_pos.x as i32;
        let cursor_y = cursor_pos.y as i32;

        log::info!("Cursor position: ({}, {})", cursor_x, cursor_y);

        // Get monitors and sort by position
        let monitors = match app.available_monitors() {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to get monitors: {}", e);
                return None;
            }
        };

        let _ = monitors;

        // Find which monitor contains the cursor
        for (idx, monitor) in sorted_monitors.iter().enumerate() {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();

            // Monitor position is in physical pixels, cursor is in logical
            // Convert cursor to physical for comparison
            let cursor_physical_x = (cursor_x as f64 * scale) as i32;
            let cursor_physical_y = (cursor_y as f64 * scale) as i32;

            if cursor_physical_x >= mon_pos.x
                && cursor_physical_x < mon_pos.x + mon_size.width as i32
                && cursor_physical_y >= mon_pos.y
                && cursor_physical_y < mon_pos.y + mon_size.height as i32
            {
                log::info!("Cursor is on monitor {} (sorted index)", idx);
                return Some(idx);
            }
        }

        // Fallback: try with logical coordinates directly (for single-scale setups)
        for (idx, monitor) in sorted_monitors.iter().enumerate() {
            let bounds = get_logical_bounds(monitor);

            if cursor_x >= bounds.x
                && cursor_x < bounds.x + bounds.width
                && cursor_y >= bounds.y
                && cursor_y < bounds.y + bounds.height
            {
                log::info!("Cursor is on monitor {} (logical fallback)", idx);
                return Some(idx);
            }
        }

        log::warn!("Could not determine monitor from cursor position");
        None
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        log::info!("Cursor-based monitor detection not implemented for this platform");
        None
    }
}

pub fn resolve_last_used_monitor<'a>(
    app: &tauri::AppHandle,
    sorted_monitors: &[&'a Monitor],
) -> Option<(usize, &'a Monitor)> {
    if sorted_monitors.is_empty() {
        return None;
    }

    let candidates: Vec<_> = sorted_monitors
        .iter()
        .map(|monitor| build_monitor_candidate(monitor))
        .collect();

    if let Some(saved_index) =
        resolve_saved_monitor_index(get_last_monitor_state().as_ref(), &candidates)
    {
        let monitor = *sorted_monitors.get(saved_index)?;
        remember_last_monitor(saved_index, monitor);
        return Some((saved_index, monitor));
    }

    if let Some(cursor_index) = find_monitor_index_from_cursor(app, sorted_monitors) {
        let monitor = *sorted_monitors.get(cursor_index)?;
        remember_last_monitor(cursor_index, monitor);
        return Some((cursor_index, monitor));
    }

    if let Ok(Some(primary_monitor)) = app.primary_monitor() {
        let primary_position = primary_monitor.position();
        let primary_size = primary_monitor.size();

        if let Some(primary_index) = sorted_monitors.iter().position(|monitor| {
            monitor.position() == primary_position && monitor.size() == primary_size
        }) {
            let monitor = *sorted_monitors.get(primary_index)?;
            remember_last_monitor(primary_index, monitor);
            return Some((primary_index, monitor));
        }
    }

    let fallback_monitor = *sorted_monitors.first()?;
    remember_last_monitor(0, fallback_monitor);
    Some((0, fallback_monitor))
}

#[tauri::command]
pub async fn get_monitors(app: tauri::AppHandle) -> Result<Vec<MonitorInfo>, String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let primary = app.primary_monitor().map_err(|e| e.to_string())?;

    let sorted_monitors = sort_monitors_by_position(monitors.iter());

    let result: Vec<MonitorInfo> = sorted_monitors
        .into_iter()
        .map(|monitor| {
            let pos = monitor.position();
            let size = monitor.size();
            let is_primary = primary
                .as_ref()
                .is_some_and(|p| p.position() == monitor.position() && p.size() == monitor.size());

            MonitorInfo {
                name: monitor.name().map(|s| s.to_string()),
                position_x: pos.x,
                position_y: pos.y,
                width: size.width,
                height: size.height,
                scale_factor: monitor.scale_factor(),
                is_primary,
            }
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub async fn get_window_position(app: tauri::AppHandle) -> Result<WindowPosition, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;

    Ok(WindowPosition {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
    })
}

#[tauri::command]
pub async fn set_window_position(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_window_size(app: tauri::AppHandle, width: u32, height: u32) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    window
        .set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn move_to_monitor(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    monitor_index: usize,
    anchor: String,
) -> Result<(), String> {
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    log::info!(
        "move_to_monitor: requested index={}, anchor={}",
        monitor_index,
        anchor
    );

    let sorted_monitors = sort_monitors_by_position(monitors.iter());

    if monitor_index >= sorted_monitors.len() {
        log::error!(
            "Invalid monitor index: {} >= {}",
            monitor_index,
            sorted_monitors.len()
        );
        return Err("Invalid monitor index".to_string());
    }

    let window = app.get_webview_window("main").ok_or("Window not found")?;

    // Get target monitor info
    let target_monitor = sorted_monitors[monitor_index];
    remember_last_monitor(monitor_index, target_monitor);
    let mon_size = target_monitor.size();
    let target_scale = target_monitor.scale_factor();
    let bounds = get_logical_bounds(target_monitor);

    // Generate monitor key for this monitor
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, target_scale);
    log::info!("Target monitor key: {}", monitor_key);

    // Get saved width for this monitor or calculate adaptive width
    let db = &state.db;
    let target_width = match db.get_monitor_window_width(&monitor_key).await {
        Ok(Some(saved_width)) => {
            log::info!(
                "Using saved width for monitor {}: {}",
                monitor_key,
                saved_width
            );
            saved_width
        }
        _ => {
            let adaptive_width = calculate_adaptive_width(bounds.width);
            log::info!(
                "Using adaptive width for monitor {}: {} (monitor logical width: {})",
                monitor_key,
                adaptive_width,
                bounds.width
            );
            adaptive_width
        }
    };
    // Get current window scale
    let current_scale = window.scale_factor().map_err(|e| e.to_string())?;

    // Check if we're moving between monitors with different scale factors
    let scale_differs = (current_scale - target_scale).abs() > 0.01;

    if scale_differs {
        // Two-phase move: first move to target monitor center to update scale factor
        let temp_x = bounds.x + bounds.width / 2;
        let temp_y = bounds.y + bounds.height / 2;

        log::info!(
            "Scale differs ({} -> {}), moving to target monitor first",
            current_scale,
            target_scale
        );

        window
            .set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: temp_x as f64,
                y: temp_y as f64,
            }))
            .map_err(|e| e.to_string())?;

        // Small delay to let the window update its scale factor
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Get current window height (we only adapt width, height is determined by drawer mode)
    let win_size = window.outer_size().map_err(|e| e.to_string())?;
    let win_scale = window.scale_factor().map_err(|e| e.to_string())?;
    let win_logical_height = (win_size.height as f64 / win_scale) as i32;

    // Set the new width
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: target_width as f64,
            height: win_logical_height as f64,
        }))
        .map_err(|e| e.to_string())?;

    log::info!(
        "Window size set to: {}x{}",
        target_width,
        win_logical_height
    );
    log::info!(
        "Target monitor[{}]: pos=({}, {}), logical size={}x{}, scale={}",
        monitor_index,
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        target_scale
    );

    // Calculate final position based on anchor using logical coordinates
    let (x, y) = match anchor.as_str() {
        "bottom" => {
            let x = bounds.x + (bounds.width - target_width) / 2;
            let y = bounds.y + bounds.height - win_logical_height;
            (x, y)
        }
        "top" => {
            let x = bounds.x + (bounds.width - target_width) / 2;
            let y = bounds.y;
            (x, y)
        }
        "center" => {
            let x = bounds.x + (bounds.width - target_width) / 2;
            let y = bounds.y + (bounds.height - win_logical_height) / 2;
            (x, y)
        }
        _ => {
            let x = bounds.x + (bounds.width - target_width) / 2;
            let y = bounds.y + bounds.height - win_logical_height;
            (x, y)
        }
    };

    log::info!("Setting final window position (logical) to: ({}, {})", x, y);

    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: x as f64,
            y: y as f64,
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_current_monitor_index(app: tauri::AppHandle) -> Result<usize, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;

    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    let sorted_monitors = sort_monitors_by_position(monitors.iter());

    // Find which monitor the window center is on
    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;

    for (sorted_index, monitor) in sorted_monitors.iter().enumerate() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();

        if win_center_x >= mon_pos.x
            && win_center_x < mon_pos.x + mon_size.width as i32
            && win_center_y >= mon_pos.y
            && win_center_y < mon_pos.y + mon_size.height as i32
        {
            // Update the saved monitor index
            remember_last_monitor(sorted_index, monitor);
            return Ok(sorted_index);
        }
    }

    if let Some((saved_index, _)) = resolve_last_used_monitor(&app, &sorted_monitors) {
        return Ok(saved_index);
    }

    Ok(0)
}

#[tauri::command]
pub async fn toggle_always_on_top(app: tauri::AppHandle) -> Result<bool, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let current = window.is_always_on_top().map_err(|e| e.to_string())?;
    window
        .set_always_on_top(!current)
        .map_err(|e| e.to_string())?;
    Ok(!current)
}

#[tauri::command]
pub async fn get_current_monitor_info(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<CurrentMonitorInfo, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;

    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    let sorted_monitors = sort_monitors_by_position(monitors.iter());

    // Find which monitor the window center is on
    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;

    let mut found_monitor = None;
    for (sorted_index, monitor) in sorted_monitors.iter().enumerate() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();

        if win_center_x >= mon_pos.x
            && win_center_x < mon_pos.x + mon_size.width as i32
            && win_center_y >= mon_pos.y
            && win_center_y < mon_pos.y + mon_size.height as i32
        {
            found_monitor = Some((sorted_index, *monitor));
            break;
        }
    }

    let (monitor_index, monitor) = found_monitor
        .or_else(|| sorted_monitors.first().map(|m| (0, *m)))
        .ok_or("No monitors available")?;

    let bounds = get_logical_bounds(monitor);
    let monitor_key = generate_monitor_key(
        monitor.size().width,
        monitor.size().height,
        monitor.scale_factor(),
    );

    let db = &state.db;
    let saved_width = db
        .get_monitor_window_width(&monitor_key)
        .await
        .unwrap_or(None);

    Ok(CurrentMonitorInfo {
        monitor_key,
        monitor_index,
        monitor_width: bounds.width,
        saved_window_width: saved_width,
    })
}

#[tauri::command]
pub async fn save_window_width_for_monitor(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    width: i32,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;

    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    // Find which monitor the window center is on
    let win_center_x = win_pos.x + win_size.width as i32 / 2;
    let win_center_y = win_pos.y + win_size.height as i32 / 2;

    let mut found_monitor = None;
    for monitor in monitors.iter() {
        let mon_pos = monitor.position();
        let mon_size = monitor.size();

        if win_center_x >= mon_pos.x
            && win_center_x < mon_pos.x + mon_size.width as i32
            && win_center_y >= mon_pos.y
            && win_center_y < mon_pos.y + mon_size.height as i32
        {
            found_monitor = Some(monitor);
            break;
        }
    }

    let monitor = match found_monitor {
        Some(m) => m,
        None => monitors.first().ok_or("No monitors available")?,
    };
    let mon_size = monitor.size();
    let scale = monitor.scale_factor();
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, scale);

    log::info!("Saving window width {} for monitor {}", width, monitor_key);

    let db = &state.db;
    db.save_monitor_window_width(&monitor_key, width)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn snap_to_bottom(app: tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;

    // Find which monitor the window is on
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    let sorted_monitors = sort_monitors_by_position(monitors.iter());

    let mut target_monitor = sorted_monitors
        .first()
        .copied()
        .ok_or("No monitors found")?;
    let mut found_index = 0usize;

    for (sorted_index, monitor) in sorted_monitors.iter().enumerate() {
        let scale = monitor.scale_factor();
        let bounds = get_logical_bounds(monitor);

        // win_pos is already in physical pixels, convert to logical
        let win_logical_x = (win_pos.x as f64 / scale) as i32;
        let win_logical_y = (win_pos.y as f64 / scale) as i32;
        let win_logical_width = (win_size.width as f64 / scale) as i32;
        let win_logical_height = (win_size.height as f64 / scale) as i32;

        let win_center_x = win_logical_x + win_logical_width / 2;
        let win_center_y = win_logical_y + win_logical_height / 2;

        if win_center_x >= bounds.x
            && win_center_x < bounds.x + bounds.width
            && win_center_y >= bounds.y
            && win_center_y < bounds.y + bounds.height
        {
            target_monitor = *monitor;
            found_index = sorted_index;
            break;
        }
    }

    remember_last_monitor(found_index, target_monitor);

    let scale = target_monitor.scale_factor();
    let bounds = get_logical_bounds(target_monitor);

    // Convert to logical coordinates
    let win_logical_x = (win_pos.x as f64 / scale) as i32;
    let win_logical_height = (win_size.height as f64 / scale) as i32;

    // Keep x position, snap y to bottom (using logical coordinates)
    let new_y = bounds.y + bounds.height - win_logical_height;

    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: win_logical_x as f64,
            y: new_y as f64,
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn snap_to_edge(app: tauri::AppHandle, threshold: i32) -> Result<SnapResult, String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    let win_pos = window.outer_position().map_err(|e| e.to_string())?;
    let win_size = window.outer_size().map_err(|e| e.to_string())?;

    // Find which monitor the window is on
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    let sorted_monitors = sort_monitors_by_position(monitors.iter());

    let mut target_monitor = sorted_monitors
        .first()
        .copied()
        .ok_or("No monitors found")?;
    let mut found_index = 0usize;

    // Determine which monitor the window is on based on window center
    for (sorted_index, monitor) in sorted_monitors.iter().enumerate() {
        let scale = monitor.scale_factor();
        let bounds = get_logical_bounds(monitor);

        // win_pos is in physical pixels, convert to logical
        let win_logical_x = (win_pos.x as f64 / scale) as i32;
        let win_logical_y = (win_pos.y as f64 / scale) as i32;
        let win_logical_width = (win_size.width as f64 / scale) as i32;
        let win_logical_height = (win_size.height as f64 / scale) as i32;

        let win_center_x = win_logical_x + win_logical_width / 2;
        let win_center_y = win_logical_y + win_logical_height / 2;

        if win_center_x >= bounds.x
            && win_center_x < bounds.x + bounds.width
            && win_center_y >= bounds.y
            && win_center_y < bounds.y + bounds.height
        {
            target_monitor = *monitor;
            found_index = sorted_index;
            break;
        }
    }

    remember_last_monitor(found_index, target_monitor);

    let scale = target_monitor.scale_factor();
    let bounds = get_logical_bounds(target_monitor);

    // Convert all measurements to logical coordinates
    let win_logical_x = (win_pos.x as f64 / scale) as i32;
    let win_logical_y = (win_pos.y as f64 / scale) as i32;
    let win_logical_width = (win_size.width as f64 / scale) as i32;
    let win_logical_height = (win_size.height as f64 / scale) as i32;

    // Calculate work area (excluding menu bar and dock on macOS)
    #[cfg(target_os = "macos")]
    let (work_top, work_bottom) = {
        let menu_bar_height = 25;
        let dock_height = 70;
        (
            bounds.y + menu_bar_height,
            bounds.y + bounds.height - dock_height,
        )
    };

    #[cfg(not(target_os = "macos"))]
    let (work_top, work_bottom) = (bounds.y, bounds.y + bounds.height);

    let work_left = bounds.x;
    let work_right = bounds.x + bounds.width;

    // Calculate distances to each edge
    let dist_to_top = win_logical_y - work_top;
    let dist_to_bottom = work_bottom - (win_logical_y + win_logical_height);
    let dist_to_left = win_logical_x - work_left;
    let dist_to_right = work_right - (win_logical_x + win_logical_width);

    let mut snapped_edges: Vec<SnapEdge> = Vec::new();
    let mut new_x = win_logical_x;
    let mut new_y = win_logical_y;

    // Check each edge and snap if within threshold
    if dist_to_top.abs() <= threshold {
        new_y = work_top;
        snapped_edges.push(SnapEdge::Top);
    } else if dist_to_bottom.abs() <= threshold {
        new_y = work_bottom - win_logical_height;
        snapped_edges.push(SnapEdge::Bottom);
    }

    if dist_to_left.abs() <= threshold {
        new_x = work_left;
        snapped_edges.push(SnapEdge::Left);
    } else if dist_to_right.abs() <= threshold {
        new_x = work_right - win_logical_width;
        snapped_edges.push(SnapEdge::Right);
    }

    let snapped = !snapped_edges.is_empty();

    // Apply new position if snapped
    if snapped {
        window
            .set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: new_x as f64,
                y: new_y as f64,
            }))
            .map_err(|e| e.to_string())?;
        log::info!(
            "snap_to_edge: snapped to {:?} at ({}, {})",
            snapped_edges,
            new_x,
            new_y
        );
    } else {
        log::info!(
            "snap_to_edge: no snap (distances: top={}, bottom={}, left={}, right={})",
            dist_to_top,
            dist_to_bottom,
            dist_to_left,
            dist_to_right
        );
    }

    Ok(SnapResult {
        snapped,
        edges: snapped_edges,
        position: WindowPosition {
            x: new_x,
            y: new_y,
            width: win_size.width,
            height: win_size.height,
        },
    })
}

#[tauri::command]
pub async fn set_drawer_collapsed(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    collapsed: bool,
) -> Result<(), String> {
    // Legacy wrapper for backwards compatibility
    let mode = if collapsed { "collapsed" } else { "expanded" };
    set_drawer_mode(app, state, mode.to_string(), None).await
}

#[tauri::command]
pub async fn set_drawer_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mode: String,
    preferred_height: Option<i32>,
) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("Window not found")?;

    // Get monitors and sort by position (left to right)
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let sorted_monitors = sort_monitors_by_position(monitors.iter());

    if sorted_monitors.is_empty() {
        return Err("No monitors found".to_string());
    }

    // Check if window is visible
    let is_visible = window.is_visible().unwrap_or(false);

    // Get current window position (in logical pixels) if visible
    let current_logical_x = if is_visible {
        let pos = window.outer_position().map_err(|e| e.to_string())?;
        let win_scale = window.scale_factor().map_err(|e| e.to_string())?;
        Some((pos.x as f64 / win_scale) as i32)
    } else {
        None
    };

    // Find which monitor to use based on current X position or saved index
    let (monitor_index, target_monitor) = if let Some(current_x) = current_logical_x {
        // Window is visible - find monitor containing current X position
        let mut found_idx = 0;
        let mut found_monitor = sorted_monitors[0];

        for (idx, monitor) in sorted_monitors.iter().enumerate() {
            let bounds = get_logical_bounds(monitor);

            // Check if current_x is within this monitor's horizontal bounds
            if current_x >= bounds.x && current_x < bounds.x + bounds.width {
                found_idx = idx;
                found_monitor = *monitor;
                break;
            }

            // If X is past this monitor, this monitor becomes the candidate
            if current_x >= bounds.x {
                found_idx = idx;
                found_monitor = *monitor;
            }
        }

        log::info!(
            "set_drawer_mode: window visible at x={}, found monitor index={}",
            current_x,
            found_idx
        );
        (found_idx, found_monitor)
    } else {
        // Window is not visible - restore the last-used monitor when available.
        resolve_last_used_monitor(&app, &sorted_monitors).unwrap_or((0, sorted_monitors[0]))
    };

    remember_last_monitor(monitor_index, target_monitor);

    let bounds = get_logical_bounds(target_monitor);

    // Generate monitor key for this monitor
    let monitor_size = target_monitor.size();
    let scale = target_monitor.scale_factor();
    let monitor_key = generate_monitor_key(monitor_size.width, monitor_size.height, scale);

    // Get saved width for this monitor or calculate adaptive width
    let db = &state.db;
    let saved_width = match db.get_monitor_window_width(&monitor_key).await {
        Ok(Some(w)) => {
            log::info!(
                "set_drawer_mode: Using saved width {} for monitor {}",
                w,
                monitor_key
            );
            w
        }
        _ => {
            let adaptive = calculate_adaptive_width(bounds.width);
            log::info!(
                "set_drawer_mode: Using adaptive width {} for monitor {}",
                adaptive,
                monitor_key
            );
            adaptive
        }
    };
    // Set new height based on mode
    let popup_min_height = 350;
    let popup_max_height = ((bounds.height as f64 * 0.72).round() as i32).min(760);
    let popup_max_height = popup_max_height.max(popup_min_height);
    let new_logical_height = match mode.as_str() {
        "collapsed" => 48, // Just header
        "expanded" => 280, // History view (header ~48px + padding ~24px + card 192px)
        "full" => 450,     // Settings/Glossary view
        "popup" => preferred_height
            .unwrap_or(popup_min_height)
            .clamp(popup_min_height, popup_max_height),
        _ => 280,
    };

    // Set new size using logical coordinates
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: saved_width as f64,
            height: new_logical_height as f64,
        }))
        .map_err(|e| e.to_string())?;

    // Calculate position
    let new_x = if let Some(current_x) = current_logical_x {
        // Keep current X, but clamp to current monitor bounds
        let min_x = bounds.x;
        let max_x = bounds.x + bounds.width - saved_width;
        current_x.clamp(min_x, max_x)
    } else {
        // Center horizontally on the monitor
        bounds.x + (bounds.width - saved_width) / 2
    };
    let new_y = bounds.y + bounds.height - new_logical_height;

    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: new_x as f64,
            y: new_y as f64,
        }))
        .map_err(|e| e.to_string())?;

    log::info!(
        "set_drawer_mode: mode={}, monitor={}, size={}x{}, pos=({}, {}), preferred_height={:?}, popup_height_range={}..={}",
        mode,
        monitor_index,
        saved_width,
        new_logical_height,
        new_x,
        new_y,
        preferred_height,
        popup_min_height,
        popup_max_height
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{resolve_saved_monitor_index, LastMonitorState, MonitorSelectionCandidate};

    #[test]
    fn resolve_saved_monitor_index_matches_saved_signature_after_reorder() {
        let saved_state = LastMonitorState {
            signature: "studio|1920x1080@1.00".to_string(),
            monitor_key: "1920x1080@1.00".to_string(),
            position_x: 3432,
            position_y: 0,
        };
        let candidates = vec![
            MonitorSelectionCandidate {
                signature: "builtin|1512x982@2.00".to_string(),
                monitor_key: "1512x982@2.00".to_string(),
                position_x: 0,
                position_y: 0,
            },
            MonitorSelectionCandidate {
                signature: "studio|1920x1080@1.00".to_string(),
                monitor_key: "1920x1080@1.00".to_string(),
                position_x: 3432,
                position_y: 0,
            },
            MonitorSelectionCandidate {
                signature: "studio|1920x1080@1.00".to_string(),
                monitor_key: "1920x1080@1.00".to_string(),
                position_x: 1512,
                position_y: 0,
            },
        ];

        assert_eq!(
            resolve_saved_monitor_index(Some(&saved_state), &candidates),
            Some(1)
        );
    }

    #[test]
    fn resolve_saved_monitor_index_chooses_nearest_duplicate_monitor() {
        let saved_state = LastMonitorState {
            signature: "studio|1920x1080@1.00".to_string(),
            monitor_key: "1920x1080@1.00".to_string(),
            position_x: 1500,
            position_y: 0,
        };
        let candidates = vec![
            MonitorSelectionCandidate {
                signature: "studio|1920x1080@1.00".to_string(),
                monitor_key: "1920x1080@1.00".to_string(),
                position_x: 0,
                position_y: 0,
            },
            MonitorSelectionCandidate {
                signature: "studio|1920x1080@1.00".to_string(),
                monitor_key: "1920x1080@1.00".to_string(),
                position_x: 1512,
                position_y: 0,
            },
        ];

        assert_eq!(
            resolve_saved_monitor_index(Some(&saved_state), &candidates),
            Some(1)
        );
    }

    #[test]
    fn resolve_saved_monitor_index_returns_none_when_monitor_is_missing() {
        let saved_state = LastMonitorState {
            signature: "studio|1920x1080@1.00".to_string(),
            monitor_key: "1920x1080@1.00".to_string(),
            position_x: 1512,
            position_y: 0,
        };
        let candidates = vec![MonitorSelectionCandidate {
            signature: "builtin|1512x982@2.00".to_string(),
            monitor_key: "1512x982@2.00".to_string(),
            position_x: 0,
            position_y: 0,
        }];

        assert_eq!(
            resolve_saved_monitor_index(Some(&saved_state), &candidates),
            None
        );
    }

    #[test]
    fn resolve_saved_monitor_index_falls_back_to_monitor_key_when_name_changes() {
        let saved_state = LastMonitorState {
            signature: "Built-in Retina Display|1512x982@2.00".to_string(),
            monitor_key: "1512x982@2.00".to_string(),
            position_x: 0,
            position_y: 0,
        };
        let candidates = vec![
            MonitorSelectionCandidate {
                signature: "|1512x982@2.00".to_string(),
                monitor_key: "1512x982@2.00".to_string(),
                position_x: 0,
                position_y: 0,
            },
            MonitorSelectionCandidate {
                signature: "studio|1920x1080@1.00".to_string(),
                monitor_key: "1920x1080@1.00".to_string(),
                position_x: 1512,
                position_y: 0,
            },
        ];

        assert_eq!(
            resolve_saved_monitor_index(Some(&saved_state), &candidates),
            Some(0)
        );
    }
}

#[tauri::command]
pub async fn open_postit_editor(
    app: tauri::AppHandle,
    mode: String,
    item_id: Option<String>,
) -> Result<(), String> {
    use tauri::WebviewUrl;
    use tauri::WebviewWindowBuilder;

    let normalized_mode = if mode == "edit" { "edit" } else { "create" };
    let payload = PostItEditorOpenPayload {
        mode: normalized_mode.to_string(),
        item_id,
    };

    // Build URL with parameters
    let mut url_params = vec![
        "window=editor".to_string(),
        format!("mode={}", payload.mode),
    ];

    if let Some(id) = payload.item_id.as_ref() {
        url_params.push(format!("itemId={}", urlencoding::encode(id)));
    }

    let url = format!("index.html?{}", url_params.join("&"));

    log::info!("Opening PostIt editor window: {}", url);

    // Get main window position to determine which monitor to use
    let main_window = app.get_webview_window("main");
    let (editor_x, editor_y) = if let Some(main_win) = main_window {
        if let (Ok(main_pos), Ok(main_size), Ok(scale)) = (
            main_win.outer_position(),
            main_win.outer_size(),
            main_win.scale_factor(),
        ) {
            // Calculate center of main window
            let main_center_x = main_pos.x as f64 + (main_size.width as f64 / 2.0);
            let main_center_y = main_pos.y as f64 + (main_size.height as f64 / 2.0);

            // Editor window size (in physical pixels)
            let editor_width = 500.0 * scale;
            let editor_height = 400.0 * scale;

            // Position editor centered on main window's center
            let editor_x = (main_center_x - editor_width / 2.0) as i32;
            let editor_y = (main_center_y - editor_height / 2.0) as i32;

            log::info!(
                "Positioning editor at ({}, {}) based on main window at ({}, {})",
                editor_x,
                editor_y,
                main_pos.x,
                main_pos.y
            );

            (Some(editor_x), Some(editor_y))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    if let Some(window) = app.get_webview_window(POSTIT_EDITOR_LABEL) {
        if let (Some(x), Some(y)) = (editor_x, editor_y) {
            window
                .set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }))
                .map_err(|e| format!("Failed to position existing editor window: {}", e))?;
        } else {
            window
                .center()
                .map_err(|e| format!("Failed to center existing editor window: {}", e))?;
        }

        window
            .emit("postit_editor_open", payload)
            .map_err(|e| format!("Failed to notify existing editor window: {}", e))?;
        window
            .show()
            .map_err(|e| format!("Failed to show existing editor window: {}", e))?;
        window
            .set_focus()
            .map_err(|e| format!("Failed to focus existing editor window: {}", e))?;

        return Ok(());
    }

    // Create the new window
    let mut builder =
        WebviewWindowBuilder::new(&app, POSTIT_EDITOR_LABEL, WebviewUrl::App(url.into()))
            .title("메모 편집")
            .inner_size(500.0, 400.0)
            .min_inner_size(400.0, 300.0)
            .decorations(true)
            .always_on_top(true);

    // Set position if we calculated it, otherwise center
    if let (Some(x), Some(y)) = (editor_x, editor_y) {
        builder = builder.position(x as f64, y as f64);
    } else {
        builder = builder.center();
    }

    let window = builder
        .build()
        .map_err(|e| format!("Failed to create editor window: {}", e))?;

    // Focus the window
    window
        .set_focus()
        .map_err(|e| format!("Failed to focus window: {}", e))?;

    Ok(())
}
