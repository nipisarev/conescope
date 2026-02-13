# Sidebar & New Instance Redesign — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the bottom activity bar + modal-based instance creation with a left sidebar that lists open instances, supports pinned/overlay modes, and includes a redesigned "New Instance" flow.

**Architecture:** Add new `SidebarMode` (Pinned/Overlay) to `SessionState`. Create a new `Sidebar` view that replaces the bottom `ActivityBar` for instance navigation. The sidebar renders a scrollable instance list with a pinned bottom section (divider + "Create new window" button). Top bar gets a sidebar toggle button after traffic lights and shows the focused instance title (case-sensitive) in focus mode. New instance modal redesigned with recent projects dropdown, browse button, and terminal button separated by dividers.

**Tech Stack:** Rust, GPUI framework, rusqlite (session persistence)

---

## Current State Summary

- **ActivityBar** (`activity_bar.rs`): Bottom bar with `⌘N` instance buttons + panel toggles. Will be **replaced** by the new Sidebar for instance navigation; panel toggles move to the new sidebar or stay in a slimmer bottom bar.
- **TopBar** (`top_bar.rs`): Center title "CONESCOPE" in overview, "⌘N TITLE" in focus. Has "+ New Window" pill button in overview right section.
- **NewInstanceModal** (`new_instance_modal.rs`): Centered modal with Terminal/ProjectHome/Browse buttons + recent projects list.
- **AppState** (`app_state.rs`): Root state, manages `new_instance_modal_open`, `editing_tile_id/input`, etc.
- **SettingsStore** (`settings_store.rs`): `SessionState` (view_mode, focused_instance_id, window bounds). `InstanceLayoutState` per instance.
- **Icons** (`icons.rs`): Phosphor icons, regular weight. Missing: `folder-plus.svg`.

---

### Task 1: Add `folder-plus.svg` icon and `ICON_FOLDER_PLUS` constant

**Files:**
- Create: `crates/conescope-ui/assets/icons/folder-plus.svg`
- Modify: `crates/conescope-ui/src/icons.rs`

**Steps:**
1. Download Phosphor `folder-plus` regular-weight SVG from https://phosphoricons.com (or create manually — it's a folder with a + sign).
2. Save as `crates/conescope-ui/assets/icons/folder-plus.svg`.
3. Add constant to `icons.rs`:
```rust
pub const ICON_FOLDER_PLUS: &str = "icons/folder-plus.svg";
```
4. Also add a `list.svg` / `sidebar-simple.svg` icon constant for the sidebar toggle (already exists as `ICON_SIDEBAR`).
5. Run `just check` to verify.
6. Commit: `feat: add folder-plus icon`

---

### Task 2: Add `SidebarMode` to state and persist in `SessionState`

**Files:**
- Modify: `crates/conescope-ui/src/state/settings_store.rs`
- Modify: `crates/conescope-ui/src/state/app_state.rs`

**Steps:**

1. Add `SidebarMode` enum to `settings_store.rs` after `SidebarTab`:
```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SidebarMode {
    #[default]
    Pinned,
    #[serde(other)]
    Overlay,
}
```

2. Add fields to `SessionState`:
```rust
pub struct SessionState {
    // ... existing fields ...
    #[serde(default)]
    pub sidebar_mode: SidebarMode,
    #[serde(default = "default_true")]
    pub sidebar_open: bool,
}
```
Update `Default` impl accordingly (`sidebar_mode: SidebarMode::Pinned`, `sidebar_open: true`).

3. Add to `AppState` convenience methods:
```rust
pub fn sidebar_mode(&self, cx: &gpui::App) -> SidebarMode {
    self.settings_store.read(cx).session().sidebar_mode
}

pub fn sidebar_open(&self, cx: &gpui::App) -> bool {
    self.settings_store.read(cx).session().sidebar_open
}

pub fn toggle_sidebar_open(&mut self, cx: &mut gpui::Context<Self>) {
    let mut session = self.settings_store.read(cx).session().clone();
    session.sidebar_open = !session.sidebar_open;
    self.settings_store.update(cx, |store, _| store.save_session(session));
    cx.notify();
}

pub fn toggle_sidebar_mode(&mut self, cx: &mut gpui::Context<Self>) {
    let mut session = self.settings_store.read(cx).session().clone();
    session.sidebar_mode = match session.sidebar_mode {
        SidebarMode::Pinned => SidebarMode::Overlay,
        SidebarMode::Overlay => SidebarMode::Pinned,
    };
    self.settings_store.update(cx, |store, _| store.save_session(session));
    cx.notify();
}
```

