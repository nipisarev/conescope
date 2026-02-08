use gpui::prelude::*;
use gpui::{MouseButton, MouseDownEvent, ScrollHandle, SharedString, div, point, px, rgba};

// Scrollbar dimensions
const SCROLLBAR_WIDTH: f32 = 8.0;
const THUMB_MIN_HEIGHT: f32 = 20.0;
const THUMB_RADIUS: f32 = 4.0;
const TRACK_PADDING: f32 = 2.0;

// Opacity levels
const OPACITY_HIDDEN: f32 = 0.0;
const OPACITY_VISIBLE: f32 = 0.6;
const OPACITY_HOVER: f32 = 0.85;
const OPACITY_ACTIVE: f32 = 1.0;

// Colors
const THUMB_COLOR: u32 = 0x8888_88ff;
const ACTIVE_COLOR: u32 = 0x007a_ccff;

/// Scrollbar hover/drag state, stored by the parent view.
#[derive(Debug, Default)]
pub struct ScrollbarState {
    /// Cursor is inside the scrollable area.
    pub container_hovered: bool,
    /// Cursor is over the thumb.
    pub thumb_hovered: bool,
    /// Active drag operation.
    pub drag: Option<ScrollbarDrag>,
}

/// State for an active scrollbar thumb drag.
#[derive(Debug, Clone, Copy)]
pub struct ScrollbarDrag {
    /// Mouse Y at drag start (window coords).
    pub start_mouse_y: f32,
    /// `scroll_handle.offset().y` at drag start.
    pub start_offset_y: f32,
}

// Right margin so the track doesn't touch the window edge.
const TRACK_RIGHT_MARGIN: f32 = 4.0;

impl ScrollbarState {
    fn opacity(&self) -> f32 {
        if self.drag.is_some() {
            return OPACITY_ACTIVE;
        }
        if self.thumb_hovered {
            return OPACITY_HOVER;
        }
        if self.container_hovered {
            return OPACITY_VISIBLE;
        }
        OPACITY_HIDDEN
    }

    fn thumb_color(&self) -> u32 {
        if self.drag.is_some() {
            ACTIVE_COLOR
        } else {
            THUMB_COLOR
        }
    }
}

/// Compute thumb geometry from scroll handle state.
/// Returns `(thumb_height, thumb_top, max_scroll)` or `None` if content fits.
fn thumb_geometry(scroll_handle: &ScrollHandle) -> Option<(f32, f32, f32)> {
    let viewport_h = f32::from(scroll_handle.bounds().size.height);
    let max_scroll = f32::from(scroll_handle.max_offset().height);

    if max_scroll <= 0.0 || viewport_h <= 0.0 {
        return None;
    }

    let track_h = viewport_h - TRACK_PADDING * 2.0;
    if track_h <= 0.0 {
        return None;
    }

    let content_h = viewport_h + max_scroll;
    let scroll_y = f32::from(scroll_handle.offset().y).abs();

    let thumb_h = (viewport_h / content_h * track_h)
        .max(THUMB_MIN_HEIGHT)
        .min(track_h);
    let scroll_ratio = if max_scroll > 0.0 {
        scroll_y / max_scroll
    } else {
        0.0
    };
    let thumb_top = TRACK_PADDING + scroll_ratio * (track_h - thumb_h);

    Some((thumb_h, thumb_top, max_scroll))
}

/// Apply a drag delta to scroll position.
/// Call this from the parent's `on_mouse_move` handler when `scrollbar_state.drag.is_some()`.
pub fn apply_drag(scroll_handle: &ScrollHandle, drag: &ScrollbarDrag, current_mouse_y: f32) {
    let Some((_, _, max_scroll)) = thumb_geometry(scroll_handle) else {
        return;
    };

    let viewport_h = f32::from(scroll_handle.bounds().size.height);
    let track_h = viewport_h - TRACK_PADDING * 2.0;
    let content_h = viewport_h + max_scroll;
    let thumb_h = (viewport_h / content_h * track_h)
        .max(THUMB_MIN_HEIGHT)
        .min(track_h);
    let scrollable_track = track_h - thumb_h;

    if scrollable_track <= 0.0 {
        return;
    }

    let mouse_delta = current_mouse_y - drag.start_mouse_y;
    let scroll_delta = mouse_delta / scrollable_track * max_scroll;
    let new_offset_y = (drag.start_offset_y - scroll_delta).clamp(-max_scroll, 0.0);

    let current_x = scroll_handle.offset().x;
    scroll_handle.set_offset(point(current_x, px(new_offset_y)));
}

