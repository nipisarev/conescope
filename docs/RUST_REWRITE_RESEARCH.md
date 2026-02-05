# Conescope Rust Rewrite Research

## Current Codebase Scope

| Metric | Count |
|--------|-------|
| Total source lines | ~5,600 |
| IPC handlers | 33 |
| React components | 20 |
| Zustand stores | 6 |
| Native modules | 2 (node-pty, better-sqlite3) |
| npm dependencies | 26 |

## Rust Alternatives for Key Components

### 1. Terminal Emulator (xterm.js replacement)

| Library | Approach | Maturity |
|---------|----------|----------|
| [Alacritty](https://github.com/alacritty/alacritty) (alacritty_terminal crate) | GPU-accelerated VTE, extractable core | High - battle-tested |
| [par-term-emu-core-rust](https://github.com/paulrobello/par-term-emu-core-rust) | VT100-VT520 emulation library with PTY | Medium - newer |
| [Ratatui](https://ratatui.rs/) | TUI framework with terminal widgets | High - but for *in-terminal* UIs, not embedding in a GUI |
| [GPUI + custom terminal](https://dev.to/zhiwei_ma_0fc08a668c1eb51/building-a-gpu-accelerated-terminal-emulator-with-rust-and-gpui-4103) | Build on Zed's framework | Medium - people have done it |

Most realistic path: extract `alacritty_terminal` as a VTE backend and render in GUI framework. Zed does exactly this for its integrated terminal.

### 2. Code Editor (CodeMirror replacement)

| Library | Features | Maturity |
|---------|----------|----------|
| [GPUI Component Editor](https://longbridge.github.io/gpui-component/docs/components/editor) | 200K+ lines, tree-sitter highlighting, LSP (diagnostics, completion, hover), line numbers, search | High - production-ready |
| [egui_code_editor](https://crates.io/crates/egui_code_editor) | Basic syntax highlighting, line numbers, auto-complete | Medium - simpler |
| [tree-sitter-highlight](https://crates.io/crates/tree-sitter-highlight) + custom rendering | Roll your own with tree-sitter parsing | High (tree-sitter), but DIY |
| [tree-house](https://github.com/helix-editor/tree-house) (from Helix editor) | Robust highlighter + query iterator | High - powers Helix |

GPUI Component editor is closest to CodeMirror -- syntax highlighting, LSP integration, large file handling.

### 3. File Tree

| Library | Features | Maturity |
|---------|----------|----------|
| [GPUI Component Tree](https://longbridge.github.io/gpui-component/) | Tree widget with folder/file icons, expand/collapse, depth-based indentation, sidebar layout | High |
| [GPUI Component Sidebar](https://longbridge.github.io/gpui-component/) | File explorer sidebar with nested menu items | High |
| [egui TreeView](https://areweguiyet.com/) | Basic tree for egui | Medium |
| [Ratatui Tree widget](https://ratatui.rs/) | TUI-based tree | High (terminal-only) |

GPUI Component provides both a tree widget and file explorer sidebar out of the box.

## Stack Migration Map

| Current (Electron/React) | Rust Replacement |
|--------------------------|-----------------|
| xterm.js | alacritty_terminal + GPUI rendering |
| CodeMirror | GPUI Component Editor (tree-sitter + LSP) |
| react-arborist (FileTree) | GPUI Component Tree/Sidebar |
| React + CSS | GPUI (GPU-accelerated, Rust-native) |
| Zustand stores | Rust state management (GPUI entities) |
| Electron IPC | Gone -- single process |
| better-sqlite3 | rusqlite |
| node-pty | portable-pty |

## Rewrite Approaches

| Approach | Effort | What Changes |
|----------|--------|-------------|
| **Tauri** (hybrid) | Medium | Rewrite `electron/` (~900 lines) to Rust backend. Keep React frontend nearly intact. |
| **Full Rust with GPUI** | High | Rewrite everything. All components have Rust alternatives via GPUI ecosystem. |
| **Full Rust native (egui/iced)** | Very High | Weaker component ecosystem. Would need to build editor and terminal from scratch. |

## Risks and Considerations

- **GPUI is pre-1.0** and tightly coupled to Zed. API can change.
- Documentation improving but not as rich as React/Electron ecosystem.
- Expect more time fighting the framework vs building features initially.
- Rust ecosystem for GUI is 2-3 years behind JS/Electron in stability and docs.
- Upside: single binary, no Chromium overhead, better performance, no IPC layer.

## References

- [GPUI Component Library](https://longbridge.github.io/gpui-component/)
- [GPUI Framework](https://www.gpui.rs/)
- [Alacritty Terminal](https://github.com/alacritty/alacritty)
- [Ratatui](https://ratatui.rs/)
- [egui_code_editor](https://crates.io/crates/egui_code_editor)
- [tree-sitter-highlight](https://crates.io/crates/tree-sitter-highlight)
- [tree-house (Helix)](https://github.com/helix-editor/tree-house)
- [par-term-emu-core-rust](https://github.com/paulrobello/par-term-emu-core-rust)
- [Building Terminal with GPUI](https://dev.to/zhiwei_ma_0fc08a668c1eb51/building-a-gpu-accelerated-terminal-emulator-with-rust-and-gpui-4103)
- [2025 Survey of Rust GUI Libraries](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
