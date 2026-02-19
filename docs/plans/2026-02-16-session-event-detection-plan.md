# Session Event Detection & Question Overlay — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Detect when Claude CLI instances ask questions or wait for input, show interactive overlays on overview tiles, play sounds, and display status in sidebar.

**Architecture:** Two-phase detection (StreamWatcher pre-filters raw PTY bytes, ScreenAnalyzer classifies screen buffer snapshots on idle). Interactive question overlays on overview tiles allow direct answering. Pulsing status dots across overview, sidebar, and focus mode.

**Tech Stack:** Rust, GPUI framework, alacritty_terminal (Zed fork), portable-pty, afplay (macOS), strip-ansi-escapes crate

**Design doc:** `docs/plans/2026-02-16-session-event-detection-design.md`

---

## Task 1: Add SessionStatus enum and SessionEvent types

**Why:** Foundation types needed by all other tasks.

**Files:**
- Create: `crates/conescope-ui/src/state/session_detector.rs`
- Modify: `crates/conescope-ui/src/state/mod.rs`

**Step 1: Create session_detector.rs with types**

Create file with SessionEvent, SessionStatus enums and the SessionDetector struct skeleton:

```rust
use std::time::Instant;

#[derive(Clone, Debug)]
pub enum SessionEvent {
    Question {
        text: String,
        choices: Vec<String>,
        screen_snapshot: String,
    },
    WaitingForInput,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SessionStatus {
    #[default]
    Working,
    Question,
    Waiting,
    Finished,
    Stopped,
}

impl SessionStatus {
    pub fn needs_attention(&self) -> bool {
        matches!(self, Self::Question | Self::Waiting | Self::Finished)
    }

    pub fn is_pulsing(&self) -> bool {
        matches!(self, Self::Question | Self::Waiting)
    }
}

pub struct SessionDetector {
    trigger_pending: bool,
    last_output_time: Instant,
    idle_threshold_ms: u64,
    pub session_event: Option<SessionEvent>,
    pub session_status: SessionStatus,
}

impl SessionDetector {
    pub fn new() -> Self {
        Self {
            trigger_pending: false,
            last_output_time: Instant::now(),
            idle_threshold_ms: 200,
            session_event: None,
            session_status: SessionStatus::Working,
        }
    }

    /// Called on every PTY output batch. Records activity and pre-filters.
    pub fn on_output(&mut self, raw_bytes: &[u8]) {
        self.last_output_time = Instant::now();
        // Phase 1: StreamWatcher — scan stripped bytes for triggers
        if Self::scan_triggers(raw_bytes) {
            self.trigger_pending = true;
        }
    }

    /// Called periodically (16ms). Returns true if screen analysis should run.
    pub fn should_analyze(&self) -> bool {
        self.trigger_pending
            && self.last_output_time.elapsed().as_millis() as u64 >= self.idle_threshold_ms
    }

    /// Phase 2: Classify screen content. Called with extracted screen text.
    pub fn analyze_screen(&mut self, screen_text: &str, last_lines: &str) {
        self.trigger_pending = false;

        // Check status bar (last 2 lines) for high-confidence signals
        let lower_last = last_lines.to_lowercase();

        if lower_last.contains("esc to interrupt") || lower_last.contains("esc to cancel") {
            // Negative signal: Claude is working
            self.session_status = SessionStatus::Working;
            self.session_event = None;
            return;
        }

        // Check for question indicators
        if lower_last.contains("enter to select") || lower_last.contains("to submit") {
            // High confidence: question with options
            let (text, choices) = Self::extract_question(screen_text);
            self.session_status = SessionStatus::Question;
            self.session_event = Some(SessionEvent::Question {
                text,
                choices,
                screen_snapshot: screen_text.to_string(),
            });
            return;
        }

        // Check body for [y/N] or [Y/n] patterns
        if screen_text.contains("[y/N]") || screen_text.contains("[Y/n]")
            || screen_text.contains("[y/n]") || screen_text.contains("[Y/N]")
        {
            self.session_status = SessionStatus::Question;
            self.session_event = Some(SessionEvent::Question {
                text: Self::extract_yn_question(screen_text),
                choices: vec!["Yes".into(), "No".into()],
                screen_snapshot: screen_text.to_string(),
            });
            return;
        }

        // Check for numbered options (1. ... 2. ... 3. ...)
        let numbered = Self::extract_numbered_options(screen_text);
        if numbered.len() >= 2 {
            let text = Self::extract_question_before_options(screen_text);
            self.session_status = SessionStatus::Question;
            self.session_event = Some(SessionEvent::Question {
                text,
                choices: numbered,
                screen_snapshot: screen_text.to_string(),
            });
            return;
        }

        // If idle but no question patterns, might be waiting for general input
        self.session_status = SessionStatus::Waiting;
        self.session_event = Some(SessionEvent::WaitingForInput);
    }

    /// Reset to working state (e.g., when PTY output resumes after answering)
    pub fn reset_to_working(&mut self) {
        self.session_status = SessionStatus::Working;
        self.session_event = None;
        self.trigger_pending = false;
    }

    /// Mark as stopped (process exited)
    pub fn mark_stopped(&mut self) {
        self.session_status = SessionStatus::Stopped;
        self.session_event = None;
    }

    // --- Private helpers ---

    fn scan_triggers(raw_bytes: &[u8]) -> bool {
        // Strip ANSI and check for trigger patterns
        let stripped = strip_ansi_escapes::strip(raw_bytes);
        let text = String::from_utf8_lossy(&stripped);
        let lower = text.to_lowercase();

        lower.contains("[y/n]")
            || lower.contains("[y/n]")
            || lower.contains("enter to select")
            || lower.contains("to submit")
            || text.contains("1.")
            || text.contains("2.")
    }

    fn extract_question(screen_text: &str) -> (String, Vec<String>) {
        // Extract question text and choices from screen
        // Look for numbered options pattern
        let choices = Self::extract_numbered_options(screen_text);
        let text = Self::extract_question_before_options(screen_text);
        (text, choices)
    }

    fn extract_yn_question(screen_text: &str) -> String {
        // Find the line containing [y/N] and return it as question text
        for line in screen_text.lines().rev() {
            if line.contains("[y/N]") || line.contains("[Y/n]")
                || line.contains("[y/n]") || line.contains("[Y/N]")
            {
                return line.trim().to_string();
            }
        }
        "Confirm?".to_string()
    }

    fn extract_numbered_options(screen_text: &str) -> Vec<String> {
        let mut options = Vec::new();
        let mut expected_num = 1;
        for line in screen_text.lines() {
            let trimmed = line.trim();
            let prefix = format!("{}.", expected_num);
            if trimmed.starts_with(&prefix) {
                let option_text = trimmed[prefix.len()..].trim().to_string();
                if !option_text.is_empty() {
                    options.push(option_text);
                    expected_num += 1;
                }
            }
        }
        options
    }

    fn extract_question_before_options(screen_text: &str) -> String {
        // Find text before the first numbered option
        for line in screen_text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("1.") {
                break;
            }
            if trimmed.ends_with('?') && !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        // Fallback: return last non-empty line before options
        let lines: Vec<&str> = screen_text.lines().collect();
        for line in lines.iter().rev() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with(|c: char| c.is_ascii_digit()) {
                return trimmed.to_string();
            }
        }
        String::new()
    }
}

impl Default for SessionDetector {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 2: Add module declaration**

In `crates/conescope-ui/src/state/mod.rs`, add:
```rust
pub mod session_detector;
```

**Step 3: Add strip-ansi-escapes dependency**

Run: `cd crates/conescope-ui && cargo add strip-ansi-escapes`

**Step 4: Verify compilation**

Run: `just check`
Expected: PASS (no clippy warnings)

**Step 5: Commit**

```bash
git add crates/conescope-ui/src/state/session_detector.rs crates/conescope-ui/src/state/mod.rs crates/conescope-ui/Cargo.toml Cargo.lock
git commit -m "feat: add SessionDetector with two-phase detection types"
```

---

## Task 2: Add screen text extraction to Terminal

**Why:** ScreenAnalyzer needs to read text from the Terminal's grid. We need a method to extract full screen text and last N lines from TerminalContent.

**Files:**
- Modify: `crates/conescope-ui/src/terminal/terminal.rs`

**Step 1: Add text extraction methods to TerminalContent**

Add these methods to the existing `impl TerminalContent` block (after the Default impl, around line 120):

```rust
impl TerminalContent {
    /// Extract full screen text as a string, one line per row.
    pub fn screen_text(&self) -> String {
        if self.cols == 0 || self.lines == 0 {
            return String::new();
        }
        let mut rows: Vec<String> = vec![String::new(); self.lines];
        for cell in &self.cells {
            let row = cell.point.line.0 as usize;
            if row < self.lines {
                rows[row].push(cell.c);
            }
        }
        // Trim trailing whitespace per line
        rows.iter()
            .map(|r| r.trim_end().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract the last N lines of screen text (for status bar analysis).
    pub fn last_lines(&self, n: usize) -> String {
        let text = self.screen_text();
        let lines: Vec<&str> = text.lines().collect();
        let start = lines.len().saturating_sub(n);
        lines[start..].join("\n")
    }
}
```

**Step 2: Verify compilation**

Run: `just check`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/conescope-ui/src/terminal/terminal.rs
git commit -m "feat: add screen_text() and last_lines() to TerminalContent"
```

---

## Task 3: Integrate SessionDetector into InstanceEntry

**Why:** Wire up detection in the PTY polling loop. This is the core integration point.

**Files:**
- Modify: `crates/conescope-ui/src/state/instance_entry.rs`

**Step 1: Add SessionDetector field to InstanceEntry**

Add to the struct fields (around line 60):
```rust
pub session_detector: crate::state::session_detector::SessionDetector,
```

Import at top of file:
```rust
use crate::state::session_detector::{SessionDetector, SessionEvent, SessionStatus};
```

Initialize in `from_instance()` constructor (around line 96):
```rust
session_detector: SessionDetector::new(),
```

**Step 2: Add accessor methods**

Add after the `send_input()` method (around line 411):

```rust
pub fn session_status(&self) -> SessionStatus {
    self.session_detector.session_status
}

pub fn session_event(&self) -> &Option<SessionEvent> {
    &self.session_detector.session_event
}

pub fn answer_question(&mut self, choice_index: usize) {
    if let Some(SessionEvent::Question { ref choices, .. }) = self.session_detector.session_event {
        if choice_index < choices.len() {
            // Write the choice number (1-indexed) + Enter to PTY
            let input = format!("{}\n", choice_index + 1);
            self.send_input(input.as_bytes());
        }
    }
    self.session_detector.reset_to_working();
}

pub fn dismiss_waiting(&mut self) {
    self.session_detector.reset_to_working();
}
```

**Step 3: Modify start_output_polling() to integrate detection**

Replace the existing polling loop in `start_output_polling()` (starting at line 524) with:

```rust
pub fn start_output_polling(&mut self, cx: &mut gpui::Context<Self>) {
    let rx = self.stdout_rx.take();
    let tv = self.terminal_view.clone();
    let weak = cx.weak_entity();

    if let (Some(rx), Some(tv)) = (rx, tv) {
        cx.spawn(async move |_weak_self, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let mut batch = Vec::new();
                while let Ok(chunk) = rx.try_recv() {
                    batch.extend_from_slice(&chunk);
                }

                if batch.is_empty() {
                    // Check if entity still exists
                    if weak.upgrade().is_none() {
                        break;
                    }

                    // Check if we should analyze (idle + trigger pending)
                    let should_analyze = cx.update(|cx| {
                        weak.upgrade()
                            .map(|e| e.read(cx).session_detector.should_analyze())
                            .unwrap_or(false)
                    }).ok().unwrap_or(false);

                    if should_analyze {
                        // Get screen text from terminal
                        let screen_data = cx.update(|cx| {
                            tv.read(cx).terminal.read(cx).last_content.screen_text()
                        }).ok();
                        let last_lines_data = cx.update(|cx| {
                            tv.read(cx).terminal.read(cx).last_content.last_lines(2)
                        }).ok();

                        if let (Some(screen), Some(last_lines)) = (screen_data, last_lines_data) {
                            let _ = cx.update(|cx| {
                                if let Some(entry) = weak.upgrade() {
                                    entry.update(cx, |e, cx| {
                                        let prev_status = e.session_detector.session_status;
                                        e.session_detector.analyze_screen(&screen, &last_lines);
                                        if e.session_detector.session_status != prev_status {
                                            cx.notify();
                                            cx.emit(InstanceEvent::StatusChanged(
                                                e.instance.status,
                                            ));
                                        }
                                    });
                                }
                            });
                        }
                    }
                    continue;
                }

                // Non-empty batch: feed to terminal and detector
                let _ = cx.update(|cx| {
                    if let Some(entry) = weak.upgrade() {
                        entry.update(cx, |e, cx| {
                            e.history.push(batch.clone());
                            // If we were in a non-Working state, PTY output means we're working again
                            let prev_status = e.session_detector.session_status;
                            e.session_detector.on_output(&batch);
                            if prev_status != SessionStatus::Working
                                && e.session_detector.session_status != prev_status
                            {
                                e.session_detector.reset_to_working();
                                cx.notify();
                            }
                        });
                    }
                    tv.update(cx, |view, cx| {
                        view.queue_output_bytes(&batch, cx);
                    });
                });
            }
        })
        .detach();
    }
}
```

**Step 4: Update mark_exited to set Stopped status**

In `mark_exited()` method (around line 200), add:
```rust
self.session_detector.mark_stopped();
```

**Step 5: Verify compilation**

Run: `just check`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/conescope-ui/src/state/instance_entry.rs
git commit -m "feat: integrate SessionDetector into InstanceEntry polling loop"
```

---

## Task 4: Create PulseTimer entity

**Why:** Shared animation timer for all pulsing status dots. Single entity, all views observe it.

**Files:**
- Create: `crates/conescope-ui/src/state/pulse_timer.rs`
- Modify: `crates/conescope-ui/src/state/mod.rs`
- Modify: `crates/conescope-ui/src/state/app_state.rs`

**Step 1: Create pulse_timer.rs**

```rust
use std::time::Instant;

pub struct PulseTimer {
    start: Instant,
    pub opacity: f32,
}

impl PulseTimer {
    pub fn new(cx: &mut gpui::Context<Self>) -> Self {
        // Start animation loop — tick every ~33ms (30fps)
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(33))
                    .await;
                let should_continue = cx.update(|cx| {
                    if let Some(timer) = this.upgrade() {
                        timer.update(cx, |t, cx| {
                            t.tick();
                            cx.notify();
                        });
                        true
                    } else {
                        false
                    }
                }).unwrap_or(false);
                if !should_continue {
                    break;
                }
            }
        })
        .detach();

        Self {
            start: Instant::now(),
            opacity: 1.0,
        }
    }

    fn tick(&mut self) {
        let t = self.start.elapsed().as_secs_f32();
        // Sine wave: 1.5 second period, opacity range 0.4 to 1.0
        let phase = (t * std::f32::consts::TAU / 1.5).sin();
        self.opacity = 0.4 + 0.6 * ((phase + 1.0) / 2.0);
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }
}
```

**Step 2: Add module declaration**

In `crates/conescope-ui/src/state/mod.rs`, add:
```rust
pub mod pulse_timer;
```

**Step 3: Add PulseTimer to AppState**

In `app_state.rs`, add field to AppState struct (around line 40):
```rust
pub pulse_timer: gpui::Entity<crate::state::pulse_timer::PulseTimer>,
```

Initialize in AppState::new() (around line 75, after git_store):
```rust
let pulse_timer = cx.new(|cx| crate::state::pulse_timer::PulseTimer::new(cx));
```

Then add it to the returned struct:
```rust
pulse_timer,
```

**Step 4: Verify compilation**

Run: `just check`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/conescope-ui/src/state/pulse_timer.rs crates/conescope-ui/src/state/mod.rs crates/conescope-ui/src/state/app_state.rs
git commit -m "feat: add PulseTimer entity for synchronized pulsing animation"
```

