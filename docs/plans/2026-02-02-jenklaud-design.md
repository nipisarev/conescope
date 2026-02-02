# Jenklaud Design Document

## Overview

Jenklaud is a native macOS Electron app that serves as a unified command center for managing multiple Claude Code CLI instances across different projects. It replaces the workflow of juggling multiple editor windows, providing clear context separation and a "director's view" of all running agents.

## Problem Statement

When working with multiple Claude Code instances across different projects:
- Easy to confuse similar editor windows (VSCode/Zed)
- Answering questions in the wrong project breaks chat context
- No unified view of what all agents are doing
- Constant context-switching wastes time

## Solution

A single interface where the user acts as a "director" receiving questions from Claude "managers" rather than running between "factories."

---

## Core Architecture

### Components

1. **Jenklaud App (Electron)**
   - Main process: runs a local HTTP server, manages spawned CLI processes
   - Renderer process: React-based UI with overview and focus modes

2. **Jenklaud Skill (installed in Claude Code)**
   - Uses Claude Code hooks for event interception
   - MCP server for bidirectional communication via SSE + HTTP POST
   - Reports metrics/status, receives control signals

3. **Instance Manager**
   - Spawns and tracks Claude Code CLI processes via PTY (node-pty)
   - Each instance runs in a pseudo-terminal for full terminal capture
   - Captures stdout/stderr for display in overview mode

### Data Flow

```
Claude CLI instance → Jenklaud Skill → SSE to localhost:PORT → App receives updates
                                                              ↓
User responds in App → HTTP POST → Skill receives command → Claude continues
```

### Communication Protocol

- **Skill → App:** SSE stream (metrics, status, questions)
- **App → Skill:** HTTP POST (pause, finish, kill, respond to question)

