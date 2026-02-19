# Session Event Detection & Question Overlay System

## Problem

Conescope manages multiple Claude Code CLI instances via PTY. Users need visibility into which instances are asking questions, waiting for input, or finished — without switching to each instance individually.

## Goals

1. Detect when Claude CLI asks the user a question (AskUserQuestion), needs permission, or is waiting for input
2. Show detected questions as interactive overlays on instance tiles in overview mode
3. Allow users to answer questions directly from the overview grid
4. Show animated status indicators (pulsing dots) per instance state
5. Play macOS system sounds on state transitions requiring attention
6. Show session status dots + labels on sidebar (ActivityBar) instance icons
7. In focus mode, show blinking notification dot on sidebar toggle when another instance has a question

## Detection Architecture

### State Machine

Per-instance states:
- **Working** (green) — PTY actively producing output
- **Question** (red, pulsing) — Claude asked user a question with options
- **Waiting** (yellow, pulsing) — waiting for user input (e.g., ">" prompt after task completion)
- **Finished** (yellow, steady) — session completed
- **Stopped** (gray) — process exited

### Two-Phase Detection

**Phase 1: StreamWatcher (raw byte pre-filter)**
- Runs on every PTY output batch in `start_output_polling()`
- Strips ANSI escape sequences from raw bytes
- Scans for trigger patterns: `[y/N]`, `[Y/n]`, numbered lists (1. 2. 3.), status bar change indicators
- Sets `trigger_pending` flag when pattern matched
- Cheap operation, no screen buffer access needed

**Phase 2: ScreenAnalyzer (screen buffer classification)**
- Fires when PTY goes idle for >=200ms AND `trigger_pending` is true
- Takes snapshot of alacritty_terminal Term grid (the existing Terminal entity)
- Multi-signal classification using weighted scoring:

| Signal | Source | Weight | Notes |
|--------|--------|--------|-------|
| Status bar: "Enter to select" | Last 2 lines of grid | High | Question with options |
| Status bar: "esc to interrupt" | Last 2 lines of grid | High | **Negative** — Working state |
| `[y/N]` or `[Y/n]` pattern | Screen body | High | Yes/No prompt |
| Numbered option list (1. 2. 3.) | Screen body | Medium | Multi-choice question |
| PTY idle > 200ms | Timer | Medium | Necessary but not sufficient |
| Spinner animation absent + idle | Screen body + timer | Medium | No animation + no output |

Note: The `>` input prompt is NOT a useful signal — it's always visible during Claude's work.

**Why idle detection is reliable:** Claude Code's spinner animation runs continuously during API calls, keeping the PTY active. PTY truly goes idle only when Claude stops and waits for user input. This makes the 200ms threshold reliable — not confused by network delays.

**Empirical logging:** In debug builds, every Phase 2 snapshot gets logged to `~/.conescope/screen_dumps/` with timestamp and classification result, for iterating pattern matching against real Claude output.

### Diagram

```
PTY bytes
    |
    v
start_output_polling()
    |
    |---> TerminalView.queue_output_bytes()   (existing: render)
    |---> InstanceEntry.history.push()         (existing: history)
    |
    +---> StreamWatcher.scan(stripped_bytes)    (NEW: Phase 1)
              |
              | trigger_pending = true
              | reset debounce timer
              |
              v (200ms idle, trigger_pending)
         ScreenAnalyzer.classify(term_grid)   (NEW: Phase 2)
              |
              |-- signals score > threshold
              |       |
              |       v
              |   SessionEvent::Question { text, choices, snapshot }
              |   or SessionEvent::WaitingForInput
              |   or SessionEvent::Finished
              |
              +-- signals score < threshold -> stay Working
```

## Data Model

```rust
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SessionStatus {
    Working,
    Question,
    Waiting,
    Finished,
    Stopped,
}
```

**Persistence:** In-memory only, stored on `InstanceEntry`. Transient — cleared when answered or session ends. No DB writes.