---

## Task 5: Add session status to overview grid tiles

**Why:** Show colored pulsing status dots and prepare for question overlay rendering.

**Files:**
- Modify: `crates/conescope-ui/src/views/overview_grid.rs`

**Step 1: Extend TileData with session fields**

Add to TileData struct (around line 528):
```rust
session_status: crate::state::session_detector::SessionStatus,
session_event: Option<crate::state::session_detector::SessionEvent>,
```

Import at top of file:
```rust
use crate::state::session_detector::{SessionEvent, SessionStatus};
```

**Step 2: Populate session fields when building TileData**

In the `render()` method where TileData is constructed (around line 485), add:
```rust
session_status: inst.session_status(),
session_event: inst.session_event().clone(),
```

**Step 3: Update status dot color to use SessionStatus**

Replace the status dot rendering in `render_tile_controls()` (around line 211) with:

```rust
let status_rgba = match tile.session_status {
    SessionStatus::Working => gpui::rgba(0x4ade80ff),   // green
    SessionStatus::Question => gpui::rgba(0xf87171ff),  // red
    SessionStatus::Waiting => gpui::rgba(0xfacc15ff),   // yellow
    SessionStatus::Finished => gpui::rgba(0xfacc15ff),  // yellow
    SessionStatus::Stopped => gpui::rgba(0x9ca3afff),   // gray
};
```

