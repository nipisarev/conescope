# Phase 6: Keyboard Shortcuts, Instance Lifecycle, Session Restore

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add keyboard shortcuts (Cmd+0/1-9/N), instance close/delete, terminal session restore on startup, and terminal focus management — making the app feel like a real native tool.

**Architecture:** GPUI actions defined via `actions!` macro, key bindings registered on `App`, action handlers on the root `Div` in AppView's render (gives `&mut Window` + `&mut App`). Session restore wires the existing `InstanceList::restore_terminals()` into the startup async task. Instance close kills the PTY and removes from list.

**Tech Stack:** Existing workspace deps. No new crates needed.

**GPUI action API notes:**
- `actions!(namespace, [ActionName])` — defines unit struct + registers it
- `KeyBinding::new("cmd-n", ActionName, None)` — bind key to action
- `cx.bind_keys([...])` — register at app level
- `Div::on_action(impl Fn(&A, &mut Window, &mut App))` — element-level handler with Window access
- `App::on_action(impl Fn(&A, &mut App))` — global handler, no Window
- Key events bubble up: terminal focus → parent divs → root. Cmd+N won't conflict with terminal Ctrl+ shortcuts.
- zig@0.14 must be linked as default (`brew link zig@0.14 --force`)

---

## Checks Configuration

**Error classes for this phase:** typing (GPUI action generics), borrow (entity read-before-update), races (PTY kill vs polling thread), leaks (detached tasks after instance close).

```bash
# After each task:
just verify
```

---

## Task 1: Define Actions + Register Key Bindings

**Goal:** Define all app actions as unit structs and register keyboard shortcuts at app startup.

### Step 1: Create actions module

**File:** Create `crates/conescope-ui/src/actions.rs`

```rust
use gpui::actions;

actions!(
    conescope,
    [
        NewInstance,
        CloseInstance,
        ReturnToOverview,
        FocusInstance1,
        FocusInstance2,
        FocusInstance3,
        FocusInstance4,
        FocusInstance5,
        FocusInstance6,
        FocusInstance7,
        FocusInstance8,
        FocusInstance9,
    ]
);
```

### Step 2: Export from lib.rs

**File:** Modify `crates/conescope-ui/src/lib.rs`

Add `pub mod actions;` line.

### Step 3: Register key bindings in main.rs

**File:** Modify `crates/conescope/src/main.rs`

After existing `cx.bind_keys([...])` block, add:

```rust
use conescope_ui::actions::*;

cx.bind_keys([
    KeyBinding::new("cmd-n", NewInstance, None),
    KeyBinding::new("cmd-w", CloseInstance, None),
    KeyBinding::new("cmd-0", ReturnToOverview, None),
    KeyBinding::new("cmd-1", FocusInstance1, None),
    KeyBinding::new("cmd-2", FocusInstance2, None),
    KeyBinding::new("cmd-3", FocusInstance3, None),
    KeyBinding::new("cmd-4", FocusInstance4, None),
    KeyBinding::new("cmd-5", FocusInstance5, None),
    KeyBinding::new("cmd-6", FocusInstance6, None),
    KeyBinding::new("cmd-7", FocusInstance7, None),
    KeyBinding::new("cmd-8", FocusInstance8, None),
    KeyBinding::new("cmd-9", FocusInstance9, None),
]);
```

### Step 4: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Commit: `feat: define GPUI actions and register keyboard shortcuts (Phase 6.1)`

---

## Task 2: Wire Action Handlers on Root Div

**Goal:** Handle keyboard actions to create instances, switch views, and focus instances.

### Step 1: Add helper to find instance by number

**File:** Modify `crates/conescope-ui/src/state/instance_list.rs`

Add method:

```rust
/// Find entry by 1-based instance number.
#[must_use]
pub fn find_by_number(&self, number: i64, cx: &gpui::App) -> Option<&Entity<InstanceEntry>> {
    self.entries
        .iter()
        .find(|e| e.read(cx).instance.instance_number == Some(number))
}
```

### Step 2: Add action handlers to AppView render

**File:** Modify `crates/conescope-ui/src/views/app_view.rs`

Import actions and add `.on_action()` calls on the root div in `render()`:

