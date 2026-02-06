# Phase 5: Real UI — Overview + Focus Views

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the single-terminal placeholder window with the actual Conescope UI: Overview mode (grid of instance tiles with mini terminals) and Focus mode (single instance with full terminal), driven by AppState.

**Architecture:** Single root `AppView` entity holds `Entity<AppState>` and conditionally renders Overview or Focus based on `ViewMode`. Raw GPUI primitives (`div`, `flex`) for layout — no gpui-component dependency yet (avoids git/crates.io version conflicts). Instance tiles show the same `Entity<TerminalView>` that InstanceEntry holds, re-parented in the render tree when switching modes.

**Tech Stack:** Existing workspace deps only. `gpui::div()` layout, `on_mouse_down` for interaction, `rgba()` colors.

**Deferred to Phase 6:** Editor pane, file tree, terminal tabs, questions panel, resizable panels, keyboard shortcuts, settings modal, title editing.

**GPUI API notes:**
- `Render::render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement`
- Entity children: store as fields, render via `.child(self.field.clone())`
- `on_mouse_down` callback: `Fn(&MouseDownEvent, &mut Window, &mut App)`
- `on_action` callback: `Fn(&mut Self, &Action, &mut Window, &mut Context<Self>)` — has window!
- `cx.observe_window_bounds(window, |this, window, cx|)` — resize handling
- `cx.subscribe(&entity, |this, entity, event, cx|)` — entity-to-entity subscription
- `cx.spawn(async move |cx: AsyncApp| { cx.background_executor().timer().await; cx.update(|cx| ...) })` — app-level async task
- `PATH="/opt/homebrew/opt/zig@0.14/bin:$PATH"` required for all cargo commands

---

## Checks Configuration

```bash
export PATH="/opt/homebrew/opt/zig@0.14/bin:$PATH"

# After each task:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Task 1: Refactor `start_output_polling` — Remove Window Dependency

**Goal:** Change output polling from `window.spawn()` to `cx.spawn()` so instance creation doesn't require `Window` access. This unblocks clean action-based instance creation in Task 8.

### Step 1: Update `start_output_polling` signature

**File:** `crates/conescope-ui/src/state/instance_entry.rs`

Change signature from:
```rust
pub fn start_output_polling(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>)
```
To:
```rust
pub fn start_output_polling(&mut self, cx: &mut gpui::Context<Self>)
```

Change body: replace `window.spawn(cx, async move |cx| { ... })` with `cx.spawn(async move |cx| { ... })`.

The async closure receives `AsyncApp` instead of `AsyncWindowContext`. Update the inner `cx.update()` call:
- Was: `cx.update(|_, cx| { ... })` (ignored window param)
- Now: `cx.update(|cx| { ... })` (just App)

### Step 2: Update callers

**File:** `crates/conescope-ui/src/state/instance_list.rs`

In `create_instance`: change `e.start_output_polling(window, cx)` to `e.start_output_polling(cx)`.

In `restore_terminals`: change `e.start_output_polling(window, cx)` to `e.start_output_polling(cx)`.

### Step 3: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs  # Terminal still works (output appears)
```

Commit: `refactor: remove Window dependency from start_output_polling (Phase 5.1)`

---

## Task 2: Views Module + AppView Root Component

**Goal:** Create the root view entity that switches between Overview and Focus modes.

### Step 1: Create views directory and module

**File:** `crates/conescope-ui/src/views/mod.rs`
```rust
pub mod app_view;
```

**File:** `crates/conescope-ui/src/lib.rs` — add `pub mod views;`

### Step 2: Create AppView

**File:** `crates/conescope-ui/src/views/app_view.rs`

```rust
use gpui::prelude::*;
use gpui::{Entity, div, rgba};

use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;

pub struct AppView {
    pub app_state: Entity<AppState>,
}

impl std::fmt::Debug for AppView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppView").finish_non_exhaustive()
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let view_mode = self.app_state.read(cx).view_mode(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgba(0x1e1e_1eff))
            .child(
                div().flex_1().child(match view_mode {
                    ViewMode::Overview => "Overview mode",
                    ViewMode::Focus => "Focus mode",
                }),
            )
    }
}
```

### Step 3: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Commit: `feat: add AppView root component with view mode switch (Phase 5.2)`

