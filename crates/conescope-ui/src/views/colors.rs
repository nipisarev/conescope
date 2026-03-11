use conescope_core::instance::InstanceStatus;
use conescope_core::project::PROJECT_COLORS;
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

/// Default instance color when `instance.color` is `None` (legacy data).
#[must_use]
pub fn default_instance_color() -> Rgba {
    hex_to_rgba(PROJECT_COLORS[0])
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

#[must_use]
#[allow(
    clippy::many_single_char_names,
    clippy::manual_midpoint,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
pub fn worktree_color_variant(parent_hex: &str, index: usize) -> String {
    let rgba = hex_to_rgba(parent_hex);
    let (r, g, b) = (rgba.r, rgba.g, rgba.b);

    // RGB -> HSL
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let delta = max - min;

    let (h, s) = if delta < 0.001 {
        (0.0, 0.0)
    } else {
        let s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };
        let h = if (max - r).abs() < 0.001 {
            let mut hue = (g - b) / delta;
            if hue < 0.0 {
                hue += 6.0;
            }
            hue
        } else if (max - g).abs() < 0.001 {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };
        (h * 60.0, s)
    };

    // Shift lightness based on index
    let shift = 0.10 + (index as f32) * 0.05;
    let new_l = if l < 0.5 {
        (l + shift).min(0.9)
    } else {
        (l - shift).max(0.1)
    };

    // HSL -> RGB
    let (r_out, g_out, b_out) = hsl_to_rgb(h, s, new_l);

    format!(
        "#{:02X}{:02X}{:02X}",
        (r_out * 255.0).round() as u8,
        (g_out * 255.0).round() as u8,
        (b_out * 255.0).round() as u8,
    )
}

#[allow(clippy::many_single_char_names)]
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < 0.001 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h_norm = h / 360.0;
    (
        hue_to_rgb(p, q, h_norm + 1.0 / 3.0),
        hue_to_rgb(p, q, h_norm),
        hue_to_rgb(p, q, h_norm - 1.0 / 3.0),
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
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

    #[test]
    fn default_instance_color_matches_first_project_color() {
        let expected = hex_to_rgba(PROJECT_COLORS[0]);
        let actual = default_instance_color();
        assert!((actual.r - expected.r).abs() < 0.001);
        assert!((actual.g - expected.g).abs() < 0.001);
        assert!((actual.b - expected.b).abs() < 0.001);
    }

    #[test]
    fn worktree_color_variant_returns_valid_hex() {
        let variant = worktree_color_variant("#4FC3F7", 0);
        assert!(variant.starts_with('#'));
        assert_eq!(variant.len(), 7);
        // Should parse back without error
        let c = hex_to_rgba(&variant);
        assert!(c.r >= 0.0 && c.r <= 1.0);
        assert!(c.g >= 0.0 && c.g <= 1.0);
        assert!(c.b >= 0.0 && c.b <= 1.0);
    }

    #[test]
    fn worktree_color_variant_differs_from_parent() {
        let parent = "#4FC3F7";
        let variant = worktree_color_variant(parent, 0);
        assert_ne!(variant.to_uppercase(), parent.to_uppercase());
    }

    #[test]
    fn worktree_color_variant_differs_by_index() {
        let v0 = worktree_color_variant("#FF5722", 0);
        let v1 = worktree_color_variant("#FF5722", 1);
        let v2 = worktree_color_variant("#FF5722", 2);
        assert_ne!(v0, v1);
        assert_ne!(v1, v2);
    }

    #[test]
    fn worktree_color_variant_handles_grayscale() {
        let variant = worktree_color_variant("#808080", 0);
        assert!(variant.starts_with('#'));
        assert_eq!(variant.len(), 7);
    }

    #[test]
    fn hsl_roundtrip_red() {
        // Pure red: H=0, S=1.0, L=0.5
        let (r, g, b) = hsl_to_rgb(0.0, 1.0, 0.5);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);
    }

    #[test]
    fn hsl_roundtrip_white() {
        let (r, g, b) = hsl_to_rgb(0.0, 0.0, 1.0);
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);
    }
}
