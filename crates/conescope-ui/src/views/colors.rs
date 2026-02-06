use conescope_core::instance::InstanceStatus;
use gpui::Rgba;

/// Parse a CSS hex color string (#RRGGBB or #RGB) to GPUI `Rgba`.
#[must_use]
pub fn hex_to_rgba(hex: &str) -> Rgba {
    let hex = hex.trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).unwrap_or(128),
            u8::from_str_radix(&hex[2..4], 16).unwrap_or(128),
            u8::from_str_radix(&hex[4..6], 16).unwrap_or(128),
        ),
        3 => (
            u8::from_str_radix(&hex[0..1], 16).unwrap_or(8) * 17,
            u8::from_str_radix(&hex[1..2], 16).unwrap_or(8) * 17,
            u8::from_str_radix(&hex[2..3], 16).unwrap_or(8) * 17,
        ),
        _ => (128, 128, 128),
    };
    Rgba {
        r: f32::from(r) / 255.0,
        g: f32::from(g) / 255.0,
        b: f32::from(b) / 255.0,
        a: 1.0,
    }
}

/// Status dot color for an instance status.
#[must_use]
pub fn status_color(status: InstanceStatus) -> Rgba {
    match status {
        InstanceStatus::Working => hex_to_rgba("#81C784"),
        InstanceStatus::Waiting => hex_to_rgba("#FFB74D"),
        InstanceStatus::Paused => hex_to_rgba("#90A4AE"),
        InstanceStatus::Starting => hex_to_rgba("#64B5F6"),
        InstanceStatus::Stopped => hex_to_rgba("#666666"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_rgba_parses_6_digit() {
        let c = hex_to_rgba("#FF0000");
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g.abs() < 0.01);
        assert!(c.b.abs() < 0.01);
        assert!((c.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn hex_to_rgba_parses_without_hash() {
        let c = hex_to_rgba("00FF00");
        assert!(c.r.abs() < 0.01);
        assert!((c.g - 1.0).abs() < 0.01);
    }

    #[test]
    fn hex_to_rgba_parses_3_digit() {
        let c = hex_to_rgba("#F00");
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g.abs() < 0.01);
    }

    #[test]
    fn status_color_returns_valid_colors() {
        let c = status_color(InstanceStatus::Working);
        assert!(c.r > 0.0);
    }
}