SSE chosen over WebSocket because:
- MCP-native (MCP's HTTP transport uses SSE)
- Simpler, no connection management
- Auto-reconnects on connection drop
- Control signals are infrequent, don't need true bidirectional

---

## User Interface

### Two Modes

1. **Overview Mode** - Bird's eye view of all running instances
2. **Focus Mode** - Deep dive into one project with full editor + terminal

### Overview Mode Layout

The screen dynamically splits based on active instances:
- 1 instance: full screen
- 2 instances: split vertical (50/50)
- 3-4 instances: 2x2 grid
- 5-6 instances: 2x3 or 3x2 grid
- 7+ instances: scrollable grid

**Empty slot [+] button:**
- When grid has empty space (e.g., 3 instances in 2x2 grid), show a [+] button in the empty slot
- Dashed border, large [+] icon, "New Instance" label
- Click opens project selector
- Provides quick way to add instances without using top bar

**Overview mockup:**

```
┌───────────────────────────────────────────────────────────────────────────────┐
│  ◉ Jenklaud                  3 instances │ 127k tokens │ $1.42   [+New] [⚙]   │
├───────────────────────────────────────────────────────────────────────────────┤
│                                                            │                  │
│  ┌─ 🔴 ───────────────────┐  ┌─ 🔵 ───────────────────┐    │  INBOX (3)  [↗]  │
│  │ Backend API            │  │ Mobile App             │    │                  │
│  │ ~/Projects/api-server  │  │ ~/Projects/mobile      │    │  ┌────────────┐  │
│  │ ● working              │  │ ⏳ waiting             │    │  │ 🔵 Mobile  │  │
│  ├────────────────────────┤  ├────────────────────────┤    │  │ Which auth │  │
│  │ > Running tests...     │  │ > Which authentication │    │  │ flow to... │  │
│  │ > ✓ user.test passed   │  │ > flow should I use?   │    │  │ ⏱ 3m ago  │  │
│  │ > ...                  │  │ > _                    │    │  │ [✓] [✗] [⏸]│  │
│  ├────────────────────────┤  ├────────────────────────┤    │  └────────────┘  │
│  │ 45k │ $0.52 │ 23 min   │  │ 32k │ $0.38 │ 15 min   │    │                  │
│  └────────────────────────┘  └────────────────────────┘    │  ┌────────────┐  │
│                                                            │  │ 🟢 Admin   │  │
│  ┌─ 🟢 ───────────────────┐  ┌┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┐    │  │ Add cache? │  │
│  │ Admin Dashboard        │  ┆                        ┆    │  │ ⏱ 8m 🟡   │  │
│  │ ~/Projects/admin       │  ┆                        ┆    │  │ [✓] [✗] [⏸]│  │
│  │ ⏳ waiting             │  ┆        [ + ]           ┆    │  └────────────┘  │
│  ├────────────────────────┤  ┆                        ┆    │                  │
│  │ > Should I add caching │  ┆    New Instance        ┆    │  [View Queue →]  │
│  │ > for API responses?   │  ┆                        ┆    │                  │
│  ├────────────────────────┤  └┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┘    │                  │
│  │ 50k │ $0.52 │ 31 min   │                                │                  │
│  └────────────────────────┘                                │                  │
└───────────────────────────────────────────────────────────────────────────────┘
```

**Legend:**
- 🔴🔵🟢 = Project colors (tile border)
- ● working / ⏳ waiting = Instance status
- [✓] approve, [✗] reject, [⏸] snooze = Quick actions (stay in Overview)
- 🟡 = Elevated urgency (time-based)
- [↗] = Expand to Questions Queue
- Click question text → Opens Focus mode for that project

**Each instance tile shows:**
- Header: Project name, directory path, status indicator (working/waiting/idle)
- **Project color**: Tile border or header accent uses the project's assigned color
- Mini terminal: Last ~10-15 lines of CLI output, live updating
- Stats bar: Tokens spent, cost estimate, session duration
- Question badge: If waiting for input, shows truncated question with escalation color

**Inbox panel (right side or toggleable):**
- Stacked list of all pending questions across instances
- Each item shows **project color dot** for quick identification
- Sorted by wait time (oldest/most urgent at top)
- **Click question → opens Focus mode** for that project (full context)
- Quick actions (without leaving Overview): Approve / Reject / Snooze
- Button to open full **Questions Queue** (sidebar/modal)

**Questions Queue (sidebar or modal):**
- Expanded view of all pending questions
- More room to view question details and context
- Each question clearly shows:
  - Project name + project color indicator
  - Full question text (expandable for more context)
  - Wait time / urgency level
- Can respond inline without leaving Overview
- Click project name to jump to Focus mode

**Top bar:**
- Total instances running
- Aggregate token spend / cost
- "New Instance" button
- Settings

**Time-based urgency escalation:**
- Questions start normal (green)
- Escalate visually the longer they wait (yellow → red)

### Focus Mode Layout

```
┌─────────────────────────────────────────────────────────┐
│ [← Back to Overview]    Project Name    [Pause][Finish] │
├─────────────────────┬───────────────────────────────────┤
│                     │                                   │
│   File Explorer     │         Editor Pane               │
│   (tree view)       │      (Monaco Editor)              │
│                     │                                   │
├─────────────────────┴───────────────────────────────────┤
│                                                         │
│              Terminal with Claude Agent                 │
│                   (xterm.js + PTY)                      │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ Stats: 45k tokens │ $0.23 │ 12 min │ [Git Changes: 3]   │
└─────────────────────────────────────────────────────────┘
```

Panes are resizable by dragging borders.

### Instance Controls

- **Pause** - Agent finishes current tool call, then waits. Can resume later.
- **Finish** - Agent completes current task, writes summary of done/remaining, exits cleanly.
- **Kill** - Hard stop immediately (with confirmation).
- **Resume** - Continue a paused instance.

### Progressive Disclosure for Questions

1. Start with minimal context (question + project name)
2. User can request more context on demand
3. "Expand" shows recent conversation/activity
4. "Focus" jumps into full Focus mode for that project

---

## Metrics & Stats

**Per-instance tracking:**
- Tokens spent (input/output, running total)
- Cost estimate (based on current API pricing)
- Session duration
- Status indicator (working/waiting/idle)

**Overview aggregates:**
- Total spend across all instances
- Can sort by consumption to find expensive agents

---

## Instance Lifecycle

### Creating New Instances

1. Click "New Instance" button (or keyboard shortcut)
2. Project selector appears:
   - **Saved projects list** (with colors) for quick launch
   - Browse filesystem
   - Paste a path
3. If new folder: automatically saved to project list with auto-assigned color
4. Select project → new tile appears in overview (with project color)
5. Instance auto-focuses so user can give initial instructions

**Under the hood:**
1. User selects project path
2. App generates unique instance ID
3. App spawns: `claude --project /path/to/project`
4. PTY captures stdout/stderr → renders in tile
5. Jenklaud skill auto-loads (if installed globally or in project)
6. Skill connects via SSE to app, registers instance ID
7. Instance tile goes "live" in overview

### Instance States

- `starting` - CLI process spawning
- `working` - Agent actively doing something
- `waiting` - Agent waiting for user input
- `paused` - User paused the agent
- `stopped` - Agent finished or was killed

### Snooze Behavior

- User can snooze questions to handle later
- Instance waits indefinitely (no autonomous decisions)
- Snoozed questions stay in inbox but are visually dimmed

---

## Jenklaud Skill Architecture

### Claude Code Hooks

```json
{
  "hooks": {
    "PostToolUse": [{
      "command": "jenklaud-notify tool-complete $TOOL_NAME"
    }],
    "Stop": [{
      "command": "jenklaud-notify agent-stopped"
    }],
    "PermissionRequest": [{
      "command": "jenklaud-notify permission-requested"
    }]
  }
}
```

### MCP Server

Lightweight TypeScript MCP server that:
- Maintains SSE connection to Jenklaud app
- Receives control signals (pause, finish) via HTTP POST
- Reports token usage after each response (if extractable)
- Can inject messages into Claude's context (for "wrap up" commands)

### Token Tracking Challenge

Token usage isn't directly exposed in Claude Code. Options for later:
- Parse Claude Code's output for cost info
- Use a proxy that intercepts API calls
- Request feature from Claude Code team

---

## Data Model

### Project (Persistent)

```typescript
interface Project {
  id: string
  path: string
  displayName: string      // editable by user
  color: string            // hex color from palette, editable
  createdAt: Date
  lastUsedAt: Date
}
```

**Color palette (preset, user selects from these):**
```
#E57373 (red)    #64B5F6 (blue)   #81C784 (green)
#FFB74D (orange) #BA68C8 (purple) #4DD0E1 (cyan)
#FFD54F (yellow) #A1887F (brown)  #90A4AE (gray)
```

Colors auto-assigned sequentially on first use; user can change anytime in settings.

### Instance

```typescript
interface Instance {
  id: string
  projectId: string        // references Project.id
  pid: number
  status: 'starting' | 'working' | 'waiting' | 'paused' | 'stopped'

  // Metrics
  tokensUsed: number
  costEstimate: number
  startedAt: Date

  // Current question (if waiting)
  pendingQuestion?: {
    text: string
    askedAt: Date
    context?: string
  }
}
```

Instance inherits `displayName`, `color`, and `path` from its Project.

### Inbox Item

```typescript
interface InboxItem {
  instanceId: string
  projectId: string        // for color lookup
  question: string
  askedAt: Date
  urgency: 'normal' | 'elevated' | 'urgent'
}
```

### Persistence

- `~/.jenklaud/projects.json` - saved projects with colors (persistent, user-editable)
- `~/.jenklaud/state.json` - active instances (restored on app restart)
- Terminal history kept in memory only (not persisted)

**Project management in Settings:**
- View all saved projects
- Edit display name
- Change color (pick from palette)
- Delete projects no longer needed

---

## MVP Scope

### In MVP

- Electron app with Overview and Focus modes
- Spawn/manage Claude Code CLI instances via PTY
- Dynamic grid layout based on instance count
- Instance tiles: mini terminal, status, basic stats, **project color**
- Focus mode: file explorer, Monaco editor, full terminal
- Inbox panel with time-based urgency escalation
- **Questions Queue** (sidebar/modal) for managing all pending questions
- Quick actions: approve, reject, snooze
- Instance controls: pause, finish, kill
- **Persistent project list** with colors (`~/.jenklaud/projects.json`)
- **Project settings**: edit names, change colors, delete
- Jenklaud skill with SSE for status reporting
- Hooks integration for event notifications
- Local HTTP server for skill communication

### NOT in MVP (Future)

- Cloud relay for mobile notifications
- Git diff viewer in focus mode
- Custom tile arrangement (manual drag)
- Multi-instance coordination (one Claude directing others)
- Session persistence across app restarts
- Token tracking (depends on data availability)

---

## Technical Stack

- **Electron** - Desktop app framework
- **React** - UI framework (renderer process)
- **TypeScript** - Throughout
- **xterm.js** - Terminal emulation
- **Monaco Editor** - Code editing
- **node-pty** - PTY spawning for CLI processes
- **Express** - Local HTTP server for skill communication

---

## Project Structure

```
jenklaud/
├── package.json
├── electron/
│   ├── main.ts              # Electron main process
│   ├── server.ts            # Local HTTP server
│   ├── instance-manager.ts  # Spawn/track CLI via PTY
│   └── preload.ts
├── src/
│   ├── App.tsx
│   ├── components/
│   │   ├── Overview/
│   │   │   ├── OverviewGrid.tsx
│   │   │   ├── InstanceTile.tsx
│   │   │   ├── InboxPanel.tsx
│   │   │   └── QuestionsQueue.tsx    # Sidebar/modal for all questions
│   │   ├── Focus/
│   │   │   ├── FocusView.tsx
│   │   │   ├── FileExplorer.tsx
│   │   │   ├── Editor.tsx
│   │   │   └── Terminal.tsx
│   │   ├── Settings/
│   │   │   ├── SettingsModal.tsx
│   │   │   └── ProjectList.tsx       # Manage projects, colors, names
│   │   └── shared/
│   │       ├── StatsBar.tsx
│   │       ├── QuickActions.tsx
│   │       └── ProjectColorBadge.tsx
│   ├── hooks/
│   ├── stores/
│   └── types/
├── skill/
│   ├── skill.md
│   ├── mcp-server/
│   │   ├── index.ts
│   │   └── package.json
│   └── hooks/
└── docs/
    └── plans/
```

---

## Open Questions for Implementation

1. **Token tracking** - How to reliably extract token usage from Claude Code?
2. **Skill installation** - Global install vs per-project? Auto-install on first run?
3. **State sync** - If app crashes, how to reconnect to running Claude processes?
