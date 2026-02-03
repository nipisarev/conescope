# CLAUDE.md

Jenklaud - macOS Electron app for managing multiple Claude Code CLI instances.

## Behavior

- Be concise, sacrifice grammar for brevity
- **CRITICAL**: Update this doc on notable architecture changes
- Always use context7 MCP for library documentation
- Prefer editing existing files over creating new ones
- Keep TypeScript files under 300 lines when practical

## Commands

```bash
npm run dev          # Start dev server + Electron (concurrent)
npm run build        # Build Vite frontend
npm run build:electron  # Package with electron-builder
npx tsc --noEmit     # Type check (run before commits)
```

## Architecture

```
electron/           # Main process (Node.js)
├── main.ts         # Electron app entry, IPC handlers
├── preload.ts      # Context bridge API exposure
├── instance-manager.ts  # PTY process management
├── database.ts     # SQLite with better-sqlite3
└── logger.ts       # File logging

src/                # Renderer process (React)
├── App.tsx         # Root component, global IPC listeners
├── components/
│   ├── Overview/   # Grid view with mini xterm tiles
│   ├── Focus/      # Single instance view (editor + terminal)
│   └── shared/     # NavSidebar, TopBar, StatusBar, modals
├── stores/         # Zustand stores (synced with SQLite)
├── hooks/          # useKeyboardShortcuts
└── types/          # TypeScript definitions

data/jenklaud.db    # SQLite database (WAL mode)
logs/               # Runtime logs
```

## Key Patterns

### IPC Communication
- Preload exposes `window.electronAPI` via contextBridge
- Event listeners return cleanup functions: `onInstanceOutput(() => {}) => () => void`
- All DB operations go through IPC handlers in main.ts

### State Management
- Zustand stores in `src/stores/` sync with SQLite
- Instance terminal history stored in memory (not DB)
- Global IPC listeners in App.tsx capture all PTY output

### Terminal Rendering
- Focus view: Full xterm.js with FitAddon
- Overview tiles: Mini xterm.js instances per tile
- PTY spawns shell then runs `claude` command

## Database Tables

| Table | Purpose |
|-------|---------|
| projects | Stored project paths with colors |
| instances | Claude Code sessions with status |
| questions | Pending questions from instances |
| settings | Key-value app settings |

## Gotchas

- `process.env.HOME` unavailable in renderer (use regex for path shortening)
- xterm `fit()` must be delayed after `open()` to avoid dimension errors
- IPC listeners must be cleaned up to prevent memory leaks
- Main window destroyed check required before `webContents.send()`