**Step 4: Add pulsing opacity to status dot**

First, read pulse_timer opacity in the render() method. Add after theme read (around line 426):
```rust
let pulse_opacity = state.pulse_timer.read(cx).opacity();
```

Then pass it to render_tile. Modify the function signature (around line 40):
```rust
fn render_tile(
    tile: &TileData,
    app_state: Entity<AppState>,
    editing_input: Option<&Entity<TextInput>>,
    terminal_font_size: f32,
    font_family: &str,
    pulse_opacity: f32,
    theme: &Theme,
) -> gpui::AnyElement {
```

Update the call sites in render() (around line 501):
```rust
render_tile(
    tile,
    self.app_state.clone(),
    input,
    terminal_font_size,
    &font_family,
    pulse_opacity,
    theme,
)
```

Then update render_tile_header signature and call:
```rust
fn render_tile_header(
    tile: &TileData,
    status_rgba: gpui::Rgba,
    app_state: Entity<AppState>,
    editing_input: Option<&Entity<TextInput>>,
    font_size: f32,
    pulse_opacity: f32,
    theme: &Theme,
) -> gpui::AnyElement {
```

Update the call in render_tile (around line 59):
```rust
.child(render_tile_header(
    tile,
    status_rgba,
    app_state.clone(),
    editing_input,
    terminal_font_size,
    pulse_opacity,
    theme,
))
```