4. Add a new action `ToggleOverviewSidebar` to `actions.rs`.
5. Run `just check`.
6. Commit: `feat: add SidebarMode state (pinned/overlay)`

---

### Task 3: Remove "+ New Window" button from top bar overview mode

**Files:**
- Modify: `crates/conescope-ui/src/views/top_bar.rs` (function `render_overview_buttons`, lines 111-198)

**Steps:**

1. In `render_overview_buttons`, remove the entire `"+ New Window"` pill button block (lines 131-158). Keep only the questions and settings buttons.
2. Run `just check`.
3. Commit: `refactor: remove new-window button from top bar`

---

### Task 4: Remove "CONESCOPE" title from top bar in overview mode

**Files:**
- Modify: `crates/conescope-ui/src/views/top_bar.rs` (Render impl, lines 55-65)

**Steps:**

1. Change the center title for `ViewMode::Overview` to empty string:
```rust
ViewMode::Overview => (String::new(), theme.text_muted),
```
2. Run `just check`.
3. Commit: `refactor: remove CONESCOPE title from overview top bar`

---

### Task 5: Add sidebar toggle button to top bar (after traffic lights)

**Files:**
- Modify: `crates/conescope-ui/src/views/top_bar.rs`

**Steps:**

1. In the `Render` impl, replace the static 76px left padding with a flex row containing:
   - 76px spacer (traffic light area)
   - Sidebar toggle button (uses `ICON_SIDEBAR` icon)

```rust
// Replace: .child(div().w(px(76.)))
// With:
.child(
    div()
        .flex()
        .flex_row()
        .items_center()
        .child(div().w(px(76.))) // traffic light spacer
        .child(render_sidebar_toggle(&self.app_state, font_size, &theme, sidebar_open))
)
```

2. Add `render_sidebar_toggle` function:
```rust
fn render_sidebar_toggle(
    app_state: &Entity<AppState>,
    font_size: f32,
    theme: &Theme,
    active: bool,
) -> gpui::Div {
    let app_state = app_state.clone();
    let fg: Hsla = if active { theme.accent.into() } else { theme.text_muted.into() };
    let hover_fg: Hsla = theme.text.into();
    let icon_size = px(font_size + 1.0);

    div()
        .px(px(4.))
        .py(px(2.))
        .rounded(px(3.))
        .cursor_pointer()
        .text_color(fg)
        .hover(move |s| s.text_color(hover_fg))
        .child(
            svg()
                .path(icons::ICON_SIDEBAR)
                .size(icon_size)
                .text_color(fg)
                .flex_shrink_0(),
        )
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, AppState::toggle_sidebar_open);
        })
}
```

3. Read `sidebar_open` from state in render and pass it to the function.
4. Run `just check`.
5. Commit: `feat: add sidebar toggle button to top bar`

---

### Task 6: Move focused instance title to top bar left (focus mode, case-sensitive)

**Files:**
- Modify: `crates/conescope-ui/src/views/top_bar.rs`

**Steps:**

1. Change `focused_info` to return the title as-is (not `.to_uppercase()`):
```rust
let title = inst
    .instance
    .title
    .as_deref()
    .unwrap_or("Untitled")
    .to_owned(); // Remove .to_uppercase()
```

2. In the Render impl, for `ViewMode::Focus`, instead of showing the title centered, show it in the left section (after the sidebar toggle button):
```rust
// Left section: traffic lights spacer + sidebar toggle + focused title
.child(
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .child(div().w(px(76.)))
        .child(render_sidebar_toggle(&self.app_state, font_size, &theme, sidebar_open))
        .when(view_mode == ViewMode::Focus, |el| {
            if let Some((num, title, color)) = focused_info_data {
                el.child(
                    div()
                        .text_color(color)
                        .text_size(px(font_size))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(format!("\u{2318}{num} {title}"))
                )
            } else { el }
        })
)
```

3. Make the center title empty for Focus mode (sidebar toggle + title are now on the left).
4. Run `just check`.
5. Commit: `feat: show case-sensitive instance title in top bar left`

---

### Task 7: Create the `Sidebar` view with instance list