**New fields on InstanceEntry:**

```rust
pub session_event: Option<SessionEvent>,
pub session_status: SessionStatus,
// internal:
session_detector: SessionDetector,
trigger_pending: bool,
last_output_time: Instant,
```

## UI Design

### Question Overlay (on overview tile)

```
+--------------- Instance 2 ------------------+
| * ~/project-b  main  +3 -1         [x]     |  <- red pulsing dot
+---------------------------------------------+
|                                              |
|  +-------------------------------------+    |
|  |  Which database should we use?       |    |
|  |                                      |    |
|  |  +------------------------------+   |    |
|  |  | 1. PostgreSQL                 |   |    |  <- clickable
|  |  |    Best for complex queries   |   |    |
|  |  +------------------------------+   |    |
|  |  +------------------------------+   |    |
|  |  | 2. SQLite                     |   |    |  <- clickable
|  |  |    Simple, embedded           |   |    |
|  |  +------------------------------+   |    |
|  |  +------------------------------+   |    |
|  |  | 3. MongoDB                    |   |    |  <- clickable
|  |  |    Document store             |   |    |
|  |  +------------------------------+   |    |
|  |                                      |    |
|  |  [More context ->]                   |    |  <- opens focus view
|  +-------------------------------------+    |
|                                              |
|  .... terminal preview (dimmed) ..........  |
+----------------------------------------------+
```

### Waiting/Finished Overlay (smaller)

```
+--------------- Instance 3 ------------------+
| * ~/project-c  main                 [x]     |  <- yellow pulsing dot
+---------------------------------------------+
|                                              |
|          +------------------+               |
|          |  Waiting...      |               |  <- small centered
|          +------------------+               |
|                                              |
|  .... terminal preview (dimmed) ..........  |
+----------------------------------------------+
```

### Answer Flow

1. User clicks option on overlay -> GPUI action `AnswerQuestion { instance_id, choice_index }`
2. Action handler writes the choice number + Enter to `stdin_tx` channel (-> PTY master fd)
3. Overlay auto-dismisses, `session_event` cleared
4. PTY output resumes -> status returns to Working (green)
5. "More context ->" button triggers `FocusInstance { instance_id }` action (switches to focus view)

### Pulsing Animation

- Single shared `PulseTimer` GPUI entity, ticks at ~30fps
- Computes: `opacity = 0.4 + 0.6 * ((sin(t * 2pi / 1.5) + 1.0) / 2.0)`
- 1.5 second full cycle (0.4 -> 1.0 -> 0.4)
- All pulsing status dots observe this entity and render with its current opacity
- Only Question (red) and Waiting (yellow) states pulse; Working (green) and Finished (yellow) are steady

### Status Colors

