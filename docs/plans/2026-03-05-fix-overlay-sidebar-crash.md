# Fix Overlay Sidebar SIGBUS Crash

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix SIGBUS crash caused by opening/closing child windows inside GPUI's `render()` method.

**Architecture:** Move overlay window lifecycle management from `render()` into `cx.observe()` callback on `app_state`. Also namespace element IDs to prevent cross-window collisions.

**Tech Stack:** GPUI, Rust

---

## Root Cause

`AppView::render()` (lines 485-492) calls `open_overlay_window()` / `close_overlay_window()` during the render pass. `cx.open_window()` triggers a second render cycle while the first is still building the element tree, corrupting GPUI's `GlobalElementId` stack. The `Arc::drop` of a stale `GlobalElementId` writes to freed/remapped memory → SIGBUS.

Contributing: duplicate element IDs (`"sidebar-scroll"`, `"sidebar-pin-toggle"`) across main window and overlay window.

---

### Task 1: Move overlay window open/close into observer

**Files:**
- Modify: `crates/conescope-ui/src/views/app_view.rs`

**Step 1: Add observer in `AppView::new()`**

After the existing settings_store observer (line 114), add a new observer on `app_state` that watches `sidebar_overlay_visible` and manages the overlay window:

```rust
// In AppView::new(), after the settings_store observer:

// Watch sidebar_overlay_visible and manage overlay child window from observer (NOT render).
cx.observe(&app_state, |this: &mut Self, _app_state, cx| {
    let sidebar_overlay_visible = {
        let state = this.app_state.read(cx);
        state.sidebar_overlay_visible
    };

    if sidebar_overlay_visible && !this.sidebar_overlay_was_visible {
        // Defer window open to after current event cycle
        cx.spawn(async move |this, mut cx| {
            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    // Re-check — state may have changed
                    let still_visible = this.app_state.read(cx).sidebar_overlay_visible;
                    if still_visible && this.overlay_window.is_none() {
                        this.open_overlay_window_deferred(cx);
                    }
                });
            });
        }).detach();
    } else if !sidebar_overlay_visible && this.sidebar_overlay_was_visible {
        this.close_overlay_window_deferred(cx);
    }
    this.sidebar_overlay_was_visible = sidebar_overlay_visible;
}).detach();
```

**Step 2: Create deferred open/close methods**

Add two new methods that work without `&mut Window`:

```rust
/// Open overlay window (called from observer, not render).
fn open_overlay_window_deferred(&mut self, cx: &mut gpui::Context<Self>) {
    if self.overlay_window.is_some() {
        return;
    }

    let app_state = self.app_state.clone();
    // We need the main window bounds — use the current window context
    let overlay_width = SIDEBAR_WIDTH + OVERLAY_SIDEBAR_PADDING;

    let overlay_handle = cx.open_window(
        WindowOptions {
            window_bounds: None, // Will be positioned by sync
            titlebar: None,
            focus: false,
            show: true,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            window_background: gpui::WindowBackgroundAppearance::Blurred,
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| OverlaySidebarView::new(app_state, cx));
            cx.new(|cx| gpui_component::Root::new(view, window, cx))
        },
    );

    if let Ok(handle) = overlay_handle {
        self.overlay_window = Some(handle.into());
    }
}

/// Close overlay window (called from observer, not render).
fn close_overlay_window_deferred(&mut self, cx: &mut gpui::Context<Self>) {
    let Some(child_handle) = self.overlay_window.take() else {
        return;
    };
    let _ = cx.update_window(child_handle, |_, child_window, _| {
        child_window.remove_window();
    });
}
```

**Step 3: Remove window open/close from `render()`**

Remove lines 484-492 from `render()`:
```rust
// DELETE these lines:
// Manage overlay child window: open/close on visibility transitions
if sidebar_overlay_visible && !self.sidebar_overlay_was_visible {
    self.open_overlay_window(window, cx);
} else if !sidebar_overlay_visible && self.sidebar_overlay_was_visible {
    self.close_overlay_window(window, cx);
} else if sidebar_overlay_visible && self.overlay_window.is_some() {
    self.sync_overlay_window_bounds(window, cx);
}
self.sidebar_overlay_was_visible = sidebar_overlay_visible;
```