```rust
use crate::actions::*;

// In render(), on the root div():
.on_action({
    let app_state = self.app_state.clone();
    move |_: &NewInstance, _window, cx| {
        app_state.update(cx, AppState::toggle_new_instance_modal);
    }
})
.on_action({
    let app_state = self.app_state.clone();
    move |_: &ReturnToOverview, _window, cx| {
        app_state.update(cx, AppState::return_to_overview);
    }
})
.on_action({
    let app_state = self.app_state.clone();
    move |_: &CloseInstance, _window, cx| {
        // Only close in Focus mode
        let state = app_state.read(cx);
        if state.view_mode(cx) != ViewMode::Focus {
            return;
        }
        let Some(id) = state.focused_instance_id(cx) else { return };
        let id = id.to_owned();
        drop(state);
        // Return to overview first, then remove
        app_state.update(cx, AppState::return_to_overview);
        let il = app_state.read(cx).instance_list.clone();
        il.update(cx, |list, cx| list.remove_instance(&id, cx));
    }
})
```

For FocusInstance1-9, add a helper function and wire each:

```rust
fn focus_instance_n(n: i64, app_state: &Entity<AppState>, cx: &mut gpui::App) {
    let state = app_state.read(cx);
    let il = state.instance_list.read(cx);
    if let Some(entry) = il.find_by_number(n, cx) {
        let id = entry.read(cx).id().to_owned();
        drop(il);
        drop(state);
        app_state.update(cx, |s, cx| s.focus_instance(&id, cx));
    }
}

// Then on the root div:
.on_action({
    let app_state = self.app_state.clone();
    move |_: &FocusInstance1, _window, cx| { focus_instance_n(1, &app_state, cx); }
})
// ... repeat for 2-9
```

### Step 3: Make root div focusable

The root div must participate in focus dispatch for key events to reach it. In the root div:

```rust
div()
    .id("app-root")
    .key_context("AppView")
    // ... rest of render
```

The `id()` and `key_context()` make the div participate in action dispatch. GPUI dispatches actions to elements with matching key contexts.

### Step 4: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs
# Test: Cmd+N should toggle new instance modal
# Test: Cmd+0 should return to overview
```

Commit: `feat: wire keyboard action handlers on AppView root (Phase 6.2)`

---

## Task 3: Instance Close/Delete Flow

**Goal:** Cmd+W in Focus mode kills the PTY and removes the instance. Close button on overview tiles.

### Step 1: Add kill_pty method to InstanceEntry

**File:** Modify `crates/conescope-ui/src/state/instance_entry.rs`

```rust
/// Kill the PTY process. Drops the master PTY handle which sends SIGHUP.
pub fn kill_pty(&mut self) {
    self.master_pty.take(); // Drop sends SIGHUP to child
    self.stdin_tx.take();   // Close input channel
    self.alive = false;
}
```

### Step 2: Update InstanceList::remove_instance to kill PTY

**File:** Modify `crates/conescope-ui/src/state/instance_list.rs`

In `remove_instance`, before removing from entries, kill the PTY:

```rust
pub fn remove_instance(&mut self, id: &str, cx: &mut gpui::Context<Self>) {
    // Kill PTY for the entry being removed
    if let Some(entry) = self.entries.iter().find(|e| e.read(cx).id() == id) {
        entry.update(cx, |e, cx| {
            e.kill_pty();
            e.mark_exited(cx);
        });
    }

    let ended = Utc::now().to_rfc3339();
    self.db.end_instance(id.to_owned(), ended);
    self.entries.retain(|e| e.read(cx).id() != id);
    cx.emit(InstanceListEvent::Removed(id.to_owned()));
    cx.notify();
}
```

### Step 3: Add close button to overview tiles

**File:** Modify `crates/conescope-ui/src/views/overview_grid.rs`

In `render_tile_header()`, add a [×] close button to the right side:

```rust
// In tile header, after status dot:
.child(
    div()
        .cursor_pointer()
        .text_color(rgba(0x6666_66ff))
        .hover(|s| s.text_color(rgba(0xcccc_ccff)))
        .child("\u{00d7}") // ×
        .on_mouse_down(MouseButton::Left, {
            let app_state = app_state.clone();
            let tile_id = tile_id.clone();
            move |_, _, cx| {
                let il = app_state.read(cx).instance_list.clone();
                il.update(cx, |list, cx| list.remove_instance(&tile_id, cx));
            }
        })
)
```

### Step 4: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs
# Test: Create instance → click [×] on tile → tile removed
# Test: Focus instance → Cmd+W → returns to overview, instance removed
```