---

## Task 3: Wire AppView into main.rs

**Goal:** Replace the standalone terminal pane in main.rs with AppView. The window should show the AppView which reads AppState.

### Step 1: Rewrite window callback in main.rs

**File:** `crates/conescope/src/main.rs`

Replace the entire `cx.open_window(...)` block. The new callback:

```rust
cx.open_window(
    WindowOptions { /* same options as before */ },
    |_window, cx| {
        let app_view = cx.new(|_| {
            conescope_ui::views::app_view::AppView {
                app_state: app_state.clone(),
            }
        });
        app_view
    },
)
```

Remove:
- `spawn_terminal_pane` call
- PTY resize observer (`observe_window_bounds`)
- Output batching async task
- All `use` imports related to `TerminalView`, `PtySize`, `Duration`, `compute_cell_metrics`, `spawn_terminal_pane`

Keep:
- `db_path()` function
- tracing initialization
- `DbHandle::spawn()`
- `AppState::new(db, cx)`
- Async DB loading block (`cx.spawn(async move |cx| { ... })`)

### Step 2: Clean up imports

Remove unused imports from main.rs. Add `use conescope_ui::views::app_view::AppView;`.

### Step 3: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs  # Window shows "Overview mode" text on dark background
```

Commit: `feat: wire AppView into main.rs, replace standalone terminal (Phase 5.3)`

---

## Task 4: TopBar Component

**Goal:** Horizontal bar at top with app title and action buttons.

### Step 1: Create TopBar

**File:** `crates/conescope-ui/src/views/top_bar.rs`

```rust
use gpui::prelude::*;
use gpui::{Entity, div, px, rgba, MouseButton};

use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;

#[derive(Debug)]
pub struct TopBar {
    app_state: Entity<AppState>,
}

impl TopBar {
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self { app_state }
    }
}
```

Render method:
- Container: `h(px(36.))`, `w_full()`, `flex()`, `flex_row()`, `items_center()`
- Background: `bg(rgba(0x2525_26ff))`, border-bottom: `border_b_1()`, `border_color(rgba(0x3c3c_3cff))`
- Left padding: `pl(px(76.))` — space for macOS traffic lights (since `appears_transparent` is true)
- Center: `div().flex_1().text_color(rgba(0x8888_88ff))`
  - Overview: "CONESCOPE"
  - Focus: read focused instance title + number from AppState
- Right section: `div().flex().flex_row().gap(px(8.)).pr(px(12.))`
  - [+] button: `div().px(px(8.)).py(px(4.)).rounded(px(4.)).cursor_pointer().bg(rgba(0x3c3c_3cff)).child("+").on_mouse_down(MouseButton::Left, ...)`
    - Click: `app_state.update(cx, |s, cx| s.toggle_new_instance_modal(cx))`
  - In Focus mode only: [←] back button
    - Click: `app_state.update(cx, |s, cx| s.return_to_overview(cx))`

Note: `on_mouse_down` callback gets `(&MouseDownEvent, &mut Window, &mut App)`. Use the `App` to update entities: `app_state.update(cx, |s, cx| ...)`.

### Step 2: Wire TopBar into AppView

**File:** `crates/conescope-ui/src/views/app_view.rs`

Add `top_bar: Entity<TopBar>` field to AppView. Initialize in constructor or in `render()` (store as field to avoid recreating each frame).

Since AppView is created via `cx.new(|_| AppView { ... })` (which gives `Context<AppView>`), create TopBar there:

```rust
// In main.rs window callback:
let app_view = cx.new(|cx| {
    let top_bar = cx.new(|_| TopBar::new(app_state.clone()));
    AppView {
        app_state: app_state.clone(),
        top_bar,
    }
});
```

In AppView render:
```rust
div()
    .size_full()
    .flex()
    .flex_col()
    .bg(rgba(0x1e1e_1eff))
    .child(self.top_bar.clone())
    .child(/* mode content */)
```

### Step 3: Update views/mod.rs

Add `pub mod top_bar;`

### Step 4: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs  # Dark top bar with "CONESCOPE" + [+] button
```

Commit: `feat: add TopBar component (Phase 5.4)`

---

## Task 5: ActivityBar (Bottom Bar)

**Goal:** Fixed bar at bottom with instance navigation buttons and token/cost totals.