/// Apply a track click to jump scroll position.
/// Call this from the track's `on_mouse_down` handler.
pub fn apply_track_click(scroll_handle: &ScrollHandle, click_y_in_track: f32) {
    let viewport_h = f32::from(scroll_handle.bounds().size.height);
    let max_scroll = f32::from(scroll_handle.max_offset().height);

    if max_scroll <= 0.0 || viewport_h <= 0.0 {
        return;
    }

    let track_h = viewport_h - TRACK_PADDING * 2.0;
    if track_h <= 0.0 {
        return;
    }

    // Click position as ratio within the track
    let ratio = ((click_y_in_track - TRACK_PADDING) / track_h).clamp(0.0, 1.0);
    let new_offset_y = -(ratio * max_scroll);

    let current_x = scroll_handle.offset().x;
    scroll_handle.set_offset(point(current_x, px(new_offset_y)));
}

/// Callbacks for scrollbar interaction, provided by the parent view.
pub struct ScrollbarCallbacks<H, T, D>
where
    H: Fn(&bool, &mut gpui::Window, &mut gpui::App) + 'static,
    T: Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    D: Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    pub on_thumb_hover: H,
    pub on_track_click: T,
    pub on_drag_start: D,
}

impl<H, T, D> std::fmt::Debug for ScrollbarCallbacks<H, T, D>
where
    H: Fn(&bool, &mut gpui::Window, &mut gpui::App) + 'static,
    T: Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    D: Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScrollbarCallbacks").finish()
    }
}

/// Render an overlay scrollbar for a scrollable container.
///
/// Returns `None` if content fits (no overflow). The returned element should be
/// added as a child of a `relative()` container that wraps the scrollable div.
pub fn render_scrollbar<H, T, D>(
    id: &str,
    scroll_handle: &ScrollHandle,
    state: &ScrollbarState,
    cb: ScrollbarCallbacks<H, T, D>,
) -> Option<gpui::Stateful<gpui::Div>>
where
    H: Fn(&bool, &mut gpui::Window, &mut gpui::App) + 'static,
    T: Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    D: Fn(&MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    let (thumb_h, thumb_top, _max_scroll) = thumb_geometry(scroll_handle)?;
    let opacity = state.opacity();
    let color = state.thumb_color();

    let track_id = SharedString::from(format!("{id}-scrollbar-track"));
    let thumb_id = SharedString::from(format!("{id}-scrollbar-thumb"));

    let on_drag_start = cb.on_drag_start;
    let on_track_click = cb.on_track_click;

    let thumb = div()
        .id(thumb_id)
        .absolute()
        .right(px(0.))
        .top(px(thumb_top))
        .w(px(SCROLLBAR_WIDTH))
        .h(px(thumb_h))
        .rounded(px(THUMB_RADIUS))
        .bg(rgba(color))
        .cursor_pointer()
        .on_hover(cb.on_thumb_hover)
        .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
            cx.stop_propagation();
            on_drag_start(ev, window, cx);
        });

    let track = div()
        .id(track_id)
        .absolute()
        .right(px(TRACK_RIGHT_MARGIN))
        .top_0()
        .bottom_0()
        .w(px(SCROLLBAR_WIDTH))
        .opacity(opacity)
        .child(thumb)
        .on_mouse_down(MouseButton::Left, move |ev, window, cx| {
            cx.stop_propagation();
            on_track_click(ev, window, cx);
        });

    Some(track)
}