**Files:**
- Create: `crates/conescope-ui/src/views/sidebar.rs`
- Modify: `crates/conescope-ui/src/views/mod.rs` (add `pub mod sidebar;`)

**Steps:**

1. Create `sidebar.rs` with a `Sidebar` struct:
```rust
use gpui::prelude::*;
use gpui::{Entity, MouseButton, ScrollHandle, SharedString, div, px, svg};

use conescope_core::instance::InstanceType;

use crate::icons;
use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;
use crate::theme::Theme;
use crate::views::colors::{default_instance_color, hex_to_rgba, status_color};

const SIDEBAR_WIDTH: f32 = 220.0;

pub struct Sidebar {
    app_state: Entity<AppState>,
    scroll_handle: ScrollHandle,
}

impl Sidebar {
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            scroll_handle: ScrollHandle::new(),
        }
    }
}
```

2. Implement instance row rendering — each row shows:
   - Line 1: `⌘{num}` + color dot + title (instance name, case-sensitive)
   - Line 2: shortened path in muted text
   - Highlight background if focused

```rust
struct SidebarEntry {
    id: String,
    num: usize,
    title: String,
    path: String,
    color: gpui::Rgba,
    is_focused: bool,
    instance_type: InstanceType,
}

fn render_instance_row(
    entry: &SidebarEntry,
    app_state: &Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let id = entry.id.clone();
    let app_state = app_state.clone();
    let accent = theme.accent;
    let element_hover = theme.element_hover;
    let icon_size = px(font_size - 2.0);

    div()
        .px(px(8.))
        .py(px(4.))
        .rounded(px(4.))
        .cursor_pointer()
        .when(entry.is_focused, move |el| el.bg(accent))
        .when(!entry.is_focused, move |el| el.hover(move |s| s.bg(element_hover)))
        .flex()
        .flex_col()
        .gap(px(1.))
        // Line 1: ⌘num + title
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(4.))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(1.))
                        .text_color(entry.color)
                        .text_size(px(font_size - 1.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(svg().path(icons::ICON_COMMAND).size(icon_size).text_color(entry.color).flex_shrink_0())
                        .child(format!("{}", entry.num)),
                )
                .child(
                    div()
                        .text_size(px(font_size - 1.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(entry.color)
                        .overflow_x_hidden()
                        .child(entry.title.clone()),
                ),
        )
        // Line 2: path
        .child(
            div()
                .pl(px(icon_size.0 + 5.0)) // align under title
                .text_size(px(font_size - 3.0))
                .text_color(theme.text_muted)
                .overflow_x_hidden()
                .child(entry.path.clone()),
        )
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, |s, cx| s.focus_instance(&id, cx));
        })
}
```

3. Implement `Render` for `Sidebar`:
```rust
impl Render for Sidebar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let state = self.app_state.read(cx);
        let theme = state.theme().clone();
        let font_size = state.settings_store.read(cx).settings().ui_font_size();

        let entries = collect_sidebar_entries(&self.app_state, cx);
        let app_state_new = self.app_state.clone();

        // Scrollable instance list
        let mut list = div().flex().flex_col().gap(px(2.)).py(px(4.)).px(px(4.));
        for entry in &entries {
            list = list.child(render_instance_row(entry, &self.app_state, font_size, &theme));
        }

        let scroll_area = div()
            .id("sidebar-instance-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .child(list);

        // Pinned bottom: divider + "Create a new window" button
        let bottom_section = div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .child(div().h(px(1.)).w_full().bg(theme.border)) // divider
            .child(render_new_window_button(&app_state_new, font_size, &theme));

        div()
            .h_full()
            .w(px(SIDEBAR_WIDTH))
            .flex()
            .flex_col()
            .bg(theme.panel)
            .border_r_1()
            .border_color(theme.border)
            .child(scroll_area)
            .child(bottom_section)
    }
}
```

4. Add `collect_sidebar_entries` helper (similar to `collect_instance_data` in `activity_bar.rs`).
5. Run `just check`.
6. Commit: `feat: create Sidebar view with instance list`

---

### Task 8: Add "Create a new window" button to sidebar bottom

**Files:**
- Modify: `crates/conescope-ui/src/views/sidebar.rs`

**Steps:**