Commit: `feat: instance close with PTY kill and overview close button (Phase 6.3)`

---

## Task 4: Session Restore on App Restart

**Goal:** When the app starts, restore instances from DB and spawn fresh PTYs for them.

### Step 1: Wire restore_terminals into startup

**File:** Modify `crates/conescope/src/main.rs`

The async DB load task currently loads settings, projects, and instances. After loading instances, we need to call `restore_terminals()` — but that needs `&mut Window`, which isn't available in `cx.spawn()`.

Instead, use a window-level observer. After loading instances, notify the window to trigger restore.

Better approach: Load instances in the async task, then use `window_handle.update()` to call `restore_terminals`.

```rust
// In the open_window callback, AFTER creating AppView, capture the window handle:
let window_handle = cx.window_handle();

// In the async spawn task, after loading instances:
if let Ok(Ok(instances)) = db_for_load.get_all_instances().recv() {
    cx.update(|cx| {
        instance_list.update(cx, |list, cx| list.load_from_db(instances, cx));
    });
    info!("Instances loaded");

    // Restore terminals (needs Window context)
    let project_store_for_restore = project_store.clone();
    let _ = window_handle.update(&mut cx, |window, cx| {
        instance_list.update(cx, |list, cx| {
            list.restore_terminals(&project_store_for_restore, window, cx);
        });
    });
    info!("Terminals restored");
}
```

Note: `window_handle.update()` takes `&mut AsyncApp` and gives `|&mut Window, &mut App|`. Check the exact GPUI API — it might be `AnyWindowHandle::update(&self, cx: &mut AsyncApp, f: impl FnOnce(&mut Window, &mut App) -> R) -> Result<R>`.

### Step 2: Filter out stale instances

Only restore instances that are not already ended. In the async task, filter:

```rust
let instances: Vec<_> = instances.into_iter()
    .filter(|i| i.ended_at.is_none())
    .collect();
```

### Step 3: Move async spawn into window callback

The async task needs `window_handle` which is only available inside `open_window`. Restructure:

```rust
cx.open_window(options, |window, cx| {
    let view = cx.new(|cx| AppView::new(app_state.clone(), cx));

    let resize_sub = register_focus_resize(&app_state, window, cx);
    resize_sub.detach();

    // Move async DB load here so we have window handle
    let window_handle = cx.window_handle();
    let app_state_for_load = app_state.clone();
    cx.spawn(async move |_view, mut cx| {
        // ... load settings, projects, instances ...
        // ... then restore_terminals via window_handle.update() ...
    }).detach();

    view
});
```

Wait — `cx` inside the window callback is `Context<AppView>`, not `App`. `Context<T>::spawn` gives `AsyncFnOnce(WeakEntity<T>, &mut AsyncApp)`. We need `AsyncApp` to call `window_handle.update(&mut cx, ...)`.

Actually, the `cx.spawn` from `Context<AppView>` gives us `|_weak, cx: &mut AsyncApp|`. And `window_handle` is `AnyWindowHandle`. We need `cx.update_window(window_handle, |window, cx| ...)`. Let me check the exact API.

The safest approach: keep the spawn on `App` level (outside open_window), pass the `AnyWindowHandle` out.

```rust
let window = cx.open_window(options, |window, cx| {
    // ...
    view
}).expect("Failed to open window");

// Now spawn async task with window handle
let window_handle = window;
cx.spawn(async move |cx| {
    // load data...
    // restore via:
    let _ = cx.update_window(window_handle, |window, cx| {
        instance_list.update(cx, |list, cx| {
            list.restore_terminals(&project_store, window, cx);
        });
    });
}).detach();
```

`App::spawn` gives `AsyncFnOnce(AsyncApp)`. `AsyncApp` has `update_window(handle, |window, cx| ...)`.