Keep the `sync_overlay_window_bounds` call in render (just repositioning, not creating/destroying windows):
```rust
// Sync overlay bounds on every render (size may have changed)
if sidebar_overlay_visible {
    self.sync_overlay_window_bounds(window, cx);
}
```

Also move `sidebar_overlay_was_visible` tracking to the observer only (remove from render). Ensure the initial value is set in `new()` (already `false`).

**Step 4: Handle parent/child window attachment**

The deferred open doesn't have access to `&mut Window` for the parent. Move the `add_child_window` / `configure_overlay_panel` calls into `sync_overlay_window_bounds` on first sync, or use a flag:

Add field to `AppView`:
```rust
overlay_needs_attach: bool,  // true after open, cleared after first sync
```

In `open_overlay_window_deferred`: set `self.overlay_needs_attach = true;`

In `sync_overlay_window_bounds` (which runs during render with `&Window`):
```rust
if self.overlay_needs_attach {
    self.overlay_needs_attach = false;
    if let Ok(parent_wh) = window.window_handle() {
        let parent_raw = parent_wh.as_raw();
        let _ = cx.update_window(child_handle, |_, child_window, _| {
            if let Ok(child_wh) = child_window.window_handle() {
                conescope_platform::add_child_window(parent_raw, child_wh.as_raw());
                conescope_platform::configure_overlay_panel(child_wh.as_raw());
            }
        });
    }
}
```

**Step 5: Clean up old methods**

Remove `open_overlay_window` and `close_overlay_window` (the ones that take `&mut Window`).

**Step 6: Run `just verify`**

Expected: compiles cleanly, no SIGBUS on launch.

**Step 7: Commit**

```bash
git add crates/conescope-ui/src/views/app_view.rs
git commit -m "fix: move overlay window lifecycle out of render() to prevent SIGBUS"
```

---

### Task 2: Namespace element IDs across windows

**Files:**
- Modify: `crates/conescope-ui/src/views/sidebar.rs`

**Step 1: Pass ID prefix into `render_inner`**

Change `render_with_width` signature to pass a prefix:

```rust
pub fn render_with_width(
    &mut self,
    width: f32,
    glass: bool,
    cx: &mut gpui::Context<Self>,
) -> impl IntoElement {
    let prefix = if glass { "overlay" } else { "pinned" };
    self.render_inner(width, glass, prefix, cx)
}
```

Update `render_inner` signature:
```rust
fn render_inner(
    &mut self,
    width: f32,
    glass: bool,
    prefix: &str,
    cx: &mut gpui::Context<Self>,
) -> impl IntoElement {
```

**Step 2: Namespace the conflicting IDs**

In `render_inner`, change:
- `"sidebar-scroll"` → `format!("{prefix}-sidebar-scroll")`
- In `render_sidebar_header`: `"sidebar-pin-toggle"` → pass prefix through and use `format!("{prefix}-sidebar-pin-toggle")`

Update `render_sidebar_header` to accept `prefix: &str` parameter and use it for the ID.

**Step 3: Update `Render for Sidebar`**

```rust
impl Render for Sidebar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        self.render_inner(SIDEBAR_WIDTH, false, "pinned", cx)
    }
}
```

**Step 4: Run `just verify`**

Expected: compiles, no ID collision warnings.

**Step 5: Commit**

```bash
git add crates/conescope-ui/src/views/sidebar.rs
git commit -m "fix: namespace sidebar element IDs to prevent cross-window collisions"
```

---

### Task 3: Smoke test

**Step 1: Run app and verify**

```bash
just run
```

1. App launches without crash
2. Hover left edge → overlay sidebar appears (frosted glass popup)
3. Click pin toggle → sidebar pins
4. Unpin → sidebar hides
5. Hover again → overlay reappears
6. No SIGBUS in Console.app crash logs

**Step 2: Commit docs**

```bash
git add docs/plans/2026-03-05-fix-overlay-sidebar-crash.md
git commit -m "docs: add crash fix plan"
```