1. Implement `render_new_window_button`:
```rust
fn render_new_window_button(
    app_state: &Entity<AppState>,
    font_size: f32,
    theme: &Theme,
) -> gpui::Div {
    let app_state = app_state.clone();
    let element_hover = theme.element_hover;
    let icon_size = px(font_size);

    div()
        .mx(px(4.))
        .my(px(4.))
        .px(px(8.))
        .py(px(6.))
        .rounded(px(4.))
        .cursor_pointer()
        .hover(move |s| s.bg(element_hover))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(6.))
        .child(
            svg()
                .path(icons::ICON_FOLDER_PLUS)
                .size(icon_size)
                .text_color(theme.text_muted)
                .flex_shrink_0(),
        )
        .child(
            div()
                .flex_1()
                .text_size(px(font_size - 1.0))
                .text_color(theme.text_muted)
                .child("Create a new window"),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(1.))
                .text_size(px(font_size - 2.0))
                .text_color(theme.text_faint)
                .child(svg().path(icons::ICON_COMMAND).size(px(font_size - 3.0)).text_color(theme.text_faint).flex_shrink_0())
                .child("N"),
        )
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state.update(cx, AppState::toggle_new_instance_modal);
        })
}
```

2. Run `just check`.
3. Commit: `feat: add create-window button to sidebar bottom`

---

### Task 9: Integrate Sidebar into `AppView` — pinned mode

**Files:**
- Modify: `crates/conescope-ui/src/views/app_view.rs`

**Steps:**

1. Add `sidebar: Entity<Sidebar>` field to `AppView`. Initialize in constructor.
2. In `render()`, wrap the main content in a horizontal flex layout:
```rust
// Main area: sidebar (optional) + content
.child(
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_row()
        // Pinned sidebar
        .when(sidebar_open && sidebar_mode == SidebarMode::Pinned, |el| {
            el.child(self.sidebar.clone())
        })
        // Content
        .child(match view_mode {
            ViewMode::Overview => div().key_context("Overview").flex_1().min_h_0()
                .child(self.overview_grid.clone()).into_any_element(),
            ViewMode::Focus => div().flex_1().min_h_0()
                .child(self.focus_view.clone()).into_any_element(),
            ViewMode::Settings => div().flex_1().min_h_0()
                .child(self.settings_view.clone()).into_any_element(),
        })
)
```

3. Read `sidebar_open` and `sidebar_mode` from `app_state`.
4. Remove `self.activity_bar` or keep it as a minimal status bar (tokens/cost only).
5. Run `just check` and `just run` to visually verify.
6. Commit: `feat: integrate pinned sidebar into app layout`

---

### Task 10: Implement sidebar overlay mode (slide-in on mouse hover)

**Files:**
- Modify: `crates/conescope-ui/src/views/app_view.rs`
- Modify: `crates/conescope-ui/src/state/app_state.rs`

**Steps:**

1. Add `sidebar_overlay_visible: bool` to `AppState` (transient, not persisted).
2. In `AppView::render()`, when `sidebar_mode == Overlay`:
   - Add a narrow hover-detection strip on the left edge:
   ```rust
   .when(sidebar_mode == SidebarMode::Overlay && !sidebar_overlay_visible, |el| {
       el.child(
           div()
               .absolute()
               .top_0()
               .left_0()
               .w(px(8.))
               .h_full()
               .on_mouse_move({
                   let app_state = self.app_state.clone();
                   move |event, _, cx| {
                       if f32::from(event.position.x) < 8.0 {
                           app_state.update(cx, |s, cx| {
                               s.sidebar_overlay_visible = true;
                               cx.notify();
                           });
                       }
                   }
               })
       )
   })
   ```
   - When overlay visible, render sidebar as absolute overlay:
   ```rust
   .when(sidebar_mode == SidebarMode::Overlay && sidebar_overlay_visible, |el| {
       el.child(
           div()
               .absolute()
               .top_0()
               .left_0()
               .h_full()
               .child(self.sidebar.clone())
       )
   })
   ```
   - Add a transparent backdrop to dismiss overlay on mouse leave/click outside.

3. In `AppState`, add method:
```rust
pub fn hide_sidebar_overlay(&mut self, cx: &mut gpui::Context<Self>) {
    self.sidebar_overlay_visible = false;
    cx.notify();
}
```

4. Run `just check` and `just run`.
5. Commit: `feat: implement overlay sidebar mode with mouse hover trigger`