Update render_tile_controls signature and call:
```rust
fn render_tile_controls(
    tile: &TileData,
    status_rgba: gpui::Rgba,
    icon_size: gpui::Pixels,
    close_state: Entity<AppState>,
    close_id: String,
    close_title: String,
    pulse_opacity: f32,
    theme: &Theme,
) -> gpui::Div {
```

Update the call in render_tile_header (around line 170):
```rust
let right = render_tile_controls(
    tile,
    status_rgba,
    icon_size,
    close_state,
    close_id,
    close_title,
    pulse_opacity,
    theme,
);
```

Finally, apply pulsing to the dot in render_tile_controls (around line 211):
```rust
let dot_opacity = if tile.session_status.is_pulsing() {
    pulse_opacity
} else {
    1.0
};

// ... later in the child chain:
.child(
    div()
        .w(px(8.))
        .h(px(8.))
        .rounded(px(4.))
        .bg(status_rgba)
        .opacity(dot_opacity)
        .flex_shrink_0(),
)
```

**Step 5: Verify compilation**

Run: `just check`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/conescope-ui/src/views/overview_grid.rs
git commit -m "feat: show session status colors and pulsing dots on overview tiles"
```

---

## Task 6: Create QuestionOverlay view

**Why:** The interactive overlay that shows question text, choices, and "More context" button on top of overview tiles.

**Files:**
- Create: `crates/conescope-ui/src/views/question_overlay.rs`
- Modify: `crates/conescope-ui/src/views/mod.rs`

**Step 1: Create question_overlay.rs**

```rust
use gpui::*;
use crate::state::session_detector::SessionEvent;
use crate::theme::Theme;

