# Conescope Rust Rewrite Research

## Current App Summary

Conescope is a macOS Electron app for managing multiple Claude Code CLI instances. ~40 TypeScript files, 39 IPC channels, 4 SQLite tables, 6 Zustand stores, 24 React components. Key dependencies: xterm.js, node-pty, better-sqlite3, CodeMirror, react-arborist.

---

## 1. UI Stack Comparison

### Option A: GPUI (Zed's Framework) - RECOMMENDED

| Aspect | Details |
|--------|---------|
| Rendering | GPU-accelerated via Metal (macOS) / Vulkan (Linux) / DirectX 11 (Windows), targets 120 FPS |
| Terminal | `gpui-ghostty` (git dep) embeds Ghostty VT core into GPUI. Requires Zig 0.14.1 build toolchain. Alternative: `gpui-terminal` by zortax (git dep, wraps `alacritty_terminal`). **Neither is on crates.io.** |
| Component library | `gpui-component` (60+ widgets: buttons, modals/dialogs, tables, 20+ themes) |
| Code editor | `gpui-component` includes editor with Tree-sitter highlighting + LSP support (200K lines) |
| Async | `cx.spawn()`, `cx.background_spawn()` return `Task<R>`, cancel-on-drop semantics. **Note:** GPUI has its own async runtime -- avoid mixing with tokio unless a dependency requires it. |
| Published crates | `gpui` 0.2.2 on crates.io but **effectively a stale snapshot** -- use git dep pinned to a commit. `gpui-component` 0.5.0 works from crates.io. |
| Platform | macOS + Linux + Windows (arm64/x86_64). Windows stable since Oct 2025 (DirectX 11) -- **requires git version of gpui**, crates.io 0.2.2 is macOS+Linux only. |
| Production apps | Zed, Loungy (launcher), Hummingbird (music), nohrs (file explorer), Zedis (Redis GUI), Longbridge Pro |
| Maturity | Pre-1.0 but actively developed, growing ecosystem. Sparse docs -- "read Zed source" is primary learning path. |
| Memory | ~30-40MB idle (vs Electron 150-300MB) |

**Why recommended:** Conescope is a terminal-heavy app -- GPUI has native terminal rendering proven in Zed. The `gpui-component` library provides all needed UI widgets. `gpui-ghostty` provides embeddable terminal views. Your requirements mention preferring Zed ecosystem. All three major platforms are now supported (Windows via git version).

**Risks:**
- Pre-1.0 APIs shift frequently, driven by Zed's needs not community's.
- Core deps (`gpui`, terminal crate) require git dependencies -- no stable crates.io releases.
- `gpui-ghostty` adds Zig 0.14.1 as a build dependency.
- crates.io `gpui` 0.2.2 has Metal crash bug on recent macOS -- must use git version.
- font-kit + core-text build issue reported Jan 2026 on macOS.

### Option B: Tauri v2

| Aspect | Details |
|--------|---------|
| Rendering | OS native WebView (no Chromium bundle) |
| Terminal | xterm.js via WebView (same as current Electron app) |
| Async | Native Tokio via `tauri::async_runtime` |
| Platform | macOS, Linux, Windows (all arm64/x86_64) + mobile |
| Maturity | Stable v2.10.2 (Feb 2026), production-ready, 10 minor releases since v2.0 |
| Memory | 30-50MB idle, 5-10MB installers |
| IPC | JSON-RPC protocol, typed via TauRPC |

**Pros:** Easiest migration path -- reuse React + xterm.js frontend, rewrite backend in Rust. Full platform support. Stable.

**Cons:** Hybrid JS+Rust architecture (not pure Rust). IPC serialization overhead. No native terminal widget -- still using WebView.

### Option C: Iced

| Aspect | Details |
|--------|---------|
| Rendering | wgpu (Vulkan/DX12/Metal), software fallback (tiny-skia) |
| Terminal | `iced_term` crate exists (wraps `alacritty_terminal`) -- basic but functional, tested on all platforms |
| Async | Tokio via feature flag, Elm-style subscriptions |
| Platform | macOS, Linux, Windows |
| Maturity | v0.14 (last experimental before v1.0, v1.0 not yet released) |
| Production | System76 COSMIC 1.0 desktop (shipped Dec 2025 with Pop!_OS 24.04 LTS), Kraken |

**Pros:** Pure Rust, full cross-platform, approaching v1.0, reactive rendering. COSMIC 1.0 validates production readiness.

**Cons:** `iced_term` is immature (pinned to iced 0.13.1, needs update for 0.14). Smaller ecosystem than GPUI.

### Other frameworks worth noting

