# Jenklaud V2 Design Document

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform Jenklaud into a fully functional Claude Code command center with integrated editor, file explorer, and persistent storage.

**Architecture:** Electron + React + TypeScript with CodeMirror 6 for editing, react-arborist for file tree, xterm.js for terminal, and SQLite for persistent storage.

**Tech Stack:** CodeMirror 6, react-arborist, xterm.js, better-sqlite3, Zustand, node-pty

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        JENKLAUD                             │
├─────────────────────────────────────────────────────────────┤
│  UI Layer (React + TypeScript)                              │
│  ├── CodeMirror 6 (editor)                                  │
│  ├── react-arborist (file tree)                             │
│  ├── xterm.js (terminal)                                    │
│  └── Custom components (tiles, status bar, etc.)            │
├─────────────────────────────────────────────────────────────┤
│  State Layer (Zustand)                                      │
│  ├── appStore (UI state, view mode)                         │
│  ├── instanceStore (Claude instances)                       │
│  ├── projectStore (projects metadata)                       │
│  └── settingsStore (user preferences)                       │
├─────────────────────────────────────────────────────────────┤
│  Storage Layer (SQLite via better-sqlite3)                  │
│  ├── projects table                                         │
│  ├── instances table (history)                              │
│  ├── settings table                                         │
│  └── questions table (pending/answered)                     │
├─────────────────────────────────────────────────────────────┤
│  Electron Main Process                                      │
│  ├── Instance Manager (node-pty)                            │
│  ├── Database Service                                       │
│  ├── File System Service                                    │
│  └── IPC Handlers                                           │
└─────────────────────────────────────────────────────────────┘
```

**Key decisions:**
- SQLite for persistent storage (projects, settings, question history)
- Zustand syncs with SQLite on changes
- All file/DB operations go through Electron main process via IPC

---

## 2. Navigation Sidebar (Persistent)

The left navigation sidebar appears on ALL views (Overview and Focus modes).

```
┌──┐
│▣ │ ← Overview (home) - highlighted when active
├──┤
│1 │ ← Instance 1 (colored dot for status)
├──┤
│2 │ ← Instance 2
├──┤
│3 │ ← Instance 3
│  │
│  │ (spacer - grows to fill)
│  │
├──┤
│⚙ │ ← Settings (pinned to bottom)
└──┘
```

**Specifications:**
- Fixed width: 48px
- All buttons: 40x40px (same size)
- Overview button (▣): Always at top, returns to Overview mode
- Instance buttons: Show number + colored status dot
- Settings button (⚙): Pinned to bottom
- Active state: Highlighted background
- Hover tooltip: "#1 Feature Auth" + "~/Projects/alpha"

---

## 3. Overview Mode UI

```
┌─────────────────────────────────────────────────────────────────────────┐
│ ● ● ●        ◉ Jenklaud                         [+ New] [Questions]    │
├──┬──────────────────────────────────────────────────────────────────────┤
│  │                                                                      │
│▣ │  ┌─────────────────────┐  ┌─────────────────────┐                   │
│  │  │ ▌#1 Feature Auth ✎  │  │ ▌#2 api-service ✎   │                   │
│──│  │   ~/Projects/alpha  │  │   ~/Projects/api    │                   │
│1 │  │                     │  │                     │                   │
│  │  │  $ claude           │  │  $ claude           │                   │
│──│  │  > Working on the   │  │  ? How should I     │                   │
│2 │  │    authentication   │  │    handle this...   │                   │
│  │  │    feature...       │  │                     │                   │
│──│  ├─────────────────────┤  ├─────────────────────┤                   │
│3 │  │ ● working     1.2k  │  │ ● waiting      450  │                   │
│  │  └─────────────────────┘  └─────────────────────┘                   │
│  │                                                                      │
│──│──────────────────────────────────────────────────────────────────────│
│⚙ │ 3 instances │ 3.75k tokens │ $0.12                                   │
└──┴──────────────────────────────────────────────────────────────────────┘
```

### Tile Design

```
┌─────────────────────┐
│ ▌#1 Feature Auth ✎ │  ← Colored number + title + pencil (click to edit)
│   ~/Projects/alpha  │  ← Smaller, path shortened from home
│                     │
│  $ claude           │  ← Live terminal output preview
│  > Working on the   │
│    authentication   │
│    feature...       │
├─────────────────────┤
│ ● working     1.2k  │  ← Status indicator + token count
└─────────────────────┘
```

**Title behavior:**
- Default: Project folder name
- Editable: Click pencil icon to edit inline
- Format: `#N Title ✎` (colored by project)