### Step 1: Create ActivityBar

**File:** `crates/conescope-ui/src/views/activity_bar.rs`

```rust
#[derive(Debug)]
pub struct ActivityBar {
    app_state: Entity<AppState>,
}
```

Render:
- Container: `h(px(28.))`, `w_full()`, `flex()`, `flex_row()`, `items_center()`, `px(px(8.))`
- Background: `bg(rgba(0x1818_18ff))`, `border_t_1()`, `border_color(rgba(0x3c3c_3cff))`
- Left section: `div().flex().flex_row().gap(px(4.)).flex_1()`
  - Grid icon: `div().child("▦").cursor_pointer()` — click → `return_to_overview()`
  - Instance number buttons: for each entry in `instance_list.entries()`:
    - Read instance number, color, id
    - Render colored numbered button (`div().px(px(6.)).rounded(px(3.)).bg(color).child(format!("{n}"))`)
    - Click → `focus_instance(id)`
    - If currently focused: brighter background
- Right section: `div().flex().flex_row().gap(px(12.)).text_color(rgba(0x8888_88ff))`
  - Total tokens: sum `entry.read(cx).instance.tokens_used` across all entries, format as `"{n/1000}k tokens"`
  - Total cost: sum `entry.read(cx).instance.cost_estimate`, format as `"${cost:.2}"`

### Step 2: Wire into AppView

Add `activity_bar: Entity<ActivityBar>` field. Create in constructor. Render as last child.

### Step 3: Update views/mod.rs

Add `pub mod activity_bar;`

### Step 4: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs  # TopBar + content + bottom bar with "0k tokens $0.00"
```

Commit: `feat: add ActivityBar bottom bar (Phase 5.5)`

---

## Task 6: OverviewGrid with Instance Tiles

**Goal:** Dynamic grid of instance tiles, each showing mini terminal preview + metadata header. Plus an empty slot with [+] button.

### Step 1: Create OverviewGrid

**File:** `crates/conescope-ui/src/views/overview_grid.rs`

```rust
#[derive(Debug)]
pub struct OverviewGrid {
    app_state: Entity<AppState>,
}
```

Grid layout logic (compute columns and rows from instance count):
```rust
fn grid_dimensions(count: usize) -> (usize, usize) {
    // count includes the empty slot
    let total = count + 1;
    match total {
        1 => (1, 1),
        2 => (2, 1),
        3..=4 => (2, 2),
        5..=6 => (3, 2),
        _ => (3, (total + 2) / 3),
    }
}
```

Render: build rows of tiles using nested `div().flex_row()`.

For each instance entry: render a tile div:
- Container: `flex_1()`, `flex()`, `flex_col()`, `border_r_1()`, `border_b_1()`, `border_color(rgba(0x3c3c_3cff))`
- Header (24px): instance number (colored), title, status dot
  - Status dot colors: Working=`0x81C7_84ff`, Waiting=`0xFFB7_4Dff`, Paused=`0x90A4_AEff`, Starting=`0x64B5_F6ff`, Stopped=`0x6666_66ff`
  - Parse instance color: `Hsla::from(hex_to_rgba(color))` or use `rgba()` directly
- Body: `flex_1()` containing the `Entity<TerminalView>` from `entry.read(cx).terminal_view.clone()`
  - If None: show placeholder text "No terminal"
- Click on entire tile: `on_mouse_down` → `app_state.update(cx, |s, cx| s.focus_instance(id, cx))`

Empty slot (last position):
- Same size as tiles (`flex_1()`)
- Centered "+" text, `bg(0x1e1e_1eff)`, cursor_pointer
- Click → `toggle_new_instance_modal()`

**Color parsing helper:** Add `fn hex_color(hex: &str) -> gpui::Hsla` to convert CSS hex colors (`#E57373`) to GPUI Hsla. Put in a shared `colors.rs` helper.

### Step 2: Create color helpers

**File:** `crates/conescope-ui/src/views/colors.rs`