- **Dioxus 0.7.3** (Jan 2026) -- React-like Rust framework. WebView mode supports xterm.js (like Tauri). Native rendering via Blitz (experimental). Hot-patching of Rust code at runtime. All platforms.
- **Slint 1.15.0** (Feb 2026) -- Stable 1.x API, DSL-based UI. No terminal widget. Better fit for structured UIs than terminal-heavy apps.
- **Floem** (pre-1.0) -- Lapce editor's framework. Has battle-tested terminal rendering in a code editor context. Sparse docs.

### Verdict

**GPUI** for maximum performance and Zed ecosystem alignment. Terminal rendering is battle-tested in Zed (though the internal terminal crate is NOT directly reusable -- use `gpui-ghostty` instead). The `gpui-component` library eliminates most UI boilerplate. Windows is now supported via git version of GPUI.

**Key tradeoff:** GPUI gives the best terminal + performance story but requires git dependencies and has a fragile ecosystem. If GPUI git dependency instability becomes a blocker: fall back to **Tauri v2.10** (easiest migration, reuse React+xterm.js) or **Iced** (pure Rust, COSMIC-validated).

---

## 2. Core / UI / Platform Architecture

```
┌──────────────────────────────────────────────────────┐
│                    GPUI Application                   │
├──────────────────────────────────────────────────────┤
│  UI Layer (GPUI Views + gpui-component)              │
│  ├── AppView          Root layout + navigation       │
│  ├── OverviewGrid     Instance tile grid             │
│  ├── InstanceTile     Mini terminal preview          │
│  ├── FocusView        Editor + terminal split        │
│  ├── TerminalView     gpui-ghostty wrapper            │
│  ├── EditorView       gpui-component editor          │
│  ├── FileTreeView     Tree widget                    │
│  ├── Modals           New instance, settings, etc.   │
│  └── NavSidebar       Instance switcher              │
├──────────────────────────────────────────────────────┤
│  State Layer (GPUI Entities)                         │
│  ├── AppState         View mode, focused instance    │
│  ├── InstanceManager  PTY lifecycle, terminal history│
│  ├── ProjectStore     Project metadata               │
│  ├── SettingsStore    Preferences, session state     │
│  └── EditorState      Open files, active tab         │
├──────────────────────────────────────────────────────┤
│  Core Layer (Pure Rust, no UI deps)                  │
│  ├── database         rusqlite + migrations          │
│  ├── pty_manager      portable-pty spawn/resize/kill │
│  ├── instance         Instance types, status machine │
│  ├── project          Project discovery, colors      │
│  ├── settings         Serialization, defaults        │
│  ├── shell_detect     Platform shell detection       │
│  └── logger           File logging with rotation     │
├──────────────────────────────────────────────────────┤
│  Platform Layer                                      │
│  ├── macos            Dock icon, traffic lights, PATH│
│  ├── linux            X11/Wayland specifics          │
│  └── windows          DirectX 11 rendering, WSL paths│
└──────────────────────────────────────────────────────┘
```

### Key architectural decisions

- **Entity-based state** (GPUI pattern): Each store is a `Model<T>` managed by GPUI's entity system. Views observe models and re-render on change. No external state library needed.
- **PTY on background thread**: `cx.background_spawn()` for PTY I/O. Terminal data flows: PTY thread -> channel -> GPUI foreground -> terminal view update.
- **Database on blocking thread**: `cx.background_spawn()` with `rusqlite` (sync). No async DB driver needed.
- **Terminal rendering**: `gpui-ghostty` (git dep) wraps Ghostty's VT core via Zig FFI and provides a GPUI `TerminalView` for GPU rendering. Requires Zig 0.14.1 build toolchain. Each instance tile and focus terminal use this. Alternative: `gpui-terminal` by zortax (wraps `alacritty_terminal` instead).
- **File tree**: Custom tree view using `gpui-component` `Tree`/`List`/`VirtualList` primitives.
- **Code editor**: `gpui-component` includes a high-performance editor widget with Tree-sitter syntax highlighting + LSP.
- **Async runtime**: Use GPUI's own async primitives (`cx.spawn()`, `cx.background_spawn()`) for all UI + IO work. Do NOT mix with tokio -- GPUI has its own runtime. Only include tokio if a dependency strictly requires it.
- **Logging**: Use `tracing` as primary + `tracing-log` bridge to capture `log` output from GPUI internals. Remove `log` as a direct dependency.

### Mapping: Electron IPC -> Rust function calls

No IPC needed. All communication is direct Rust function calls within the same process:

| Electron IPC | Rust equivalent |
|-------------|-----------------|
| `instance:create` | `InstanceManager::create_instance(&mut self, path)` |
| `instance:output` (event) | GPUI subscription on `Model<Instance>` |
| `db:projects:getAll` | `Database::get_all_projects(&self)` |
| `fs:readFile` | `std::fs::read_to_string(path)` |
| `dialog:selectDirectory` | GPUI platform file dialog |

---

## 3. Cargo Workspace Structure

```
conescope/
├── Cargo.toml                    # [workspace]
├── Cargo.lock
├── rust-toolchain.toml           # Pin toolchain
├── clippy.toml                   # Clippy config
├── .rustfmt.toml                 # Format config
├── deny.toml                     # cargo-deny config
│
├── crates/
│   ├── conescope/                # Binary crate (entry point)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # App bootstrap, GPUI init
│   │       └── app.rs            # Root view, global keybindings
│   │
│   ├── conescope-core/           # Pure logic, no UI deps
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── database.rs       # rusqlite, migrations, queries
│   │       ├── instance.rs       # Instance types, status FSM
│   │       ├── project.rs        # Project model, colors
│   │       ├── settings.rs       # Settings model, defaults
│   │       ├── shell.rs          # Platform shell detection
│   │       └── logger.rs         # File logging
│   │
│   ├── conescope-pty/            # PTY management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs        # Spawn, resize, kill
│   │       ├── env.rs            # PATH setup per platform
│   │       └── history.rs        # Terminal history buffer
│   │
│   ├── conescope-ui/             # GPUI views and components
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── overview/
│   │       │   ├── mod.rs
│   │       │   ├── grid.rs       # Overview grid layout
│   │       │   └── tile.rs       # Instance tile with mini terminal
│   │       ├── focus/
│   │       │   ├── mod.rs
│   │       │   ├── view.rs       # Split panel layout
│   │       │   ├── terminal.rs   # Full terminal wrapper
│   │       │   ├── editor.rs     # Code editor wrapper
│   │       │   ├── file_tree.rs  # File browser
│   │       │   └── tabs.rs       # Editor + terminal tabs
│   │       ├── shared/
│   │       │   ├── mod.rs
│   │       │   ├── nav_sidebar.rs
│   │       │   ├── top_bar.rs
│   │       │   └── modals.rs     # New instance, settings, confirm
│   │       └── theme.rs          # Color palette, dark/light
│   │
│   └── conescope-platform/       # Platform-specific code
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── macos.rs          # Dock, traffic lights, PATH
│           ├── linux.rs          # X11/Wayland specifics
│           └── windows.rs        # DirectX 11, WSL paths
│
├── assets/                       # Icons, fonts
│   ├── icons/
│   └── fonts/
│
├── data/                         # SQLite database (dev)
└── docs/
    └── plans/
```

### Workspace `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT"