### Step 4: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs
# Test: Create 2 instances → quit app → restart → instances appear in grid with live terminals
```

Commit: `feat: restore instance terminals on app startup (Phase 6.4)`

---

## Task 5: Terminal Focus Management

**Goal:** Clicking a terminal tile or entering Focus mode should focus the terminal so keyboard input goes to the PTY.

### Step 1: Add focus_terminal method to InstanceEntry

**File:** Modify `crates/conescope-ui/src/state/instance_entry.rs`

```rust
/// Focus this instance's terminal view, if attached.
pub fn focus_terminal(&self, window: &mut gpui::Window, cx: &mut gpui::App) {
    if let Some(ref tv) = self.terminal_view {
        tv.update(cx, |view, cx| {
            view.focus_handle(cx).focus(window, cx);
        });
    }
}
```

Note: Check if `TerminalView` exposes `focus_handle()`. Looking at gpui-ghostty, it likely does since it was created with one. If not, store the `FocusHandle` in `InstanceEntry` during `attach_terminal`.

### Step 2: Focus terminal on entering Focus mode

**File:** Modify `crates/conescope-ui/src/views/app_view.rs`

In the `FocusInstance1-9` handlers and tile click handlers, after calling `focus_instance()`, also focus the terminal:

```rust
// After app_state.update(cx, |s, cx| s.focus_instance(&id, cx)):
let state = app_state.read(cx);
let il = state.instance_list.read(cx);
if let Some(entry) = il.find_by_id(&id, cx) {
    entry.read(cx).focus_terminal(window, cx);
}
```

This works because `Div::on_action` gives `(&A, &mut Window, &mut App)`.

### Step 3: Focus terminal when clicking FocusView area

**File:** Modify `crates/conescope-ui/src/views/focus_view.rs`

Add click handler on the terminal container in FocusView render:

```rust
div()
    .size_full()
    .flex()
    .flex_col()
    .on_mouse_down(MouseButton::Left, {
        let entry = entry.clone();
        move |_, window, cx| {
            entry.read(cx).focus_terminal(window, cx);
        }
    })
    .child(div().flex_1().child(tv.clone()))
```

### Step 4: Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conescope-rs
# Test: Create instance → click tile → Focus mode → type in terminal → text appears
# Test: Cmd+1 → Focus mode → terminal focused → keyboard input works
```

Commit: `feat: terminal focus management on mode switch and click (Phase 6.5)`

---

## Task 6: Final Verification + Commit

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

1. App opens with Overview (empty grid + [+] slot)
2. **Cmd+N** → modal appears → click "New Terminal" → tile in grid
3. Click tile → Focus mode → terminal works (type `ls`, see output)
4. **Cmd+0** → back to Overview
5. **Cmd+1** → focuses instance #1 → terminal has focus
6. **Cmd+N** → modal → create second terminal
7. **Cmd+2** → focuses instance #2
8. **Cmd+W** → closes instance #2, returns to Overview
9. Click [×] on remaining tile → tile removed, empty grid
10. Create 2 instances → quit → restart → instances restored with live terminals
11. Cmd+1-9 only works for existing instances (no crash for missing)

### Step 3: Fix any issues discovered

### Step 4: Commit

Commit: `feat: Phase 6 complete — keyboard shortcuts, instance lifecycle, session restore`

---

## Key Files Summary

| File | Action | Purpose |
|------|--------|---------|
| `conescope-ui/src/actions.rs` | Create | Action definitions (actions! macro) |
| `conescope-ui/src/lib.rs` | Modify | Export actions module |
| `conescope-ui/src/state/instance_entry.rs` | Modify | Add kill_pty, focus_terminal |
| `conescope-ui/src/state/instance_list.rs` | Modify | Add find_by_number, update remove_instance |
| `conescope-ui/src/views/app_view.rs` | Modify | Wire action handlers on root div |
| `conescope-ui/src/views/overview_grid.rs` | Modify | Add [×] close button to tiles |
| `conescope-ui/src/views/focus_view.rs` | Modify | Focus terminal on click |
| `conescope/src/main.rs` | Modify | Key bindings, session restore |