```rust
use gpui::Rgba;

/// Parse a CSS hex color string (#RRGGBB or #RGB) to GPUI Rgba.
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
    Rgba { r: f32::from(r) / 255.0, g: f32::from(g) / 255.0, b: f32::from(b) / 255.0, a: 1.0 }
}

/// Status color for an instance status.
pub fn status_color(status: conescope_core::instance::InstanceStatus) -> Rgba {
    use conescope_core::instance::InstanceStatus;
    match status {
        InstanceStatus::Working => hex_to_rgba("#81C784"),
        InstanceStatus::Waiting => hex_to_rgba("#FFB74D"),
        InstanceStatus::Paused => hex_to_rgba("#90A4AE"),
        InstanceStatus::Starting => hex_to_rgba("#64B5F6"),
        InstanceStatus::Stopped => hex_to_rgba("#666666"),
    }
}
```

### Step 3: Wire into AppView

Replace the Overview placeholder in AppView render with `self.overview_grid.clone()`.

Add `overview_grid: Entity<OverviewGrid>` field to AppView. Create in constructor.

### Step 4: Update views/mod.rs

Add `pub mod overview_grid;` and `pub mod colors;`

### Step 5: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs  # Grid with just the empty [+] slot (no instances yet)
```

Commit: `feat: add OverviewGrid with instance tiles and empty slot (Phase 5.6)`

---

## Task 7: FocusView Component

**Goal:** Full-screen terminal view for the focused instance with PTY resize.

### Step 1: Create FocusView

**File:** `crates/conescope-ui/src/views/focus_view.rs`

```rust
use gpui::prelude::*;
use gpui::{Entity, div, px, rgba};

use crate::state::app_state::AppState;
use crate::terminal::compute_cell_metrics;

#[derive(Debug)]
pub struct FocusView {
    app_state: Entity<AppState>,
}
```

Render:
- Read `focused_instance_id` from `app_state.read(cx).focused_instance_id(cx)`
- If None: show "No instance focused" message
- Find instance: `app_state.read(cx).instance_list.read(cx).find_by_id(id, cx)`
- If found with terminal_view:
  - `div().size_full().flex().flex_col().child(terminal_view.clone())`
  - The terminal view fills the entire content area
- If not found: "Instance not found" message

### Step 2: PTY resize on window bounds change

In FocusView, register a window bounds observer. Since we need to register this once (not every render), do it in AppView's initialization.

**In AppView initialization** (where we have access to window via the open_window callback context):

After creating AppView, register the resize observer:
```rust
// In main.rs, after creating app_view:
let app_state_for_resize = app_state.clone();
let resize_sub = app_view.update(cx, |_, cx| {
    cx.observe_window_bounds(window, move |_this, window, cx| {
        let state = app_state_for_resize.read(cx);
        let view_mode = state.view_mode(cx);
        if view_mode != ViewMode::Focus {
            return;
        }
        let Some(focused_id) = state.focused_instance_id(cx) else { return };
        let Some(entry) = state.instance_list.read(cx).find_by_id(focused_id, cx) else { return };

        let size = window.viewport_size();
        let content_height = f32::from(size.height) - 36.0 - 28.0; // minus TopBar + ActivityBar
        let width = f32::from(size.width);

        let Some((cell_width, cell_height)) = compute_cell_metrics(window) else { return };
        let cols = (width / cell_width).floor().max(1.0) as u16;
        let rows = (content_height / cell_height).floor().max(1.0) as u16;

        entry.read(cx).resize_pty(cols, rows);
        if let Some(ref tv) = entry.read(cx).terminal_view {
            tv.update(cx, |view, cx| view.resize_terminal(cols, rows, cx));
        }
    })
});
resize_sub.detach();
```

### Step 3: Wire into AppView

Add `focus_view: Entity<FocusView>` field. Create in constructor. Render when `ViewMode::Focus`.

### Step 4: Update views/mod.rs

Add `pub mod focus_view;`

### Step 5: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Commit: `feat: add FocusView with full terminal and PTY resize (Phase 5.7)`

---

## Task 8: Instance Creation Flow

**Goal:** Wire [+] button → modal → create instance → show in grid. Uses GPUI action dispatch for window access.

### Step 1: Define CreateInstance action

**File:** `crates/conescope-ui/src/views/app_view.rs` (or a shared actions module)

```rust
use conescope_core::instance::InstanceType;

#[derive(Debug, Clone)]
pub struct CreateInstanceAction {
    pub instance_type: InstanceType,
    pub project_path: Option<String>,
}

