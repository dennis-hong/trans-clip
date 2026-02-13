use crate::utils::monitor::{
    calculate_adaptive_width, generate_monitor_key, get_logical_bounds, sort_monitors_by_position,
};
use crate::AppState;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{Manager, State};

use super::types::{CurrentMonitorInfo, MonitorInfo, SnapEdge, SnapResult, WindowPosition};

// Store the last valid monitor index to preserve position across hide/show cycles
static LAST_MONITOR_INDEX: AtomicUsize = AtomicUsize::new(0);

pub fn get_last_monitor_index() -> usize {
    LAST_MONITOR_INDEX.load(Ordering::SeqCst)
}

/// Update LAST_MONITOR_INDEX based on current mouse cursor position
/// This is called before showing the window to ensure it appears on the correct monitor
pub fn update_monitor_from_cursor(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

        // Get current mouse position
        let source = match CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
            Ok(s) => s,
            Err(_) => {
                log::warn!("Failed to create event source for cursor position");
                return;
            }
        };
        let event = match CGEvent::new(source) {
            Ok(e) => e,
            Err(_) => {
                log::warn!("Failed to create event for cursor position");
                return;
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
                return;
            }
        };

        let sorted_monitors = sort_monitors_by_position(monitors.iter());

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
                LAST_MONITOR_INDEX.store(idx, Ordering::SeqCst);
                return;
            }
        }

        // Fallback: try with logical coordinates directly (for single-scale setups)
        for (idx, monitor) in sorted_monitors.iter().enumerate() {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();

            let mon_logical_width = (mon_size.width as f64 / scale) as i32;
            let mon_logical_height = (mon_size.height as f64 / scale) as i32;

            if cursor_x >= mon_pos.x
                && cursor_x < mon_pos.x + mon_logical_width
                && cursor_y >= mon_pos.y
                && cursor_y < mon_pos.y + mon_logical_height
            {
                log::info!("Cursor is on monitor {} (logical fallback)", idx);
                LAST_MONITOR_INDEX.store(idx, Ordering::SeqCst);
                return;
            }
        }

        log::warn!("Could not determine monitor from cursor position");
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        log::info!("Cursor-based monitor detection not implemented for this platform");
    }
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
            let is_primary = primary.as_ref().is_some_and(|p| {
                p.position() == monitor.position() && p.size() == monitor.size()
            });

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

    // Save the monitor index for later use
    LAST_MONITOR_INDEX.store(monitor_index, Ordering::SeqCst);
    log::info!(
        "move_to_monitor: saved LAST_MONITOR_INDEX={}",
        monitor_index
    );

    let window = app.get_webview_window("main").ok_or("Window not found")?;

    // Get target monitor info
    let target_monitor = sorted_monitors[monitor_index];
    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let target_scale = target_monitor.scale_factor();

    // Convert target monitor size to logical
    let mon_logical_width = (mon_size.width as f64 / target_scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / target_scale) as i32;

    // Generate monitor key for this monitor
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, target_scale);
    log::info!("Target monitor key: {}", monitor_key);

    // Get saved width for this monitor or calculate adaptive width
    let db = state.db.lock().await;
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
            let adaptive_width = calculate_adaptive_width(mon_logical_width);
            log::info!(
                "Using adaptive width for monitor {}: {} (monitor logical width: {})",
                monitor_key,
                adaptive_width,
                mon_logical_width
            );
            adaptive_width
        }
    };
    drop(db);

    // Get current window scale
    let current_scale = window.scale_factor().map_err(|e| e.to_string())?;

    // Check if we're moving between monitors with different scale factors
    let scale_differs = (current_scale - target_scale).abs() > 0.01;

    if scale_differs {
        // Two-phase move: first move to target monitor center to update scale factor
        let temp_x = mon_pos.x + mon_logical_width / 2;
        let temp_y = mon_pos.y + mon_logical_height / 2;

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
        mon_pos.x,
        mon_pos.y,
        mon_logical_width,
        mon_logical_height,
        target_scale
    );

    // Calculate final position based on anchor using logical coordinates
    let (x, y) = match anchor.as_str() {
        "bottom" => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y + mon_logical_height - win_logical_height;
            (x, y)
        }
        "top" => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y;
            (x, y)
        }
        "center" => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y + (mon_logical_height - win_logical_height) / 2;
            (x, y)
        }
        _ => {
            let x = mon_pos.x + (mon_logical_width - target_width) / 2;
            let y = mon_pos.y + mon_logical_height - win_logical_height;
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
            LAST_MONITOR_INDEX.store(sorted_index, Ordering::SeqCst);
            return Ok(sorted_index);
        }
    }

    // Default to saved index or first monitor if not found
    let saved_index = LAST_MONITOR_INDEX.load(Ordering::SeqCst);
    if saved_index < sorted_monitors.len() {
        Ok(saved_index)
    } else {
        Ok(0)
    }
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

    let db = state.db.lock().await;
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

    let db = state.db.lock().await;
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
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let scale = monitor.scale_factor();

        // Convert to logical coordinates for comparison
        let mon_logical_width = (mon_size.width as f64 / scale) as i32;
        let mon_logical_height = (mon_size.height as f64 / scale) as i32;

        // win_pos is already in physical pixels, convert to logical
        let win_logical_x = (win_pos.x as f64 / scale) as i32;
        let win_logical_y = (win_pos.y as f64 / scale) as i32;
        let win_logical_width = (win_size.width as f64 / scale) as i32;
        let win_logical_height = (win_size.height as f64 / scale) as i32;

        let win_center_x = win_logical_x + win_logical_width / 2;
        let win_center_y = win_logical_y + win_logical_height / 2;

        if win_center_x >= mon_pos.x
            && win_center_x < mon_pos.x + mon_logical_width
            && win_center_y >= mon_pos.y
            && win_center_y < mon_pos.y + mon_logical_height
        {
            target_monitor = *monitor;
            found_index = sorted_index;
            break;
        }
    }

    // Update the saved monitor index
    LAST_MONITOR_INDEX.store(found_index, Ordering::SeqCst);
    log::info!("snap_to_bottom: updated LAST_MONITOR_INDEX={}", found_index);

    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();

    // Convert to logical coordinates
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;
    let win_logical_x = (win_pos.x as f64 / scale) as i32;
    let win_logical_height = (win_size.height as f64 / scale) as i32;

    // Keep x position, snap y to bottom (using logical coordinates)
    let new_y = mon_pos.y + mon_logical_height - win_logical_height;

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
        let mon_pos = monitor.position();
        let mon_size = monitor.size();
        let scale = monitor.scale_factor();

        // Convert to logical coordinates for comparison
        let mon_logical_width = (mon_size.width as f64 / scale) as i32;
        let mon_logical_height = (mon_size.height as f64 / scale) as i32;

        // win_pos is in physical pixels, convert to logical
        let win_logical_x = (win_pos.x as f64 / scale) as i32;
        let win_logical_y = (win_pos.y as f64 / scale) as i32;
        let win_logical_width = (win_size.width as f64 / scale) as i32;
        let win_logical_height = (win_size.height as f64 / scale) as i32;

        let win_center_x = win_logical_x + win_logical_width / 2;
        let win_center_y = win_logical_y + win_logical_height / 2;

        if win_center_x >= mon_pos.x
            && win_center_x < mon_pos.x + mon_logical_width
            && win_center_y >= mon_pos.y
            && win_center_y < mon_pos.y + mon_logical_height
        {
            target_monitor = *monitor;
            found_index = sorted_index;
            break;
        }
    }

    // Update the saved monitor index
    LAST_MONITOR_INDEX.store(found_index, Ordering::SeqCst);
    log::info!("snap_to_edge: updated LAST_MONITOR_INDEX={}", found_index);

    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();

    // Convert all measurements to logical coordinates
    let mon_logical_width = (mon_size.width as f64 / scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;
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
            mon_pos.y + menu_bar_height,
            mon_pos.y + mon_logical_height - dock_height,
        )
    };

    #[cfg(not(target_os = "macos"))]
    let (work_top, work_bottom) = (mon_pos.y, mon_pos.y + mon_logical_height);

    let work_left = mon_pos.x;
    let work_right = mon_pos.x + mon_logical_width;

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
    set_drawer_mode(app, state, mode.to_string()).await
}