| State | Color | Animation |
|-------|-------|-----------|
| Working | Green (#4ade80) | Steady |
| Question | Red (#f87171) | Pulse |
| Waiting | Yellow (#facc15) | Pulse |
| Finished | Yellow (#facc15) | Steady |
| Stopped | Gray (#9ca3af) | Steady |

## Sound Notifications

Play macOS system sounds via `NSSound` when an instance transitions to a state requiring attention.

**Trigger:** `SessionStatus` changes on any instance. Only play sound on transitions TO attention states, not on every re-detection.

| Transition | Sound | Notes |
|-----------|-------|-------|
| Any → Question | `NSSound(named: "Purr")` | Gentle but noticeable |
| Any → Waiting | `NSSound(named: "Tink")` | Softer, less urgent |
| Any → Finished | `NSSound(named: "Pop")` | Brief completion chime |
| Any → Stopped (unexpected) | `NSSound(named: "Basso")` | Error/alert tone |

**Debounce:** If multiple instances transition within 500ms, play only one sound (highest priority: Question > Stopped > Waiting > Finished).

**Implementation:** `conescope-platform` crate already handles macOS-specific code. Add `play_system_sound(name: &str)` there using `objc2` / `NSSound::play()`. Called from `InstanceEntry` when `session_status` changes.

**User setting:** `sound_notifications: bool` in `SettingsJson` (default: true). Respectable — can be disabled.

## Sidebar Status Indicators (ActivityBar)

Each instance icon in the ActivityBar gets a small status dot + label at bottom-right corner.

```
ActivityBar (vertical sidebar):

┌────┐
│ 1  │   ← green dot, no label (Working = default, quiet)
│  ● │
└────┘
┌────┐
│ 2  │   ← red pulsing dot + "question" label
│ ●Q │
└────┘
┌────┐
│ 3  │   ← yellow pulsing dot + "waiting" label
│ ●W │
└────┘
┌────┐
│ 4  │   ← gray dot, no label (Stopped)
│  ● │
└────┘
```

**Dot spec:**
- 6px circle at bottom-right of icon
- Same colors as overview tile dots (green/red/yellow/gray)
- Same pulsing animation (shared PulseTimer) for Question/Waiting
- Short text label: "Q" for question, "W" for waiting, "F" for finished — only shown for non-Working states
- No question overlay in sidebar — just the dot + label. User clicks icon to switch to that instance.

**Implementation:** Modify `ActivityBar` render to read cached `session_status` per instance and draw the dot overlay.

## Focus Mode Notification Dot

When the user is in focus mode (viewing a single instance), they can't see other instances. If another instance enters Question state, show a notification dot on the sidebar toggle button.

```
┌──────────────────────────────────────────────────┐
│ [≡ ●] Instance 1 - ~/project-a            [×]   │  ← red dot on sidebar toggle
├──────────────────────────────────────────────────│
│                                                   │
│  ... terminal content ...                         │
│                                                   │
└──────────────────────────────────────────────────┘
      ^
      red pulsing dot on the sidebar toggle button [≡]
      indicates another instance has a question
```

**Trigger:** Any non-focused instance has `SessionStatus::Question`. Only Question state triggers this — Waiting/Finished do not.

**Behavior:**
- Red pulsing dot appears on the sidebar toggle button (or activity bar collapse/expand button)
- Same pulsing animation as other dots (shared PulseTimer)
- Dot disappears when: user opens sidebar, user switches to the questioning instance, or the question gets answered
- Sound plays when dot first appears (same Question sound)

**Implementation:** `AppView` or `TopBar` checks all instances for Question status. If any non-focused instance has Question, render dot on sidebar toggle.

## New Files

```
crates/conescope-ui/src/
  state/
    session_detector.rs    # StreamWatcher + ScreenAnalyzer logic
    pulse_timer.rs         # Shared animation timer entity
  views/
    question_overlay.rs    # GPUI overlay view for tiles

crates/conescope-platform/src/
    sound.rs               # macOS NSSound wrapper (play_system_sound)
```

## Modified Files

```
crates/conescope-ui/src/
  state/
    instance_entry.rs      # Add session fields, detection integration in polling loop
    app_state.rs           # Hold PulseTimer entity, sound debounce logic
  views/
    overview_grid.rs       # Render overlays on tiles, read session_event
    activity_bar.rs        # Status dot + label on each instance icon
    top_bar.rs             # Focus mode notification dot on sidebar toggle
    app_view.rs            # Focus mode notification dot logic

crates/conescope-core/src/
    settings.rs            # Add sound_notifications bool to SettingsJson
```

## Open Questions

1. **Exact Claude Code patterns:** Need empirical data from real sessions to finalize ScreenAnalyzer patterns. The debug logging system will capture this.
2. **Permission prompts:** Should tool permission prompts ("Allow this bash command?") be treated identically to AskUserQuestion? Likely yes — they also need user action.
3. **Multiple questions:** If Claude asks another question before the first is answered (unlikely), should we queue them? Initial approach: replace — latest question wins.
4. **Non-Claude instances:** For plain terminal instances, detection still works but patterns differ. May need configurable pattern sets per instance type.