// GPUI actions need the Action derive (check actual GPUI API)
// If gpui::Action derive exists, use it. Otherwise implement manually.
```

Note: Check how GPUI registers actions. Zed uses `actions!(workspace, [NewTerminal])` macro. We may need `gpui::actions!` or manual `impl Action`.

### Step 2: Add `push_entry` method to InstanceList

**File:** `crates/conescope-ui/src/state/instance_list.rs`

```rust
/// Add a pre-built entry to the list (used by action handlers that build entries externally).
pub fn push_entry(&mut self, entry: Entity<InstanceEntry>, cx: &mut gpui::Context<Self>) {
    let id = entry.read(cx).id().to_owned();

    // Subscribe for DB persistence
    let sub = cx.subscribe(&entry, |this, entity, event, cx| {
        let inst = entity.read(cx);
        match event {
            InstanceEvent::StatusChanged(status) => {
                this.db.update_instance(
                    inst.id().to_owned(),
                    InstanceUpdate { status: Some(*status), ..Default::default() },
                );
            }
            InstanceEvent::Exited => {
                let ended = chrono::Utc::now().to_rfc3339();
                this.db.end_instance(inst.id().to_owned(), ended);
            }
        }
    });
    sub.detach();

    self.entries.push(entry);
    cx.emit(InstanceListEvent::Added(id));
    cx.notify();
}
```

### Step 3: Register action handler on AppView

In AppView initialization (constructor or dedicated init method):

```rust
impl AppView {
    fn init(&mut self, cx: &mut gpui::Context<Self>) {
        cx.on_action(Self::handle_create_instance);
    }

    fn handle_create_instance(
        &mut self,
        action: &CreateInstanceAction,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let cwd = action.project_path.as_deref().unwrap_or(&home);

        // Spawn PTY (needs window + cx)
        let pane = crate::terminal::spawn_terminal_pane(Some(cwd), window, cx);

        // Build Instance
        let id = uuid::Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();
        let instance = conescope_core::instance::Instance {
            id: id.clone(),
            project_id: None, // TODO: wire project for Project type
            title: Some(match action.instance_type {
                InstanceType::Project => "New Project".into(),
                InstanceType::Terminal => "Terminal".into(),
            }),
            status: conescope_core::instance::InstanceStatus::Starting,
            instance_number: None, // TODO: get next number from DB
            tokens_used: 0,
            cost_estimate: 0.0,
            started_at,
            ended_at: None,
            instance_type: action.instance_type,
            color: None,
        };

        // DB insert (fire-and-forget)
        self.app_state.read(cx).db.insert_instance(instance.clone());

        // Create InstanceEntry with terminal
        let is_project = action.instance_type == InstanceType::Project;
        let entry = cx.new(|_| {
            let mut e = crate::state::instance_entry::InstanceEntry::from_instance(instance);
            e.attach_terminal(pane);
            e
        });

        // Start output polling (no longer needs window!)
        entry.update(cx, |e, cx| e.start_output_polling(cx));

        // Send "claude\r" for project instances
        if is_project {
            entry.read(cx).send_input(b"claude\r");
        }

        // Push to instance list
        let instance_list = self.app_state.read(cx).instance_list.clone();
        instance_list.update(cx, |list, cx| list.push_entry(entry, cx));

        // Close modal
        self.app_state.update(cx, |s, cx| {
            s.new_instance_modal_open = false;
            cx.notify();
        });
    }
}
```

### Step 4: Create NewInstanceModal

**File:** `crates/conescope-ui/src/views/new_instance_modal.rs`

Simple modal with two buttons — no text input for v1.

```rust
#[derive(Debug)]
pub struct NewInstanceModal {
    app_state: Entity<AppState>,
}
```

Render:
- Backdrop: `position_absolute()`, full size, `bg(rgba(0x0000_0080))`
- Modal box: centered (via flex + margins), 300×200px, `bg(rgba(0x2d2d_2dff))`, `rounded(px(8.))`, `border_1()`, `border_color(rgba(0x4c4c_4cff))`
- Header: "New Instance" + [×] close button
- Two buttons stacked vertically:
  - "New Terminal" → dispatch `CreateInstanceAction { instance_type: Terminal, project_path: None }`
  - "New Project (~/)" → dispatch `CreateInstanceAction { instance_type: Project, project_path: Some(home_dir) }`
- Each button: `on_mouse_down(MouseButton::Left, move |_, window, cx| { window.dispatch_action(..., cx); })`
- Close: `app_state.update(cx, |s, cx| s.toggle_new_instance_modal(cx))`

### Step 5: Wire modal into AppView

In AppView render, conditionally render the modal overlay when `new_instance_modal_open`:

```rust
.when(self.app_state.read(cx).new_instance_modal_open, |el| {
    el.child(self.new_instance_modal.clone())
})
```

Store `new_instance_modal: Entity<NewInstanceModal>` as AppView field.

### Step 6: Update views/mod.rs

Add `pub mod new_instance_modal;`

### Step 7: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs
# Test: Click [+] → modal appears → click "New Terminal" → tile appears in grid with live terminal
# Test: Click tile → switches to Focus mode → full terminal → type commands
# Test: Click [←] in TopBar → returns to Overview
```