---

### Task 11: Click-to-rename in sidebar rows (real-time top bar update)

**Files:**
- Modify: `crates/conescope-ui/src/views/sidebar.rs`
- Modify: `crates/conescope-ui/src/state/app_state.rs`

**Steps:**

1. In `render_instance_row`, add double-click handler to start inline editing:
```rust
.on_mouse_down(MouseButton::Left, move |event, window, cx| {
    // Single click: focus instance
    app_state.update(cx, |s, cx| s.focus_instance(&id, cx));
})
```

2. Add click handler on the **title text** specifically (not the whole row) to trigger rename:
   - When `editing_tile_id == Some(entry.id)`, render a `TextInput` instead of static title
   - Reuse existing `AppState::start_edit_title` / `save_edit_title` / `cancel_edit_title` — they already update the instance title in memory + DB and call `cx.notify()`, which propagates to top bar

3. The top bar already reads `instance.title` on each render, so renaming in sidebar auto-updates the top bar in real time (no extra wiring needed).

4. Run `just check`.
5. Commit: `feat: click-to-rename instance title in sidebar`

---

### Task 12: Redesign New Instance Modal

**Files:**
- Modify: `crates/conescope-ui/src/views/new_instance_modal.rs`

**Steps:**

1. Redesign `render_modal_body` layout top-to-bottom:

**Section 1: Recent Projects dropdown**
```rust
// "RECENT PROJECTS" header + scrollable project list
// List opens over other elements (absolute positioning)
// Each row: project name + path, click to open
```

**Section 2: Browse + Select Directory**
```rust
// "BROWSE" label
// "Select Directory..." button — on click: prompt_for_paths, then immediately create project instance
// (reuse existing browse_for_directory logic)
```

**Section 3: Divider + Open Terminal**
```rust
// Horizontal divider
// "Open Terminal" button with terminal icon
// On click: create_terminal
```

2. Remove the old "New Terminal" / "New Project (~/) " / "Browse..." triple-button layout.
3. Move Recent Projects section to the **top** of the modal (currently at bottom).
4. Make Recent Projects a dropdown-style list with internal scroll (max-height with overflow_y_scroll).
5. The dropdown should open over other elements — use absolute positioning.
6. "Select Directory..." button should directly open the file picker and create instance (already does this via `browse_for_directory`).
7. Run `just check` and `just run`.
8. Commit: `feat: redesign new instance modal`

---

### Task 13: Clean up ActivityBar — convert to minimal status bar

**Files:**
- Modify: `crates/conescope-ui/src/views/activity_bar.rs`

**Steps:**

1. Remove instance buttons from the activity bar (they're now in the sidebar).
2. Keep only:
   - Panel toggles (sidebar/editor/terminal) in focus mode
   - Token/cost stats on the right
3. Or alternatively, remove ActivityBar entirely and move panel toggles to the top bar right section in focus mode.
4. Run `just check`.
5. Commit: `refactor: simplify activity bar after sidebar migration`

---

### Task 14: Wire up keyboard shortcuts

**Files:**
- Modify: `crates/conescope/src/main.rs` (keybinding registration)

**Steps:**

1. Ensure `ToggleOverviewSidebar` action is bound (e.g., `Cmd+\` or reuse `Cmd+B` context-aware).
2. Ensure `Cmd+N` still triggers `NewInstance` action (opens the modal).
3. Run `just check`.
4. Commit: `feat: wire sidebar keyboard shortcuts`

---

### Task 15: Final integration testing and polish

**Steps:**

1. Run `just verify` (fmt-check + clippy + test).
2. Manual testing:
   - Overview mode: sidebar visible, no "CONESCOPE" title, no "+ New Window" button
   - Click sidebar toggle → sidebar hides/shows
   - Pinned mode: tiles re-layout when sidebar shown/hidden
   - Overlay mode: sidebar slides in on mouse hover near left edge, overlays content
   - Instance rows show ⌘num + title + path (second line)
   - Click instance row → focuses that instance
   - Click title in sidebar → inline rename, top bar updates in real time
   - "Create a new window" button at sidebar bottom → opens modal
   - New Instance modal: recent projects at top, browse in middle, terminal at bottom
   - Scrolling instance list: divider + button pinned at bottom
3. Fix any visual issues.
4. Commit: `polish: sidebar redesign final adjustments`