#[tauri::command]
pub async fn set_drawer_mode(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    mode: String,
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
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let scale = monitor.scale_factor();
            let mon_logical_width = (mon_size.width as f64 / scale) as i32;

            // Check if current_x is within this monitor's horizontal bounds
            if current_x >= mon_pos.x && current_x < mon_pos.x + mon_logical_width {
                found_idx = idx;
                found_monitor = *monitor;
                break;
            }

            // If X is past this monitor, this monitor becomes the candidate
            if current_x >= mon_pos.x {
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
        // Window is not visible - use saved monitor index
        let saved_index = LAST_MONITOR_INDEX.load(Ordering::SeqCst);
        let idx = if saved_index < sorted_monitors.len() {
            saved_index
        } else {
            0
        };
        log::info!(
            "set_drawer_mode: window not visible, using saved monitor index={}",
            idx
        );
        (idx, sorted_monitors[idx])
    };

    // Update LAST_MONITOR_INDEX
    LAST_MONITOR_INDEX.store(monitor_index, Ordering::SeqCst);

    let mon_pos = target_monitor.position();
    let mon_size = target_monitor.size();
    let scale = target_monitor.scale_factor();

    // Convert monitor size to logical coordinates
    let mon_logical_width = (mon_size.width as f64 / scale) as i32;
    let mon_logical_height = (mon_size.height as f64 / scale) as i32;

    // Generate monitor key for this monitor
    let monitor_key = generate_monitor_key(mon_size.width, mon_size.height, scale);

    // Get saved width for this monitor or calculate adaptive width
    let db = state.db.lock().await;
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
            let adaptive = calculate_adaptive_width(mon_logical_width);
            log::info!(
                "set_drawer_mode: Using adaptive width {} for monitor {}",
                adaptive,
                monitor_key
            );
            adaptive
        }
    };
    drop(db);

    // Set new height based on mode
    let new_logical_height = match mode.as_str() {
        "collapsed" => 48,  // Just header
        "expanded" => 280,  // History view (header ~48px + padding ~24px + card 192px)
        "full" => 450,      // Settings/Glossary view
        "popup" => 350,     // Translation/Polish popup view
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
        let min_x = mon_pos.x;
        let max_x = mon_pos.x + mon_logical_width - saved_width;
        current_x.clamp(min_x, max_x)
    } else {
        // Center horizontally on the monitor
        mon_pos.x + (mon_logical_width - saved_width) / 2
    };
    let new_y = mon_pos.y + mon_logical_height - new_logical_height;

    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: new_x as f64,
            y: new_y as f64,
        }))
        .map_err(|e| e.to_string())?;

    log::info!(
        "set_drawer_mode: mode={}, monitor={}, size={}x{}, pos=({}, {})",
        mode,
        monitor_index,
        saved_width,
        new_logical_height,
        new_x,
        new_y
    );

    Ok(())
}

#[tauri::command]
pub async fn open_postit_editor(
    app: tauri::AppHandle,
    mode: String,
    item_id: Option<String>,
    initial_content: Option<String>,
) -> Result<(), String> {
    use tauri::WebviewUrl;
    use tauri::WebviewWindowBuilder;

    // Generate a unique window label
    let window_label = format!("postit-editor-{}", uuid::Uuid::new_v4());

    // Build URL with parameters
    let mut url_params = vec![format!("window=editor"), format!("mode={}", mode)];

    if let Some(id) = item_id {
        url_params.push(format!("itemId={}", urlencoding::encode(&id)));
    }

    if let Some(content) = initial_content {
        url_params.push(format!("content={}", urlencoding::encode(&content)));
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

    // Create the new window
    let mut builder = WebviewWindowBuilder::new(&app, &window_label, WebviewUrl::App(url.into()))
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