[workspace.dependencies]
# GPUI: use git dep pinned to a tested commit -- crates.io 0.2.2 is stale and has macOS Metal crash bug
gpui = { git = "https://github.com/zed-industries/zed", rev = "<pin-to-tested-commit>" }
gpui-component = "0.5"
# Terminal: gpui-ghostty embeds Ghostty VT core. Requires Zig 0.14.1 installed.
gpui-ghostty = { git = "https://github.com/Xuanwo/gpui-ghostty" }
portable-pty = "0.9"
rusqlite = { version = "0.34", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-log = "0.2"

[workspace.lints.rust]
unsafe_code = "deny"
missing_debug_implementations = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
cargo = { level = "warn", priority = -1 }
```

### Dependency graph

```
conescope (bin)
├── conescope-ui
│   ├── conescope-core
│   ├── conescope-pty
│   ├── conescope-platform
│   ├── gpui (git)
│   ├── gpui-component (crates.io)
│   └── gpui-ghostty (git, requires Zig 0.14.1)
├── conescope-core
│   ├── rusqlite
│   ├── serde / serde_json
│   ├── uuid
│   └── anyhow
├── conescope-pty
│   ├── conescope-core (Instance types)
│   └── portable-pty
└── conescope-platform
    └── conescope-core
```

---

## 4. Strict Diagnostics Configuration

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
```

### `clippy.toml`

```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 5
type-complexity-threshold = 200
```

### `.rustfmt.toml`

```toml
edition = "2024"
style_edition = "2024"
max_width = 100
tab_spaces = 4
use_field_init_shorthand = true
use_try_shorthand = true
# NOTE: imports_granularity and group_imports are nightly-only rustfmt options.
# They also have a known non-idempotency bug when combined.
# Use rust-analyzer's import organization instead, or uncomment below for nightly rustfmt:
# unstable_features = true
# imports_granularity = "Crate"
# group_imports = "StdExternalCrate"
```

### `deny.toml` (cargo-deny)

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
unlicensed = "deny"
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "warn"
allow-git = [
    "https://github.com/zed-industries/zed",
    "https://github.com/Xuanwo/gpui-ghostty",
]
```

### CI commands

```bash
# Format check
cargo fmt --all -- --check

# Clippy with all targets
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Type check
cargo check --workspace --all-targets

# Tests
cargo test --workspace

# License/advisory audit
cargo deny check

# Build release (macOS universal)
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

# Build release (Linux)
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu

# Build release (Windows -- requires DirectX 11 SDK)
cargo build --release --target x86_64-pc-windows-msvc
```

### Pre-commit hook

```bash
#!/bin/sh
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings
```

---

## 5. Step-by-Step Migration Plan

### Phase 0: Setup

1. Initialize Cargo workspace with structure above
2. Add `rust-toolchain.toml`, `.rustfmt.toml`, `clippy.toml`, `deny.toml`
3. Install Zig 0.14.1 (required for `gpui-ghostty` build)
4. Set up CI with format + clippy + test + deny checks. **Note:** CI needs GPU headers (Metal/Vulkan) for GPUI compilation -- test on macOS runners or configure Linux with appropriate packages.
5. Verify GPUI builds on your machine: create minimal window with:
   ```rust
   gpui::Application::new().run(|cx: &mut gpui::App| {
       cx.open_window(gpui::WindowOptions::default(), |window, cx| {
           cx.new(|_cx| MyRootView {})
       }).unwrap();
   });
   ```
6. Verify `gpui-ghostty` renders a basic PTY (see `gpui-ghostty/examples/pty_terminal`)
7. Pin GPUI git dependency to the commit that works: update `rev = "..."` in workspace Cargo.toml
8. Commit skeleton

### Phase 1: Core crate -- database + models

1. Port `database.ts` to `conescope-core/database.rs`:
   - Same schema (projects, instances, questions, settings)
   - WAL mode, foreign keys on
   - All queries as methods on `Database` struct
   - Migration system (version table + sequential migrations)
2. Port type definitions: `Instance`, `Project`, `Settings`, `Question`
3. Port `shell.rs` -- platform shell detection logic
4. Port `logger.rs` -- file logging with rotation
5. Write tests for all database operations
6. Commit

### Phase 2: PTY management

1. Port `instance-manager.ts` to `conescope-pty/manager.rs`:
   - `portable-pty` for PTY spawn/resize/kill
   - Platform PATH setup (macOS Homebrew, NVM paths)
   - Two instance types: Claude (spawn shell + `claude\r`) and Terminal (plain shell)
2. Terminal history ring buffer (last 500 chunks)
3. Status FSM: starting -> working/waiting/paused -> stopped
4. Write tests for PTY lifecycle
5. Commit

### Phase 3: Minimal GPUI app with single terminal

**Note:** This phase is the highest-risk integration point. `gpui-ghostty` is a young project (Jan 2026) and may require debugging. Budget extra time.

1. Create `conescope` binary that opens a GPUI window
2. Integrate `gpui-ghostty` to render a single PTY (reference: `gpui-ghostty/examples/pty_terminal`)
3. Wire up PTY manager: spawn shell -> pipe output to terminal view
4. Handle keyboard input: terminal view -> PTY stdin
5. Handle resize: terminal view resize -> PTY resize
6. Platform: macOS traffic light position, dock icon
7. Commit

### Phase 4: State management + persistence

1. Create `Model<AppState>` -- view mode, focused instance
2. Create `Model<InstanceManager>` -- instances list, CRUD
3. Create `Model<ProjectStore>` -- projects list, CRUD
4. Create `Model<SettingsStore>` -- preferences, session state
5. Wire models to Database (background_spawn for DB ops)
6. Session restore: load instances from DB, reconnect PTYs
7. Commit

### Phase 5: Overview mode

1. Build overview grid layout (responsive 1-3 columns)
2. Build instance tile with mini `gpui-ghostty` terminal preview
3. Tile header: instance number, title (editable), status dot
4. Tile meta: project path, token count
5. Empty slot (+) button
6. Click tile -> focus instance
7. Commit

### Phase 6: Focus mode

1. Build split-panel layout (sidebar | editor | terminal)
2. Resizable dividers (mouse drag)
3. Terminal tabs (Claude + shell tabs)
4. File tree using gpui-component tree/list
5. Code editor using gpui-component editor widget
6. Editor tabs with modified indicator
7. Panel visibility toggles
8. Commit

### Phase 7: Navigation + modals

1. Top bar: title, instance controls, buttons
2. Nav sidebar: overview, instance numbers, panel toggles, stats
3. Instance popup (hover to switch)
4. New instance modal: recent projects, browse, terminal
5. Settings modal: theme, font sizes
6. Close confirm modal
7. Keyboard shortcuts (Cmd+0-9, Cmd+N, Cmd+,)
8. Commit

### Phase 8: Polish + parity

1. Questions panel
2. Error boundary equivalent
3. Token/cost tracking display
4. File type icons in file tree
5. Syntax highlighting for 40+ languages in editor
6. Session state persistence (panel sizes, open files)
7. Window state save/restore
8. Commit

### Phase 9: Testing + packaging

1. Integration tests for full app lifecycle
2. Cross-compile for macOS arm64 + x86_64
3. Create universal macOS binary (`lipo`)
4. Linux builds (x86_64, aarch64)
5. Windows build (x86_64, requires DirectX 11 -- test on Windows VM)
6. App icon and metadata
7. cargo-deny audit pass
8. Performance profiling vs Electron version
9. Final commit

### Dependency matrix

```
Phase 0 ─── Phase 1 ─── Phase 2 ─── Phase 3
                │            │           │
                └────────────┴───────────┤
                                         │
            Phase 4 ─── Phase 5 ─── Phase 6
                │           │           │
                └───────────┴───────────┤
                                        │
                    Phase 7 ─── Phase 8 ─── Phase 9
```

Phases 1-2 can partially overlap. Phases 5-6 can partially overlap. Phase 7-8 can partially overlap.

---

## Key Crate Versions (as of Feb 2026)

| Crate | Version | Source | Purpose |
|-------|---------|--------|---------|
| `gpui` | 0.2.2 (stale) / git main | **git dep required** | UI framework |
| `gpui-component` | 0.5.0 | crates.io | Widget library (60+ components). Note: v0.5.0 renamed `Modal` to `Dialog`, changed `Resizable` API. |
| `gpui-ghostty` | latest | **git dep** (github.com/Xuanwo/gpui-ghostty) | Terminal emulator for GPUI. Requires Zig 0.14.1. |
| `portable-pty` | 0.9.0 | crates.io | Cross-platform PTY (macOS/Linux/Windows) |
| `rusqlite` | 0.34.x | crates.io | SQLite (bundled) |
| `serde` | 1.x | crates.io | Serialization |
| `uuid` | 1.x | crates.io | ID generation |
| `anyhow` | 1.x | crates.io | Error handling |
| `tracing` | 0.1.x | crates.io | Structured logging |
| `tracing-log` | 0.2.x | crates.io | Bridge `log` output (from GPUI) into `tracing` |

## Terminal component: gpui-ghostty (RECOMMENDED)

`gpui-ghostty` by Xuanwo (Apache 2.0) embeds Ghostty's VT core (libghostty-vt) into GPUI. Architecture:
- `ghostty_vt_sys` -- Zig build + C ABI for Ghostty VT core (vendored at v1.2.3)
- `ghostty_vt` -- Safe Rust wrapper over C ABI
- `gpui_ghostty_terminal` -- GPUI TerminalView + input/selection/rendering

Working examples: `basic_terminal`, `pty_terminal`, `split_pty_terminal`.

**Build requirements:** Zig 0.14.1 (pinned), Ghostty vendored as git submodule.

Alternative: `gpui-terminal` by zortax (git dep, wraps `alacritty_terminal` 0.25.1). Less actively developed but avoids Zig dependency.

## Version pinning strategy

Since `gpui` and `gpui-ghostty` are git dependencies, pin to specific commits:
```toml
gpui = { git = "https://github.com/zed-industries/zed", rev = "abc123..." }
gpui-ghostty = { git = "https://github.com/Xuanwo/gpui-ghostty", rev = "def456..." }
```
Test thoroughly before bumping. Upstream breaking changes are frequent (pre-1.0).

## Known ecosystem risks

1. **GPUI is coupled to Zed** -- breaking changes driven by Zed's needs, not community's
2. **crates.io gpui 0.2.2 crashes on modern macOS** -- Metal SDK mismatch (`MTLCompilerService` error)
3. **font-kit build issue** on macOS with newer core-text (reported Jan 2026)
4. **GPUI docs are sparse** -- "read Zed source" is the primary learning path
5. **gpui-component v0.5.0 has breaking changes** from v0.3.x (renamed Modal->Dialog, new Resizable API)
6. **gpui-ghostty is 1 month old** (Jan 2026) -- may have stability issues