**Path display:**
- Shortened from home: `/Users/nikopi/Projects/alpha` → `~/Projects/alpha`

**Edit mode:**
- Inline input field on pencil click
- Enter/blur to save, Esc to cancel

---

## 4. Focus Mode UI

```
┌─────────────────────────────────────────────────────────────────────────┐
│ ● ● ●        ◉ #1 Feature Auth                  [+ New] [Questions]    │
├──┬──────────────────────────────────────────────────────────────────────┤
│  │        │                                                             │
│▣ │ FILES  │  ┌─ main.ts ─┬─ utils.ts ─┬─ config.json ─┐                │
│  │        │  │                                         │                │
│──│ ▼ src  │  │  1  import { App } from './app'        │                │
│1 │   app.ts  │  2                                      │                │
│  │   main.ts │  3  const config = {                   │                │
│──│ ▼ tests│  │  4    port: 3000,                      │                │
│2 │   app.test│  5    host: 'localhost'                │                │
│  │ config.json│ 6  }                                  │                │
│──│ package.json│                                       │                │
│3 │        │  └─────────────────────────────────────────┘                │
│  │        ├─────────────────────────────────────────────────────────────│
│  │        │ TERMINAL                                                    │
│  │        │ $ claude                                                    │
│  │        │ > I'll help you implement the new feature. Let me start    │
│  │        │   by reading the existing code...                          │
│  │        │ █                                                           │
│──│────────┴─────────────────────────────────────────────────────────────│
│⚙ │ ● working │ main.ts:42 │ 1.2k tokens │ $0.04                         │
└──┴──────────────────────────────────────────────────────────────────────┘
```

**Layout (VSCode-style):**
- Left panel: File tree (react-arborist) - collapsible
- Top right: Editor with tabs (CodeMirror 6)
- Bottom right: Terminal (xterm.js) - resizable divider

**Editor features:**
- Tabs for open files
- Syntax highlighting via CodeMirror 6
- Auto-reload when Claude modifies files

**Terminal features:**
- Full interactive terminal (node-pty)
- Claude CLI runs here
- Input/output captured for Overview tiles

**Bottom status bar:**
- Instance status indicator
- Current file + cursor position
- Token count + cost

---

## 5. Questions Panel (Slide-out)

Toggles via "Questions" button in top bar.

```
┌──────────────────────────────────────────────────────┬─────────────┐
│                                                      │ QUESTIONS   │
│            Main content area                         │             │
│                                                      │ ▌#2 api     │
│                                                      │ ? How to... │
│                                                      │ ⏱ 3m ago   │
│                                                      │ [Answer]    │
│                                                      │             │
│                                                      │ ▌#3 frontend│
│                                                      │ ? Should I..│
│                                                      │ ⏱ 1m ago   │
│                                                      │ [Answer]    │
└──────────────────────────────────────────────────────┴─────────────┘
```

---

## 6. Frameless Window (macOS)

**Top bar structure:**
```
┌─────────────────────────────────────────────────────────────────────────┐
│ ● ● ●        ◉ Jenklaud                         [+ New] [Questions]    │
│ traffic     (drag region)                        action buttons        │
│ lights                                                                  │
└─────────────────────────────────────────────────────────────────────────┘
```