/// Renders question overlay on top of an overview tile.
/// Not a GPUI Entity/View — just a function returning an Element.
pub fn render_question_overlay(
    event: &SessionEvent,
    instance_id: String,
    app_state: gpui::Entity<crate::state::app_state::AppState>,
    theme: &Theme,
) -> AnyElement {
    match event {
        SessionEvent::Question { text, choices, .. } => {
            render_question_card(text, choices, instance_id, app_state, theme)
        }
        SessionEvent::WaitingForInput => {
            render_waiting_badge(theme)
        }
        SessionEvent::Finished => {
            render_finished_badge(theme)
        }
    }
}

fn render_question_card(
    text: &str,
    choices: &[String],
    instance_id: String,
    app_state: gpui::Entity<crate::state::app_state::AppState>,
    theme: &Theme,
) -> AnyElement {
    let bg: Hsla = theme.surface.into();
    let fg: Hsla = theme.text.into();
    let muted: Hsla = theme.text_muted.into();
    let border_color: Hsla = theme.border.into();

    let mut col = div()
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .bg(hsla(0., 0., 0., 0.5)) // dimming backdrop
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        });

    let mut card = div()
        .mx(px(12.))
        .p(px(12.))
        .rounded(px(8.))
        .bg(bg)
        .border_1()
        .border_color(border_color)
        .max_w(px(320.))
        .flex()
        .flex_col()
        .gap(px(8.));

    // Question text
    card = card.child(
        div()
            .text_color(fg)
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .child(text.to_string()),
    );

    // Choice buttons
    for (i, choice) in choices.iter().enumerate() {
        let choice_text = choice.clone();
        let iid = instance_id.clone();
        let app = app_state.clone();
        let idx = i;

        card = card.child(
            div()
                .id(SharedString::from(format!("choice-{}", i)))
                .px(px(8.))
                .py(px(6.))
                .rounded(px(4.))
                .border_1()
                .border_color(border_color)
                .cursor_pointer()
                .hover(move |s| s.bg(hsla(0., 0., 0.5, 0.1)))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(fg)
                        .child(format!("{}. {}", i + 1, choice_text)),
                )
                .on_click(move |_, _, cx| {
                    // Answer the question
                    app.update(cx, |state, cx| {
                        state.answer_instance_question(&iid, idx, cx);
                    });
                }),
        );
    }

    // "More context" button
    let iid = instance_id.clone();
    let app = app_state.clone();
    card = card.child(
        div()
            .id("more-context-btn")
            .mt(px(4.))
            .px(px(8.))
            .py(px(4.))
            .rounded(px(4.))
            .cursor_pointer()
            .hover(move |s| s.bg(hsla(0., 0., 0.5, 0.1)))
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(muted)
                    .child("More context →"),
            )
            .on_click(move |_, _, cx| {
                app.update(cx, |state, cx| {
                    state.focus_instance(&iid, cx);
                });
            }),
    );

    col = col.child(card);
    col.into_any_element()
}

fn render_waiting_badge(theme: &Theme) -> AnyElement {
    let bg: Hsla = theme.surface.into();
    let fg: Hsla = theme.text_muted.into();

    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(hsla(0., 0., 0., 0.3))
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .child(
            div()
                .px(px(16.))
                .py(px(8.))
                .rounded(px(6.))
                .bg(bg)
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(fg)
                        .child("Waiting..."),
                ),
        )
        .into_any_element()
}

fn render_finished_badge(theme: &Theme) -> AnyElement {
    let bg: Hsla = theme.surface.into();
    let fg: Hsla = theme.text_muted.into();

    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(hsla(0., 0., 0., 0.3))
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .child(
            div()
                .px(px(16.))
                .py(px(8.))
                .rounded(px(6.))
                .bg(bg)
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(fg)
                        .child("Finished"),
                ),
        )
        .into_any_element()
}
```

**Step 2: Add module declaration**

In `crates/conescope-ui/src/views/mod.rs`:
```rust
pub mod question_overlay;
```

**Step 3: Add answer_instance_question to AppState**

In `app_state.rs`, add method after `cancel_edit_title()` (around line 202):

```rust
pub fn answer_instance_question(&self, instance_id: &str, choice_index: usize, cx: &mut gpui::Context<Self>) {
    self.instance_list.update(cx, |list, cx| {
        if let Some(entry) = list.find_by_id(instance_id, cx) {
            entry.update(cx, |e, cx| {
                e.answer_question(choice_index);
                cx.notify();
            });
        }
    });
}
```

**Step 4: Verify compilation**

Run: `just check`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/conescope-ui/src/views/question_overlay.rs crates/conescope-ui/src/views/mod.rs crates/conescope-ui/src/state/app_state.rs
git commit -m "feat: add QuestionOverlay view for overview tile overlays"
```

---

## Task 7: Render question overlay on overview tiles

**Why:** Wire the QuestionOverlay into the overview grid tile rendering.

**Files:**
- Modify: `crates/conescope-ui/src/views/overview_grid.rs`

**Step 1: Add overlay rendering to render_tile_body**