Commit: `feat: add instance creation flow with modal and action dispatch (Phase 5.8)`

---

## Task 9: Final Verification + Polish

### Step 1: Full verification suite

```bash
export PATH="/opt/homebrew/opt/zig@0.14/bin:$PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Step 2: Manual testing checklist

```bash
cargo run -p conescope-rs
```

1. App opens with TopBar ("CONESCOPE") + empty grid ([+] slot) + ActivityBar
2. Click [+] → New Instance modal appears
3. Click "New Terminal" → modal closes, terminal tile appears in grid
4. Terminal tile shows mini terminal with shell prompt
5. Click tile → switches to Focus mode with full terminal
6. Type `ls` in terminal → output appears
7. TopBar shows instance title + [←] back button
8. Click [←] → returns to Overview grid
9. Click [+] again → create second terminal → grid adjusts to 2 columns
10. ActivityBar shows instance number buttons
11. Click instance number in ActivityBar → focuses that instance
12. Tokens/cost display: "0k tokens $0.00"

### Step 3: Fix any issues discovered in manual testing

### Step 4: Commit

```bash
# If there were polish fixes:
git add -A && git commit -m "fix: Phase 5 polish fixes"
```

Final commit: `feat: Phase 5 complete — Overview + Focus UI with instance creation`

---

## Data Flow Summary

```
User clicks [+] → TopBar → app_state.toggle_new_instance_modal()
  → AppView re-renders → NewInstanceModal overlay shown

User clicks "New Terminal" → window.dispatch_action(CreateInstanceAction)
  → AppView.handle_create_instance()
  → spawn_terminal_pane(cwd, window, cx)       [needs Window]
  → InstanceEntry::from_instance() + attach_terminal()
  → start_output_polling(cx)                     [no Window needed]
  → InstanceList.push_entry(entry)
  → OverviewGrid re-renders → tile appears

User clicks tile → app_state.focus_instance(id)
  → AppView re-renders with ViewMode::Focus
  → FocusView renders focused instance's TerminalView full-size
  → Window resize observer updates PTY dimensions

User clicks [←] → app_state.return_to_overview()
  → AppView re-renders with ViewMode::Overview
  → OverviewGrid renders all tiles
```

## Key Files Summary

| File | Action | Purpose |
|------|--------|---------|
| `conescope-ui/src/state/instance_entry.rs` | Modify | Remove window param from start_output_polling |
| `conescope-ui/src/state/instance_list.rs` | Modify | Update callers, add push_entry |
| `conescope-ui/src/lib.rs` | Modify | Add `pub mod views;` |
| `conescope-ui/src/views/mod.rs` | Create | Module re-exports |
| `conescope-ui/src/views/app_view.rs` | Create | Root view entity |
| `conescope-ui/src/views/top_bar.rs` | Create | Top bar component |
| `conescope-ui/src/views/activity_bar.rs` | Create | Bottom bar component |
| `conescope-ui/src/views/overview_grid.rs` | Create | Grid of instance tiles |
| `conescope-ui/src/views/focus_view.rs` | Create | Full terminal view |
| `conescope-ui/src/views/new_instance_modal.rs` | Create | Instance creation modal |
| `conescope-ui/src/views/colors.rs` | Create | Color parsing helpers |
| `conescope/src/main.rs` | Rewrite | Use AppView instead of standalone terminal |
