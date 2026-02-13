use tauri::Monitor;

/// Sort monitors by position (left to right, top to bottom)
pub fn sort_monitors_by_position<'a>(
    monitors: impl Iterator<Item = &'a Monitor>,
) -> Vec<&'a Monitor> {
    let mut sorted: Vec<_> = monitors.collect();
    sorted.sort_by(|a, b| {
        let pos_a = a.position();
        let pos_b = b.position();
        match pos_a.x.cmp(&pos_b.x) {
            std::cmp::Ordering::Equal => pos_a.y.cmp(&pos_b.y),
            other => other,
        }
    });
    sorted
}

/// Generate a unique key for a monitor based on its resolution and scale
pub fn generate_monitor_key(width: u32, height: u32, scale_factor: f64) -> String {
    format!("{}x{}@{:.2}", width, height, scale_factor)
}

/// Calculate adaptive window width based on monitor width
pub fn calculate_adaptive_width(monitor_logical_width: i32) -> i32 {
    let base_width = if monitor_logical_width >= 2560 {
        // Ultra-wide: 60%
        ((monitor_logical_width as f64) * 0.6) as i32
    } else if monitor_logical_width >= 1920 {
        // Wide: 70%
        ((monitor_logical_width as f64) * 0.7) as i32
    } else {
        // Standard: 80%
        ((monitor_logical_width as f64) * 0.8) as i32
    };

    base_width.clamp(800, 1600)
}

/// Get logical bounds of a monitor
pub struct MonitorLogicalBounds {
    pub width: i32,
}

pub fn get_logical_bounds(monitor: &Monitor) -> MonitorLogicalBounds {
    let size = monitor.size();
    let scale = monitor.scale_factor();

    MonitorLogicalBounds {
        width: (size.width as f64 / scale) as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::{calculate_adaptive_width, generate_monitor_key};

    #[test]
    fn calculate_adaptive_width_respects_breakpoints_and_clamps() {
        assert_eq!(calculate_adaptive_width(1440), 1152);
        assert_eq!(calculate_adaptive_width(1920), 1344);
        assert_eq!(calculate_adaptive_width(3000), 1600);
        assert_eq!(calculate_adaptive_width(700), 800);
    }

    #[test]
    fn generate_monitor_key_is_stable() {
        let key = generate_monitor_key(3024, 1964, 2.0);
        assert_eq!(key, "3024x1964@2.00");
    }
}