In render_tile_body, wrap the terminal_child in a relative container and add conditional overlay. Replace the existing function (around line 328) with:

```rust
fn render_tile_body(
    tile: &TileData,
    app_state: Entity<AppState>,
    terminal_font_size: f32,
    font_family: &str,
    theme: &Theme,
) -> gpui::Div {
    let tile_id = tile.id.clone();
    let tile_focus_handle = tile.focus_handle.clone();
    let panel = theme.panel;

    let terminal_child = tile.terminal_view.as_ref().map(|tv| {
        div()
            .size_full()
            .relative()
            .px(px(4.))
            .child(tv.clone())
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        app_state.update(cx, |s, cx| s.focus_instance(&tile_id, cx));
                        if let Some(ref fh) = tile_focus_handle {
                            fh.focus(window, cx);
                        }
                    }),
            )
            .children(tile.session_event.as_ref().map(|event| {
                crate::views::question_overlay::render_question_overlay(
                    event,
                    tile.id.clone(),
                    app_state.clone(),
                    theme,
                )
            }))
            .into_any_element()
    });

    div()
        .flex_1()
        .overflow_hidden()
        .font_family(gpui::SharedString::from(font_family.to_owned()))
        .text_size(px(terminal_font_size))
        .line_height(relative(1.0))
        .hover(move |s| s.bg(panel))
        .children(terminal_child)
}
```

**Step 2: Verify compilation and visual test**

Run: `just check`
Then: `just run` and manually trigger a Claude question to verify overlay appears.

**Step 3: Commit**

```bash
git add crates/conescope-ui/src/views/overview_grid.rs
git commit -m "feat: render question overlay on overview tiles"
```

---

## Task 8: Add status indicators to ActivityBar

**Why:** Show per-instance status dots in the sidebar.

**Files:**
- Modify: `crates/conescope-ui/src/views/activity_bar.rs`

**Step 1: Cache instance session statuses in ActivityBar**

Add cached data field to ActivityBar struct (around line 18):
```rust
cached_statuses: Vec<(String, SessionStatus)>,
cached_pulse_opacity: f32,
```

Import SessionStatus:
```rust
use crate::state::session_detector::SessionStatus;
```

Initialize in new() (around line 19):
```rust
pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
    // Observe PulseTimer for opacity
    cx.observe(&app_state.read(cx).pulse_timer, |this, timer, cx| {
        this.cached_pulse_opacity = timer.read(cx).opacity();
        cx.notify();
    })
    .detach();

    // Observe InstanceList for status changes
    cx.observe(&app_state.read(cx).instance_list, |this, _list, cx| {
        this.update_cached_statuses(cx);
        cx.notify();
    })
    .detach();

    let mut bar = Self {
        app_state,
        cached_statuses: Vec::new(),
        cached_pulse_opacity: 1.0,
    };
    bar.update_cached_statuses(cx);
    bar
}

fn update_cached_statuses(&mut self, cx: &mut gpui::Context<Self>) {
    let state = self.app_state.read(cx);
    let il = state.instance_list.read(cx);
    self.cached_statuses = il
        .entries()
        .iter()
        .map(|e| {
            let entry = e.read(cx);
            (entry.id().to_owned(), entry.session_status())
        })
        .collect();
}
```

**Step 2: Render status badges on instance icons**

ActivityBar currently doesn't render instance icons in focus mode. We need to add instance icon rendering. Since this view only shows panel toggles in focus mode, we'll skip this for now and just note it as a future enhancement. Instead, let's prepare the rendering function for when instance icons are added:

Add this helper function after the existing render functions (around line 184):

```rust
/// Render status badge at bottom-right of icon.
fn render_status_badge(
    status: SessionStatus,
    pulse_opacity: f32,
    theme: &Theme,
) -> impl IntoElement {
    let (color, label) = match status {
        SessionStatus::Working => (gpui::rgba(0x4ade80ff), None),
        SessionStatus::Question => (gpui::rgba(0xf87171ff), Some("Q")),
        SessionStatus::Waiting => (gpui::rgba(0xfacc15ff), Some("W")),
        SessionStatus::Finished => (gpui::rgba(0xfacc15ff), Some("F")),
        SessionStatus::Stopped => (gpui::rgba(0x9ca3afff), None),
    };
    let opacity = if status.is_pulsing() { pulse_opacity } else { 1.0 };

    div()
        .absolute()
        .bottom(px(-1.))
        .right(px(-1.))
        .flex()
        .items_center()
        .gap(px(2.))
        .child(
            div()
                .w(px(6.))
                .h(px(6.))
                .rounded(px(3.))
                .bg(color)
                .opacity(opacity),
        )
        .children(label.map(|l| {
            div()
                .text_size(px(8.))
                .text_color(color)
                .opacity(opacity)
                .child(l.to_string())
        }))
}
```

NOTE: Since ActivityBar doesn't currently render instance icons in focus mode, the badge rendering is prepared but not wired up. This is intentional — the design doc calls for sidebar status indicators, but the current ActivityBar implementation only shows panel toggles.

**Step 3: Verify compilation**