**Electron config:**
```javascript
new BrowserWindow({
  titleBarStyle: 'hiddenInset',  // or 'hidden' for full control
  trafficLightPosition: { x: 12, y: 12 },
  // ...
})
```

---

## 7. SQLite Database Schema

```sql
-- Projects: stored project configurations
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    path TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    color TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);

-- Instances: running/historical Claude sessions
CREATE TABLE instances (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    title TEXT,                    -- User-editable title
    status TEXT NOT NULL DEFAULT 'starting',
    instance_number INTEGER,       -- For sidebar navigation (1, 2, 3...)
    tokens_used INTEGER DEFAULT 0,
    cost_estimate REAL DEFAULT 0,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Questions: pending and answered questions from Claude
CREATE TABLE questions (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL,
    question_text TEXT NOT NULL,
    context TEXT,
    asked_at TEXT NOT NULL,
    answered_at TEXT,
    answer TEXT,
    snoozed_until TEXT,
    FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);

-- Settings: user preferences (key-value)
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Default settings
INSERT INTO settings (key, value) VALUES
    ('theme', 'dark'),
    ('questions_panel_visible', 'true'),
    ('editor_font_size', '13'),
    ('terminal_font_size', '13');
```

**Sync strategy:**
- On app start: Load from SQLite → Zustand stores
- On changes: Zustand → SQLite (debounced writes)
- Instance terminal output: Memory only (not persisted)

---

## 8. Libraries & Dependencies

| Component | Library | Version | Purpose |
|-----------|---------|---------|---------|
| **Editor** | `@uiw/react-codemirror` | ^4.x | Code editing with tabs |
| **File Tree** | `react-arborist` | ^3.x | VSCode-style file explorer |
| **Terminal** | `xterm.js` + `@xterm/addon-fit` | ^5.x | Terminal emulation |
| **Database** | `better-sqlite3` | ^11.x | SQLite storage |
| **State** | `zustand` | ^5.x | Already using |
| **PTY** | `node-pty` | ^1.x | Already using |

**Installation:**
```bash
npm install @uiw/react-codemirror @codemirror/lang-javascript @codemirror/lang-typescript @codemirror/lang-json @codemirror/lang-markdown @codemirror/lang-css @codemirror/lang-html
npm install react-arborist
npm install better-sqlite3
npm install -D @types/better-sqlite3
npx electron-rebuild -f -w better-sqlite3
```

---

## 9. Summary of Changes from V1

1. **Left Navigation Sidebar** - Persistent on all views with Overview, instance numbers, Settings
2. **Frameless Titlebar** - Traffic lights + drag region + [+ New] [Questions]
3. **Bottom Status Bar** - Aggregate stats, replaces top bar info
4. **Renamed Inbox → Questions** - Toggle via top bar button
5. **Overview Tiles** - Live terminal preview, status bar at bottom, editable title with #N
6. **Focus Mode** - File tree (left), Editor with tabs (top), Terminal (bottom)
7. **SQLite Storage** - Persistent projects, instances, questions, settings
8. **Instance Titles** - User-editable, defaults to project folder name

---

## 10. Implementation Order

1. **Phase 1: Foundation**
   - Install new dependencies
   - Set up SQLite database service
   - Create database schema and migrations

2. **Phase 2: Navigation & Layout**
   - Implement left navigation sidebar
   - Update to frameless window
   - Add bottom status bar
   - Remove old top bar stats

3. **Phase 3: Overview Mode**
   - Update tile design with new layout
   - Add editable titles
   - Show live terminal preview in tiles
   - Rename Inbox to Questions

4. **Phase 4: Focus Mode**
   - Integrate react-arborist file tree
   - Integrate CodeMirror 6 editor with tabs
   - Wire up terminal to editor (file watching)
   - Add resizable panels

5. **Phase 5: Polish**
   - Questions panel slide-out
   - Settings panel
   - Keyboard shortcuts
   - Error handling & edge cases
