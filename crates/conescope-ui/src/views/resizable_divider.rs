use gpui::prelude::*;
use gpui::{CursorStyle, MouseButton, MouseDownEvent, Pixels, div, px, rgba};

/// Axis of a resizable divider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Vertical bar that drags left/right (resizes width).
    Horizontal,
    /// Horizontal bar that drags up/down (resizes height).
    Vertical,
}

/// Drag target identifier for the parent to track which divider is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragTarget {
    Sidebar,
    Terminal,
}

/// State for an active drag operation, stored by the parent view.
#[derive(Debug, Clone, Copy)]
pub struct DragState {
    pub target: DragTarget,
    pub axis: Axis,
    pub last_pos: f32,
}

impl DragState {
    /// Compute delta from current position and update `last_pos`.
    /// Returns the pixel delta since last call.
    pub fn delta(&mut self, current_pos: f32) -> f32 {
        let d = current_pos - self.last_pos;
        self.last_pos = current_pos;
        d
    }

    /// Extract the relevant coordinate from a mouse position.
    #[must_use]
    pub fn position_component(&self, pos: gpui::Point<Pixels>) -> f32 {
        match self.axis {
            Axis::Horizontal => f32::from(pos.x),
            Axis::Vertical => f32::from(pos.y),
        }
    }
}

/// Render a resizable divider bar.
///
/// The caller must attach an `on_mouse_down` handler to initiate drag tracking
/// at the parent level. The parent's root div should handle `on_mouse_move` and
/// `on_mouse_up`/`on_mouse_up_out` to track the drag.
///
/// # Arguments
/// * `axis` - Direction of the divider
/// * `active` - Whether this divider is currently being dragged
/// * `on_drag_start` - Closure called on `mouse_down` with the starting position
pub fn render_divider(
    axis: Axis,
    active: bool,
    on_drag_start: impl Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    let cursor_style = match axis {
        Axis::Horizontal => CursorStyle::ResizeLeftRight,
        Axis::Vertical => CursorStyle::ResizeUpDown,
    };
    let color = if active {
        rgba(0x007a_ccff)
    } else {
        rgba(0x3c3c_3cff)
    };

    let base = div()
        .cursor(cursor_style)
        .bg(color)
        .hover(|s| s.bg(rgba(0x007a_ccff)))
        .on_mouse_down(MouseButton::Left, on_drag_start);

    match axis {
        Axis::Horizontal => base.w(px(1.)).h_full(),
        Axis::Vertical => base.h(px(1.)).w_full(),
    }
}

/// Clamp a panel size to reasonable bounds.
#[must_use]
pub fn clamp_size(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}