Run: `just check`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/conescope-ui/src/views/activity_bar.rs
git commit -m "feat: prepare status badge rendering for ActivityBar"
```

---

## Task 9: Add macOS system sound support

**Why:** Play notification sounds on state transitions.

**Files:**
- Create: `crates/conescope-platform/src/sound.rs`
- Modify: `crates/conescope-platform/src/lib.rs`
- Modify: `crates/conescope-core/src/settings.rs`
- Modify: `crates/conescope-ui/src/state/instance_entry.rs`

**Step 1: Add sound_notifications to SettingsJson**

In `crates/conescope-core/src/settings.rs`, add field after terminal_line_height (around line 28):
```rust
#[serde(default = "default_sound_notifications")]
pub sound_notifications: bool,
```

Add default function (around line 45):
```rust
fn default_sound_notifications() -> bool {
    true
}
```

Update Default impl (around line 56):
```rust
sound_notifications: default_sound_notifications(),
```

**Step 2: Create sound.rs in conescope-platform**

```rust
#[cfg(target_os = "macos")]
pub fn play_system_sound(name: &str) {
    use std::process::Command;
    // Use afplay with system sounds as a simple approach
    // System sounds are in /System/Library/Sounds/
    let path = format!("/System/Library/Sounds/{}.aiff", name);
    std::thread::spawn(move || {
        let _ = Command::new("afplay").arg(&path).output();
    });
}

#[cfg(not(target_os = "macos"))]
pub fn play_system_sound(_name: &str) {
    // No-op on non-macOS
}

/// Play sound for a session status transition
pub fn play_status_sound(status: &str) {
    match status {
        "question" => play_system_sound("Purr"),
        "waiting" => play_system_sound("Tink"),
        "finished" => play_system_sound("Pop"),
        "stopped" => play_system_sound("Basso"),
        _ => {}
    }
}
```

**Step 3: Add module to lib.rs**

In `crates/conescope-platform/src/lib.rs`, add:
```rust
pub mod sound;
```

**Step 4: Integrate sound into InstanceEntry status transitions**

In `instance_entry.rs`, add sound playing in the polling loop. Modify the analyze_screen section (in start_output_polling, around the status change check):

```rust
if e.session_detector.session_status != prev_status {
    // Play sound on transition to attention states
    match e.session_detector.session_status {
        SessionStatus::Question => {
            conescope_platform::sound::play_status_sound("question");
        }
        SessionStatus::Waiting => {
            conescope_platform::sound::play_status_sound("waiting");
        }
        SessionStatus::Finished => {
            conescope_platform::sound::play_status_sound("finished");
        }
        SessionStatus::Stopped => {
            conescope_platform::sound::play_status_sound("stopped");
        }
        _ => {}
    }
    cx.notify();
    cx.emit(InstanceEvent::StatusChanged(
        e.instance.status,
    ));
}
```

NOTE: The current implementation plays sounds unconditionally. In a future enhancement, we'd check the settings.json sound_notifications flag. For now, we'll leave this as is since adding the settings check requires threading SettingsStore through to InstanceEntry.

**Step 5: Verify compilation**

Run: `just check`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/conescope-platform/src/sound.rs crates/conescope-platform/src/lib.rs crates/conescope-core/src/settings.rs crates/conescope-ui/src/state/instance_entry.rs
git commit -m "feat: add macOS system sound notifications for session state changes"
```

---

## Task 10: Add focus mode notification dot

**Why:** When in focus mode, show a red pulsing dot on the sidebar toggle button if any other instance has a Question.

**Files:**
- Modify: `crates/conescope-ui/src/views/top_bar.rs`
- Modify: `crates/conescope-ui/src/state/app_state.rs`

**Step 1: Add method to AppState to check for pending questions**

In `app_state.rs`, add method after `toggle_sidebar_open()` (around line 429):

```rust
pub fn has_unfocused_questions(&self, focused_id: Option<&str>, cx: &gpui::App) -> bool {
    let list = self.instance_list.read(cx);
    list.entries().iter().any(|entry| {
        let e = entry.read(cx);
        e.instance.id != focused_id.unwrap_or("")
            && e.session_status() == crate::state::session_detector::SessionStatus::Question
    })
}
```

**Step 2: Cache unfocused question state in TopBar**

Add cached field to TopBar struct (around line 13):
```rust
cached_has_unfocused_questions: bool,
cached_pulse_opacity: f32,
```

Import SessionStatus:
```rust
use crate::state::session_detector::SessionStatus;
```

Update new() to initialize and observe (around line 19):
```rust
pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
    // Observe PulseTimer for opacity
    cx.observe(&app_state.read(cx).pulse_timer, |this, timer, cx| {
        this.cached_pulse_opacity = timer.read(cx).opacity();
        cx.notify();
    })
    .detach();

    // Observe InstanceList for question state changes
    cx.observe(&app_state.read(cx).instance_list, |this, _list, cx| {
        this.update_cached_questions(cx);
        cx.notify();
    })
    .detach();

    let mut bar = Self {
        app_state,
        cached_has_unfocused_questions: false,
        cached_pulse_opacity: 1.0,
    };
    bar.update_cached_questions(cx);
    bar
}

fn update_cached_questions(&mut self, cx: &mut gpui::Context<Self>) {
    let state = self.app_state.read(cx);
    let focused_id = state.focused_instance_id(cx);
    self.cached_has_unfocused_questions = state.has_unfocused_questions(focused_id, cx);
}
```

**Step 3: Render notification dot on sidebar toggle in TopBar**

In render() method, modify the sidebar toggle button rendering (around line 192). Replace the sidebar toggle button div with:

```rust
.child({
    let mut sidebar_btn = div()
        .relative()
        .px(px(6.))
        .py(px(4.))
        .rounded(px(4.))
        .cursor_pointer()
        .hover(move |s| s.text_color(sidebar_hover))
        .child(
            svg()
                .path(icons::ICON_SIDEBAR)
                .size(icon_size)
                .text_color(sidebar_icon_color)
                .flex_shrink_0(),
        )
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            app_state_for_sidebar.update(cx, AppState::toggle_sidebar_open);
        });

    // Add notification dot if there are unfocused questions in focus mode
    if view_mode == ViewMode::Focus && self.cached_has_unfocused_questions {
        sidebar_btn = sidebar_btn.child(
            div()
                .absolute()
                .top(px(-2.))
                .right(px(-2.))
                .w(px(8.))
                .h(px(8.))
                .rounded(px(4.))
                .bg(gpui::rgba(0xf87171ff)) // red
                .opacity(self.cached_pulse_opacity)
        );
    }

    sidebar_btn
})
```

**Step 4: Verify compilation**

Run: `just check`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/conescope-ui/src/views/top_bar.rs crates/conescope-ui/src/state/app_state.rs
git commit -m "feat: add focus mode notification dot on sidebar toggle"
```

---

## Task 11: Add debug screen dump logging

**Why:** Empirical logging for iterating detection patterns against real Claude output.

**Files:**
- Modify: `crates/conescope-ui/src/state/session_detector.rs`

**Step 1: Add debug logging to analyze_screen**

Add logging function to SessionDetector impl:

```rust
#[cfg(debug_assertions)]
fn log_screen_dump(screen_text: &str, last_lines: &str, result: &SessionStatus) {
    use std::fs;

    let dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".conescope")
        .join("screen_dumps");
    let _ = fs::create_dir_all(&dir);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let filename = format!("{}_{:?}.txt", timestamp, result);
    let path = dir.join(filename);

    let content = format!(
        "=== Status: {:?} ===\n\n--- Last 2 lines ---\n{}\n\n--- Full screen ---\n{}\n",
        result, last_lines, screen_text
    );
    let _ = fs::write(path, content);
}
```

Call this at the end of `analyze_screen()`:
```rust
#[cfg(debug_assertions)]
Self::log_screen_dump(screen_text, last_lines, &self.session_status);
```

**Step 2: Verify compilation**

Run: `just check`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/conescope-ui/src/state/session_detector.rs
git commit -m "feat: add debug screen dump logging for pattern iteration"
```

---

## Task 12: Final verification and polish

**Why:** End-to-end verification that everything works together.

**Step 1: Run full verification**

Run: `just verify`
Expected: fmt-check PASS, clippy PASS (no warnings), tests PASS

**Step 2: Manual integration test**

1. `just run`
2. Create a new Claude instance
3. Give Claude a task that triggers AskUserQuestion (e.g., "Create a web server using Express or Flask")
4. Verify: red pulsing dot appears on overview tile
5. Verify: question overlay shows with clickable options
6. Click an option → verify answer sent, overlay dismisses
7. Verify: sound plays on state change
8. Switch to focus mode → verify notification dot on sidebar toggle if another instance has a question
9. Check `~/.conescope/screen_dumps/` for logged snapshots

**Step 3: Fix any issues found in manual testing**

Iterate on patterns, UI positioning, timing.

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: session event detection and question overlay system"
```

---

## Summary

| Task | Component | Est. Complexity |
|------|-----------|-----------------|
| 1 | SessionDetector types + logic | Medium |
| 2 | Terminal screen text extraction | Low |
| 3 | InstanceEntry integration | High |
| 4 | PulseTimer entity | Low |
| 5 | Overview grid status colors | Medium |
| 6 | QuestionOverlay view | Medium |
| 7 | Wire overlay into grid | Low |
| 8 | ActivityBar status badges | Medium |
| 9 | macOS sound support | Low |
| 10 | Focus mode notification dot | Medium |
| 11 | Debug screen dump logging | Low |
| 12 | Final verification | Medium |

**Dependencies:**
- Task 1 → all other tasks depend on it
- Task 2 → Task 3 depends on it
- Task 4 → Tasks 5, 10 depend on it (for pulse_opacity)
- Task 6 → Task 7 depends on it
- Tasks 1-7 form the critical path
- Tasks 8, 9, 10, 11 can be done in parallel after Task 4

**Total estimated time:** ~4-6 hours for a developer familiar with GPUI and the codebase.

**Testing strategy:**
1. Unit tests: SessionDetector pattern extraction functions
2. Integration tests: InstanceEntry polling loop with mocked PTY output
3. Manual tests: Real Claude CLI sessions with questions
4. Visual tests: Overlay positioning, pulsing animation smoothness
