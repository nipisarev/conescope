# Jenklaud Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an Electron app that serves as a unified command center for managing multiple Claude Code CLI instances across projects.

**Architecture:** Electron main process manages PTY instances and HTTP server. React renderer provides Overview (grid of instances) and Focus (editor + terminal) modes. State managed with Zustand. Communication with Claude instances via SSE.

**Tech Stack:** Electron, React, TypeScript, xterm.js, Monaco Editor, node-pty, Express, Zustand

---

## Phase 1: Project Scaffolding

### Task 1.1: Initialize Electron + React + TypeScript Project

**Files:**
- Create: `package.json`
- Create: `tsconfig.json`
- Create: `electron/main.ts`
- Create: `electron/preload.ts`
- Create: `src/index.tsx`
- Create: `src/App.tsx`
- Create: `index.html`

**Step 1: Initialize npm project**

Run: `npm init -y`

**Step 2: Install dependencies**

Run:
```bash
npm install electron react react-dom zustand xterm xterm-addon-fit @xterm/addon-fit monaco-editor @monaco-editor/react express uuid
npm install -D typescript @types/react @types/react-dom @types/node @types/express @types/uuid electron-builder vite @vitejs/plugin-react
```

**Step 3: Create package.json scripts**

Modify: `package.json`
```json
{
  "name": "jenklaud",
  "version": "0.1.0",
  "main": "dist-electron/main.js",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build && electron-builder",
    "electron:dev": "vite build && electron ."
  }
}
```

**Step 4: Create tsconfig.json**

Create: `tsconfig.json`
```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

**Step 5: Create tsconfig.node.json for Electron**

Create: `tsconfig.node.json`
```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "outDir": "dist-electron",
    "rootDir": "electron"
  },
  "include": ["electron"]
}
```

**Step 6: Create vite.config.ts**

Create: `vite.config.ts`
```typescript
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  base: './',
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  },
  build: {
    outDir: 'dist'
  }
})
```

**Step 7: Create index.html**

Create: `index.html`
```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Jenklaud</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/index.tsx"></script>
  </body>
</html>
```

**Step 8: Create src/index.tsx**

Create: `src/index.tsx`
```typescript
import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
```

**Step 9: Create src/App.tsx**

Create: `src/App.tsx`
```typescript
export default function App() {
  return (
    <div className="app">
      <h1>Jenklaud</h1>
      <p>Multi-instance Claude Code manager</p>
    </div>
  )
}
```

**Step 10: Create src/index.css**

Create: `src/index.css`
```css
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #1e1e1e;
  color: #fff;
}

.app {
  padding: 20px;
}
```

**Step 11: Create electron/main.ts**

Create: `electron/main.ts`
```typescript
import { app, BrowserWindow } from 'electron'
import path from 'path'

function createWindow() {
  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  if (process.env.NODE_ENV === 'development') {
    win.loadURL('http://localhost:5173')
  } else {
    win.loadFile(path.join(__dirname, '../dist/index.html'))
  }
}

app.whenReady().then(createWindow)

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow()
  }
})
```

**Step 12: Create electron/preload.ts**

Create: `electron/preload.ts`
```typescript
import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('electronAPI', {
  // Instance management
  createInstance: (projectPath: string) =>
    ipcRenderer.invoke('instance:create', projectPath),
  killInstance: (instanceId: string) =>
    ipcRenderer.invoke('instance:kill', instanceId),
  pauseInstance: (instanceId: string) =>
    ipcRenderer.invoke('instance:pause', instanceId),
  resumeInstance: (instanceId: string) =>
    ipcRenderer.invoke('instance:resume', instanceId),
  sendInput: (instanceId: string, input: string) =>
    ipcRenderer.invoke('instance:input', instanceId, input),

  // Event listeners
  onInstanceOutput: (callback: (instanceId: string, data: string) => void) => {
    ipcRenderer.on('instance:output', (_, instanceId, data) => callback(instanceId, data))
  },
  onInstanceStatusChange: (callback: (instanceId: string, status: string) => void) => {
    ipcRenderer.on('instance:status', (_, instanceId, status) => callback(instanceId, status))
  },

  // File system
  selectDirectory: () => ipcRenderer.invoke('dialog:selectDirectory'),
  readDirectory: (path: string) => ipcRenderer.invoke('fs:readDirectory', path),
  readFile: (path: string) => ipcRenderer.invoke('fs:readFile', path),
  writeFile: (path: string, content: string) => ipcRenderer.invoke('fs:writeFile', path, content)
})
```

**Step 13: Commit**

Run:
```bash
git add -A
git commit -m "feat: scaffold Electron + React + TypeScript project"
```

---

## Phase 2: Type Definitions

### Task 2.1: Create Core Type Definitions

**Files:**
- Create: `src/types/index.ts`

**Step 1: Create type definitions**

Create: `src/types/index.ts`
```typescript
export interface Project {
  id: string
  path: string
  displayName: string
  color: string
  createdAt: string
  lastUsedAt: string
}

export type InstanceStatus = 'starting' | 'working' | 'waiting' | 'paused' | 'stopped'

export interface PendingQuestion {
  text: string
  askedAt: string
  context?: string
}

export interface Instance {
  id: string
  projectId: string
  pid: number | null
  status: InstanceStatus
  tokensUsed: number
  costEstimate: number
  startedAt: string
  pendingQuestion?: PendingQuestion
  terminalHistory: string[]
}

export type Urgency = 'normal' | 'elevated' | 'urgent'

export interface InboxItem {
  instanceId: string
  projectId: string
  question: string
  askedAt: string
  urgency: Urgency
  snoozed: boolean
}

export const PROJECT_COLORS = [
  '#E57373', // red
  '#64B5F6', // blue
  '#81C784', // green
  '#FFB74D', // orange
  '#BA68C8', // purple
  '#4DD0E1', // cyan
  '#FFD54F', // yellow
  '#A1887F', // brown
  '#90A4AE', // gray
] as const

export type ViewMode = 'overview' | 'focus'

export interface AppState {
  viewMode: ViewMode
  focusedInstanceId: string | null
  questionsQueueOpen: boolean
  settingsOpen: boolean
}
```

**Step 2: Commit**

Run:
```bash
git add src/types/index.ts
git commit -m "feat: add core TypeScript type definitions"
```

---

## Phase 3: State Management

### Task 3.1: Create Zustand Stores

**Files:**
- Create: `src/stores/projectStore.ts`
- Create: `src/stores/instanceStore.ts`
- Create: `src/stores/appStore.ts`

**Step 1: Create project store**

Create: `src/stores/projectStore.ts`
```typescript
import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { v4 as uuid } from 'uuid'
import { Project, PROJECT_COLORS } from '@/types'

interface ProjectStore {
  projects: Project[]
  addProject: (path: string, displayName?: string) => Project
  updateProject: (id: string, updates: Partial<Project>) => void
  deleteProject: (id: string) => void
  getProject: (id: string) => Project | undefined
  getNextColor: () => string
}

export const useProjectStore = create<ProjectStore>()(
  persist(
    (set, get) => ({
      projects: [],

      addProject: (path: string, displayName?: string) => {
        const existing = get().projects.find(p => p.path === path)
        if (existing) {
          set(state => ({
            projects: state.projects.map(p =>
              p.id === existing.id
                ? { ...p, lastUsedAt: new Date().toISOString() }
                : p
            )
          }))
          return existing
        }

        const project: Project = {
          id: uuid(),
          path,
          displayName: displayName || path.split('/').pop() || path,
          color: get().getNextColor(),
          createdAt: new Date().toISOString(),
          lastUsedAt: new Date().toISOString()
        }

        set(state => ({ projects: [...state.projects, project] }))
        return project
      },

      updateProject: (id, updates) => {
        set(state => ({
          projects: state.projects.map(p =>
            p.id === id ? { ...p, ...updates } : p
          )
        }))
      },

      deleteProject: (id) => {
        set(state => ({
          projects: state.projects.filter(p => p.id !== id)
        }))
      },

      getProject: (id) => get().projects.find(p => p.id === id),

      getNextColor: () => {
        const usedColors = get().projects.map(p => p.color)
        const available = PROJECT_COLORS.filter(c => !usedColors.includes(c))
        return available[0] || PROJECT_COLORS[get().projects.length % PROJECT_COLORS.length]
      }
    }),
    {
      name: 'jenklaud-projects'
    }
  )
)
```

**Step 2: Create instance store**

Create: `src/stores/instanceStore.ts`
```typescript
import { create } from 'zustand'
import { v4 as uuid } from 'uuid'
import { Instance, InstanceStatus, PendingQuestion, InboxItem, Urgency } from '@/types'

interface InstanceStore {
  instances: Instance[]

  createInstance: (projectId: string) => Instance
  updateInstance: (id: string, updates: Partial<Instance>) => void
  removeInstance: (id: string) => void
  getInstance: (id: string) => Instance | undefined

  appendTerminalOutput: (id: string, output: string) => void
  setStatus: (id: string, status: InstanceStatus) => void
  setPendingQuestion: (id: string, question: PendingQuestion | undefined) => void

  getInboxItems: () => InboxItem[]
}

function calculateUrgency(askedAt: string): Urgency {
  const waitMs = Date.now() - new Date(askedAt).getTime()
  const waitMinutes = waitMs / 1000 / 60
  if (waitMinutes > 10) return 'urgent'
  if (waitMinutes > 5) return 'elevated'
  return 'normal'
}

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: [],

  createInstance: (projectId: string) => {
    const instance: Instance = {
      id: uuid(),
      projectId,
      pid: null,
      status: 'starting',
      tokensUsed: 0,
      costEstimate: 0,
      startedAt: new Date().toISOString(),
      terminalHistory: []
    }
    set(state => ({ instances: [...state.instances, instance] }))
    return instance
  },

  updateInstance: (id, updates) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id ? { ...i, ...updates } : i
      )
    }))
  },

  removeInstance: (id) => {
    set(state => ({
      instances: state.instances.filter(i => i.id !== id)
    }))
  },

  getInstance: (id) => get().instances.find(i => i.id === id),

  appendTerminalOutput: (id, output) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id
          ? {
              ...i,
              terminalHistory: [...i.terminalHistory.slice(-500), output]
            }
          : i
      )
    }))
  },

  setStatus: (id, status) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id ? { ...i, status } : i
      )
    }))
  },

  setPendingQuestion: (id, question) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id
          ? {
              ...i,
              pendingQuestion: question,
              status: question ? 'waiting' : i.status
            }
          : i
      )
    }))
  },

  getInboxItems: () => {
    return get()
      .instances
      .filter(i => i.pendingQuestion)
      .map(i => ({
        instanceId: i.id,
        projectId: i.projectId,
        question: i.pendingQuestion!.text,
        askedAt: i.pendingQuestion!.askedAt,
        urgency: calculateUrgency(i.pendingQuestion!.askedAt),
        snoozed: false
      }))
      .sort((a, b) => new Date(a.askedAt).getTime() - new Date(b.askedAt).getTime())
  }
}))
```

**Step 3: Create app store**

Create: `src/stores/appStore.ts`
```typescript
import { create } from 'zustand'
import { ViewMode } from '@/types'

interface AppStore {
  viewMode: ViewMode
  focusedInstanceId: string | null
  questionsQueueOpen: boolean
  settingsOpen: boolean
  newInstanceModalOpen: boolean

  setViewMode: (mode: ViewMode) => void
  focusInstance: (instanceId: string) => void
  returnToOverview: () => void
  toggleQuestionsQueue: () => void
  toggleSettings: () => void
  toggleNewInstanceModal: () => void
}

export const useAppStore = create<AppStore>((set) => ({
  viewMode: 'overview',
  focusedInstanceId: null,
  questionsQueueOpen: false,
  settingsOpen: false,
  newInstanceModalOpen: false,

  setViewMode: (mode) => set({ viewMode: mode }),

  focusInstance: (instanceId) => set({
    viewMode: 'focus',
    focusedInstanceId: instanceId
  }),

  returnToOverview: () => set({
    viewMode: 'overview',
    focusedInstanceId: null
  }),

  toggleQuestionsQueue: () => set(state => ({
    questionsQueueOpen: !state.questionsQueueOpen
  })),

  toggleSettings: () => set(state => ({
    settingsOpen: !state.settingsOpen
  })),

  toggleNewInstanceModal: () => set(state => ({
    newInstanceModalOpen: !state.newInstanceModalOpen
  }))
}))
```

**Step 4: Create stores index**

Create: `src/stores/index.ts`
```typescript
export { useProjectStore } from './projectStore'
export { useInstanceStore } from './instanceStore'
export { useAppStore } from './appStore'
```

**Step 5: Commit**

Run:
```bash
git add src/stores/
git commit -m "feat: add Zustand stores for projects, instances, and app state"
```

---

## Phase 4: Instance Manager (Electron Main Process)

### Task 4.1: Create Instance Manager with node-pty

**Files:**
- Create: `electron/instance-manager.ts`
- Modify: `electron/main.ts`

**Step 1: Install node-pty**

Run: `npm install node-pty`

**Step 2: Create instance manager**

Create: `electron/instance-manager.ts`
```typescript
import * as pty from 'node-pty'
import { BrowserWindow } from 'electron'

interface ManagedInstance {
  id: string
  projectPath: string
  pty: pty.IPty
  isPaused: boolean
}

class InstanceManager {
  private instances: Map<string, ManagedInstance> = new Map()
  private mainWindow: BrowserWindow | null = null

  setMainWindow(window: BrowserWindow) {
    this.mainWindow = window
  }

  createInstance(id: string, projectPath: string): void {
    const shell = process.platform === 'win32' ? 'powershell.exe' : 'zsh'

    const ptyProcess = pty.spawn(shell, [], {
      name: 'xterm-256color',
      cols: 120,
      rows: 30,
      cwd: projectPath,
      env: process.env as { [key: string]: string }
    })

    const instance: ManagedInstance = {
      id,
      projectPath,
      pty: ptyProcess,
      isPaused: false
    }

    ptyProcess.onData((data) => {
      if (this.mainWindow && !instance.isPaused) {
        this.mainWindow.webContents.send('instance:output', id, data)
      }
    })

    ptyProcess.onExit(({ exitCode }) => {
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'stopped')
      }
      this.instances.delete(id)
    })

    this.instances.set(id, instance)

    // Start Claude Code CLI
    ptyProcess.write('claude\r')

    if (this.mainWindow) {
      this.mainWindow.webContents.send('instance:status', id, 'working')
    }
  }

  sendInput(id: string, input: string): void {
    const instance = this.instances.get(id)
    if (instance && !instance.isPaused) {
      instance.pty.write(input)
    }
  }

  pauseInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.isPaused = true
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'paused')
      }
    }
  }

  resumeInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.isPaused = false
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'working')
      }
    }
  }

  killInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.pty.kill()
      this.instances.delete(id)
    }
  }

  resizeInstance(id: string, cols: number, rows: number): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.pty.resize(cols, rows)
    }
  }

  cleanup(): void {
    for (const [id, instance] of this.instances) {
      instance.pty.kill()
    }
    this.instances.clear()
  }
}

export const instanceManager = new InstanceManager()
```

**Step 3: Update electron/main.ts with IPC handlers**

Modify: `electron/main.ts`
```typescript
import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import path from 'path'
import fs from 'fs'
import { instanceManager } from './instance-manager'

function createWindow() {
  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  instanceManager.setMainWindow(win)

  if (process.env.NODE_ENV === 'development') {
    win.loadURL('http://localhost:5173')
  } else {
    win.loadFile(path.join(__dirname, '../dist/index.html'))
  }

  return win
}

// IPC Handlers
ipcMain.handle('instance:create', (_, projectPath: string) => {
  const id = crypto.randomUUID()
  instanceManager.createInstance(id, projectPath)
  return id
})

ipcMain.handle('instance:kill', (_, instanceId: string) => {
  instanceManager.killInstance(instanceId)
})

ipcMain.handle('instance:pause', (_, instanceId: string) => {
  instanceManager.pauseInstance(instanceId)
})

ipcMain.handle('instance:resume', (_, instanceId: string) => {
  instanceManager.resumeInstance(instanceId)
})

ipcMain.handle('instance:input', (_, instanceId: string, input: string) => {
  instanceManager.sendInput(instanceId, input)
})

ipcMain.handle('dialog:selectDirectory', async () => {
  const result = await dialog.showOpenDialog({
    properties: ['openDirectory']
  })
  return result.canceled ? null : result.filePaths[0]
})

ipcMain.handle('fs:readDirectory', async (_, dirPath: string) => {
  try {
    const entries = await fs.promises.readdir(dirPath, { withFileTypes: true })
    return entries.map(entry => ({
      name: entry.name,
      isDirectory: entry.isDirectory(),
      path: path.join(dirPath, entry.name)
    }))
  } catch {
    return []
  }
})

ipcMain.handle('fs:readFile', async (_, filePath: string) => {
  try {
    return await fs.promises.readFile(filePath, 'utf-8')
  } catch {
    return null
  }
})

ipcMain.handle('fs:writeFile', async (_, filePath: string, content: string) => {
  try {
    await fs.promises.writeFile(filePath, content, 'utf-8')
    return true
  } catch {
    return false
  }
})

app.whenReady().then(createWindow)

app.on('window-all-closed', () => {
  instanceManager.cleanup()
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow()
  }
})
```

**Step 4: Commit**

Run:
```bash
git add electron/
git commit -m "feat: add PTY-based instance manager for Claude CLI processes"
```

---

## Phase 5: UI Components - Overview Mode

### Task 5.1: Create Top Bar Component

**Files:**
- Create: `src/components/shared/TopBar.tsx`

**Step 1: Create TopBar component**

Create: `src/components/shared/TopBar.tsx`
```typescript
import { useInstanceStore, useAppStore } from '@/stores'
import './TopBar.css'

export function TopBar() {
  const instances = useInstanceStore(state => state.instances)
  const toggleSettings = useAppStore(state => state.toggleSettings)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)

  const totalTokens = instances.reduce((sum, i) => sum + i.tokensUsed, 0)
  const totalCost = instances.reduce((sum, i) => sum + i.costEstimate, 0)

  return (
    <div className="top-bar">
      <div className="top-bar-left">
        <span className="app-name">◉ Jenklaud</span>
      </div>

      <div className="top-bar-center">
        <span className="stat">{instances.length} instances</span>
        <span className="stat-divider">│</span>
        <span className="stat">{(totalTokens / 1000).toFixed(1)}k tokens</span>
        <span className="stat-divider">│</span>
        <span className="stat">${totalCost.toFixed(2)}</span>
      </div>

      <div className="top-bar-right">
        <button className="btn btn-primary" onClick={toggleNewInstanceModal}>
          + New
        </button>
        <button className="btn btn-icon" onClick={toggleSettings}>
          ⚙
        </button>
      </div>
    </div>
  )
}
```

**Step 2: Create TopBar styles**

Create: `src/components/shared/TopBar.css`
```css
.top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  background: #252526;
  border-bottom: 1px solid #3c3c3c;
}

.top-bar-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.app-name {
  font-size: 16px;
  font-weight: 600;
  color: #fff;
}

.top-bar-center {
  display: flex;
  align-items: center;
  gap: 12px;
}

.stat {
  font-size: 13px;
  color: #9d9d9d;
}

.stat-divider {
  color: #4a4a4a;
}

.top-bar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.btn {
  padding: 6px 12px;
  border: none;
  border-radius: 4px;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-primary {
  background: #0e639c;
  color: #fff;
}

.btn-primary:hover {
  background: #1177bb;
}

.btn-icon {
  background: transparent;
  color: #9d9d9d;
  font-size: 16px;
  padding: 6px 8px;
}

.btn-icon:hover {
  background: #3c3c3c;
  color: #fff;
}
```

**Step 3: Commit**

Run:
```bash
git add src/components/shared/
git commit -m "feat: add TopBar component with instance stats"
```

### Task 5.2: Create Instance Tile Component

**Files:**
- Create: `src/components/Overview/InstanceTile.tsx`
- Create: `src/components/Overview/InstanceTile.css`

**Step 1: Create InstanceTile component**

Create: `src/components/Overview/InstanceTile.tsx`
```typescript
import { useProjectStore, useAppStore } from '@/stores'
import { Instance } from '@/types'
import './InstanceTile.css'

interface InstanceTileProps {
  instance: Instance
}

export function InstanceTile({ instance }: InstanceTileProps) {
  const project = useProjectStore(state => state.getProject(instance.projectId))
  const focusInstance = useAppStore(state => state.focusInstance)

  if (!project) return null

  const statusIcon = {
    starting: '◌',
    working: '●',
    waiting: '⏳',
    paused: '⏸',
    stopped: '○'
  }[instance.status]

  const duration = Math.floor(
    (Date.now() - new Date(instance.startedAt).getTime()) / 1000 / 60
  )

  const recentOutput = instance.terminalHistory.slice(-8).join('')

  return (
    <div
      className="instance-tile"
      style={{ borderColor: project.color }}
      onClick={() => focusInstance(instance.id)}
    >
      <div className="tile-header" style={{ borderBottomColor: project.color }}>
        <div className="tile-title">
          <span className="project-name">{project.displayName}</span>
          <span className="project-path">{project.path}</span>
        </div>
        <div className="tile-status">
          <span className="status-icon">{statusIcon}</span>
          <span className="status-text">{instance.status}</span>
        </div>
      </div>

      <div className="tile-terminal">
        <pre>{recentOutput || 'Starting...'}</pre>
      </div>

      {instance.pendingQuestion && (
        <div className="tile-question">
          <span className="question-badge">?</span>
          <span className="question-text">
            {instance.pendingQuestion.text.slice(0, 100)}...
          </span>
        </div>
      )}

      <div className="tile-stats">
        <span>{(instance.tokensUsed / 1000).toFixed(0)}k</span>
        <span className="stats-divider">│</span>
        <span>${instance.costEstimate.toFixed(2)}</span>
        <span className="stats-divider">│</span>
        <span>{duration} min</span>
      </div>
    </div>
  )
}
```

**Step 2: Create InstanceTile styles**

Create: `src/components/Overview/InstanceTile.css`
```css
.instance-tile {
  display: flex;
  flex-direction: column;
  background: #252526;
  border: 2px solid;
  border-radius: 8px;
  overflow: hidden;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.instance-tile:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.tile-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 12px;
  border-bottom: 1px solid;
  background: #2d2d2d;
}

.tile-title {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.project-name {
  font-size: 14px;
  font-weight: 600;
  color: #fff;
}

.project-path {
  font-size: 11px;
  color: #808080;
}

.tile-status {
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-icon {
  font-size: 12px;
}

.status-text {
  font-size: 11px;
  color: #9d9d9d;
  text-transform: capitalize;
}

.tile-terminal {
  flex: 1;
  padding: 8px 12px;
  overflow: hidden;
  min-height: 120px;
  max-height: 160px;
}

.tile-terminal pre {
  font-family: 'SF Mono', Monaco, monospace;
  font-size: 10px;
  color: #d4d4d4;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  line-height: 1.4;
}

.tile-question {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #3c3c3c;
}

.question-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  background: #f9a825;
  color: #000;
  font-size: 11px;
  font-weight: 700;
  border-radius: 50%;
}

.question-text {
  font-size: 11px;
  color: #d4d4d4;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tile-stats {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  font-size: 11px;
  color: #808080;
  background: #1e1e1e;
}

.stats-divider {
  color: #3c3c3c;
}
```

**Step 3: Commit**

Run:
```bash
git add src/components/Overview/
git commit -m "feat: add InstanceTile component with project color and terminal preview"
```

### Task 5.3: Create Overview Grid Component

**Files:**
- Create: `src/components/Overview/OverviewGrid.tsx`
- Create: `src/components/Overview/OverviewGrid.css`
- Create: `src/components/Overview/EmptySlot.tsx`

**Step 1: Create EmptySlot component**

Create: `src/components/Overview/EmptySlot.tsx`
```typescript
import { useAppStore } from '@/stores'
import './OverviewGrid.css'

export function EmptySlot() {
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)

  return (
    <div className="empty-slot" onClick={toggleNewInstanceModal}>
      <div className="empty-slot-content">
        <span className="empty-slot-icon">+</span>
        <span className="empty-slot-text">New Instance</span>
      </div>
    </div>
  )
}
```

**Step 2: Create OverviewGrid component**

Create: `src/components/Overview/OverviewGrid.tsx`
```typescript
import { useInstanceStore } from '@/stores'
import { InstanceTile } from './InstanceTile'
import { EmptySlot } from './EmptySlot'
import './OverviewGrid.css'

export function OverviewGrid() {
  const instances = useInstanceStore(state => state.instances)

  // Calculate grid layout
  const count = instances.length
  let cols = 1
  let rows = 1

  if (count === 0) {
    cols = 1
    rows = 1
  } else if (count === 1) {
    cols = 1
    rows = 1
  } else if (count === 2) {
    cols = 2
    rows = 1
  } else if (count <= 4) {
    cols = 2
    rows = 2
  } else if (count <= 6) {
    cols = 3
    rows = 2
  } else {
    cols = 3
    rows = Math.ceil(count / 3)
  }

  const totalSlots = cols * rows
  const emptySlots = Math.max(0, totalSlots - count)

  // Only show one empty slot for the [+] button
  const showEmptySlot = emptySlots > 0 || count === 0

  return (
    <div
      className="overview-grid"
      style={{
        gridTemplateColumns: `repeat(${cols}, 1fr)`,
        gridTemplateRows: `repeat(${rows}, 1fr)`
      }}
    >
      {instances.map(instance => (
        <InstanceTile key={instance.id} instance={instance} />
      ))}
      {showEmptySlot && <EmptySlot />}
    </div>
  )
}
```

**Step 3: Create OverviewGrid styles**

Create: `src/components/Overview/OverviewGrid.css`
```css
.overview-grid {
  display: grid;
  gap: 16px;
  padding: 16px;
  flex: 1;
  overflow: auto;
}

.empty-slot {
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 2px dashed #3c3c3c;
  border-radius: 8px;
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s;
}

.empty-slot:hover {
  border-color: #0e639c;
  background: rgba(14, 99, 156, 0.1);
}

.empty-slot-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.empty-slot-icon {
  font-size: 48px;
  color: #3c3c3c;
  font-weight: 300;
}

.empty-slot:hover .empty-slot-icon {
  color: #0e639c;
}

.empty-slot-text {
  font-size: 14px;
  color: #808080;
}

.empty-slot:hover .empty-slot-text {
  color: #fff;
}
```

**Step 4: Commit**

Run:
```bash
git add src/components/Overview/
git commit -m "feat: add OverviewGrid with dynamic layout and empty slot button"
```

### Task 5.4: Create Inbox Panel Component

**Files:**
- Create: `src/components/Overview/InboxPanel.tsx`
- Create: `src/components/Overview/InboxPanel.css`

**Step 1: Create InboxPanel component**

Create: `src/components/Overview/InboxPanel.tsx`
```typescript
import { useInstanceStore, useProjectStore, useAppStore } from '@/stores'
import { InboxItem } from '@/types'
import './InboxPanel.css'

interface InboxItemRowProps {
  item: InboxItem
}

function InboxItemRow({ item }: InboxItemRowProps) {
  const project = useProjectStore(state => state.getProject(item.projectId))
  const focusInstance = useAppStore(state => state.focusInstance)

  if (!project) return null

  const waitMinutes = Math.floor(
    (Date.now() - new Date(item.askedAt).getTime()) / 1000 / 60
  )

  const urgencyClass = {
    normal: '',
    elevated: 'urgency-elevated',
    urgent: 'urgency-urgent'
  }[item.urgency]

  return (
    <div
      className={`inbox-item ${urgencyClass}`}
      onClick={() => focusInstance(item.instanceId)}
    >
      <div
        className="inbox-item-color"
        style={{ backgroundColor: project.color }}
      />
      <div className="inbox-item-content">
        <span className="inbox-item-project">{project.displayName}</span>
        <span className="inbox-item-question">{item.question}</span>
        <span className="inbox-item-time">⏱ {waitMinutes}m ago</span>
      </div>
      <div className="inbox-item-actions">
        <button className="action-btn" title="Approve">✓</button>
        <button className="action-btn" title="Reject">✗</button>
        <button className="action-btn" title="Snooze">⏸</button>
      </div>
    </div>
  )
}

export function InboxPanel() {
  const inboxItems = useInstanceStore(state => state.getInboxItems())
  const toggleQuestionsQueue = useAppStore(state => state.toggleQuestionsQueue)

  return (
    <div className="inbox-panel">
      <div className="inbox-header">
        <span className="inbox-title">INBOX ({inboxItems.length})</span>
        <button className="expand-btn" onClick={toggleQuestionsQueue}>↗</button>
      </div>

      <div className="inbox-list">
        {inboxItems.length === 0 ? (
          <div className="inbox-empty">No pending questions</div>
        ) : (
          inboxItems.map(item => (
            <InboxItemRow key={item.instanceId} item={item} />
          ))
        )}
      </div>

      {inboxItems.length > 0 && (
        <button className="view-queue-btn" onClick={toggleQuestionsQueue}>
          View Queue →
        </button>
      )}
    </div>
  )
}
```

**Step 2: Create InboxPanel styles**

Create: `src/components/Overview/InboxPanel.css`
```css
.inbox-panel {
  width: 280px;
  background: #252526;
  border-left: 1px solid #3c3c3c;
  display: flex;
  flex-direction: column;
}

.inbox-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #3c3c3c;
}

.inbox-title {
  font-size: 12px;
  font-weight: 600;
  color: #9d9d9d;
  letter-spacing: 0.5px;
}

.expand-btn {
  background: none;
  border: none;
  color: #808080;
  font-size: 14px;
  cursor: pointer;
  padding: 4px;
}

.expand-btn:hover {
  color: #fff;
}

.inbox-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.inbox-empty {
  padding: 20px;
  text-align: center;
  color: #808080;
  font-size: 13px;
}

.inbox-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px;
  background: #2d2d2d;
  border-radius: 6px;
  margin-bottom: 8px;
  cursor: pointer;
  transition: background 0.2s;
}

.inbox-item:hover {
  background: #3c3c3c;
}

.inbox-item.urgency-elevated {
  border-left: 3px solid #f9a825;
}

.inbox-item.urgency-urgent {
  border-left: 3px solid #e53935;
  background: rgba(229, 57, 53, 0.1);
}

.inbox-item-color {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  margin-top: 4px;
}

.inbox-item-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.inbox-item-project {
  font-size: 12px;
  font-weight: 600;
  color: #fff;
}

.inbox-item-question {
  font-size: 11px;
  color: #d4d4d4;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.inbox-item-time {
  font-size: 10px;
  color: #808080;
}

.inbox-item-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s;
}

.inbox-item:hover .inbox-item-actions {
  opacity: 1;
}

.action-btn {
  background: #3c3c3c;
  border: none;
  color: #d4d4d4;
  width: 24px;
  height: 24px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}

.action-btn:hover {
  background: #4a4a4a;
  color: #fff;
}

.view-queue-btn {
  margin: 8px 16px 16px;
  padding: 10px;
  background: #3c3c3c;
  border: none;
  border-radius: 4px;
  color: #d4d4d4;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.2s;
}

.view-queue-btn:hover {
  background: #4a4a4a;
  color: #fff;
}
```

**Step 3: Commit**

Run:
```bash
git add src/components/Overview/
git commit -m "feat: add InboxPanel with urgency escalation and quick actions"
```

---

## Phase 6: UI Components - Focus Mode

### Task 6.1: Create Focus Mode Terminal Component

**Files:**
- Create: `src/components/Focus/Terminal.tsx`
- Create: `src/components/Focus/Terminal.css`

**Step 1: Create Terminal component**

Create: `src/components/Focus/Terminal.tsx`
```typescript
import { useEffect, useRef } from 'react'
import { Terminal as XTerm } from 'xterm'
import { FitAddon } from '@xterm/addon-fit'
import 'xterm/css/xterm.css'
import './Terminal.css'

interface TerminalProps {
  instanceId: string
  onInput: (data: string) => void
}

export function Terminal({ instanceId, onInput }: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const terminalRef = useRef<XTerm | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)

  useEffect(() => {
    if (!containerRef.current) return

    const terminal = new XTerm({
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#d4d4d4',
        cursorAccent: '#1e1e1e',
        selectionBackground: '#264f78'
      },
      fontFamily: '"SF Mono", Monaco, "Cascadia Code", monospace',
      fontSize: 13,
      lineHeight: 1.2,
      cursorBlink: true
    })

    const fitAddon = new FitAddon()
    terminal.loadAddon(fitAddon)

    terminal.open(containerRef.current)
    fitAddon.fit()

    terminal.onData((data) => {
      onInput(data)
    })

    terminalRef.current = terminal
    fitAddonRef.current = fitAddon

    // Listen for output from this instance
    window.electronAPI.onInstanceOutput((id, data) => {
      if (id === instanceId && terminalRef.current) {
        terminalRef.current.write(data)
      }
    })

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      fitAddonRef.current?.fit()
    })
    resizeObserver.observe(containerRef.current)

    return () => {
      resizeObserver.disconnect()
      terminal.dispose()
    }
  }, [instanceId, onInput])

  return <div ref={containerRef} className="terminal-container" />
}
```

**Step 2: Create Terminal styles**

Create: `src/components/Focus/Terminal.css`
```css
.terminal-container {
  width: 100%;
  height: 100%;
  background: #1e1e1e;
}

.terminal-container .xterm {
  padding: 8px;
}
```

**Step 3: Commit**

Run:
```bash
git add src/components/Focus/
git commit -m "feat: add xterm.js Terminal component with resize support"
```

### Task 6.2: Create Focus View Component

**Files:**
- Create: `src/components/Focus/FocusView.tsx`
- Create: `src/components/Focus/FocusView.css`

**Step 1: Create FocusView component**

Create: `src/components/Focus/FocusView.tsx`
```typescript
import { useCallback } from 'react'
import { useInstanceStore, useProjectStore, useAppStore } from '@/stores'
import { Terminal } from './Terminal'
import './FocusView.css'

export function FocusView() {
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const instance = useInstanceStore(state =>
    focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
  )
  const project = useProjectStore(state =>
    instance ? state.getProject(instance.projectId) : undefined
  )

  const handleInput = useCallback((data: string) => {
    if (focusedInstanceId) {
      window.electronAPI.sendInput(focusedInstanceId, data)
    }
  }, [focusedInstanceId])

  const handlePause = () => {
    if (focusedInstanceId) {
      window.electronAPI.pauseInstance(focusedInstanceId)
    }
  }

  const handleKill = () => {
    if (focusedInstanceId && confirm('Are you sure you want to kill this instance?')) {
      window.electronAPI.killInstance(focusedInstanceId)
      returnToOverview()
    }
  }

  if (!instance || !project) {
    return (
      <div className="focus-view-empty">
        <p>No instance selected</p>
        <button onClick={returnToOverview}>Return to Overview</button>
      </div>
    )
  }

  const duration = Math.floor(
    (Date.now() - new Date(instance.startedAt).getTime()) / 1000 / 60
  )

  return (
    <div className="focus-view">
      <div className="focus-header" style={{ borderBottomColor: project.color }}>
        <button className="back-btn" onClick={returnToOverview}>
          ← Back to Overview
        </button>
        <span className="focus-project-name">{project.displayName}</span>
        <div className="focus-actions">
          {instance.status === 'paused' ? (
            <button
              className="action-btn"
              onClick={() => window.electronAPI.resumeInstance(focusedInstanceId!)}
            >
              Resume
            </button>
          ) : (
            <button className="action-btn" onClick={handlePause}>
              Pause
            </button>
          )}
          <button className="action-btn action-btn-danger" onClick={handleKill}>
            Kill
          </button>
        </div>
      </div>

      <div className="focus-content">
        <div className="focus-sidebar">
          <div className="sidebar-section">
            <h3>Files</h3>
            <p className="placeholder">File explorer coming soon</p>
          </div>
        </div>

        <div className="focus-main">
          <div className="focus-editor">
            <p className="placeholder">Monaco Editor coming soon</p>
          </div>

          <div className="focus-terminal">
            <Terminal instanceId={instance.id} onInput={handleInput} />
          </div>
        </div>
      </div>

      <div className="focus-stats-bar">
        <span>Stats: {(instance.tokensUsed / 1000).toFixed(0)}k tokens</span>
        <span className="stats-divider">│</span>
        <span>${instance.costEstimate.toFixed(2)}</span>
        <span className="stats-divider">│</span>
        <span>{duration} min</span>
        <span className="stats-divider">│</span>
        <span>Git Changes: 0</span>
      </div>
    </div>
  )
}
```

**Step 2: Create FocusView styles**

Create: `src/components/Focus/FocusView.css`
```css
.focus-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #1e1e1e;
}

.focus-view-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  gap: 16px;
  color: #808080;
}

.focus-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  background: #252526;
  border-bottom: 2px solid;
}

.back-btn {
  background: none;
  border: none;
  color: #808080;
  font-size: 13px;
  cursor: pointer;
  padding: 6px 12px;
}

.back-btn:hover {
  color: #fff;
}

.focus-project-name {
  font-size: 16px;
  font-weight: 600;
  color: #fff;
}

.focus-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 6px 12px;
  background: #3c3c3c;
  border: none;
  border-radius: 4px;
  color: #d4d4d4;
  font-size: 13px;
  cursor: pointer;
}

.action-btn:hover {
  background: #4a4a4a;
  color: #fff;
}

.action-btn-danger:hover {
  background: #e53935;
}

.focus-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.focus-sidebar {
  width: 240px;
  background: #252526;
  border-right: 1px solid #3c3c3c;
  overflow-y: auto;
}

.sidebar-section {
  padding: 12px;
}

.sidebar-section h3 {
  font-size: 11px;
  font-weight: 600;
  color: #9d9d9d;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 8px;
}

.placeholder {
  color: #808080;
  font-size: 12px;
  padding: 12px;
  text-align: center;
}

.focus-main {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.focus-editor {
  flex: 1;
  background: #1e1e1e;
  border-bottom: 1px solid #3c3c3c;
  display: flex;
  align-items: center;
  justify-content: center;
}

.focus-terminal {
  height: 300px;
  min-height: 200px;
}

.focus-stats-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 20px;
  background: #007acc;
  color: #fff;
  font-size: 12px;
}

.stats-divider {
  opacity: 0.5;
}
```

**Step 3: Commit**

Run:
```bash
git add src/components/Focus/
git commit -m "feat: add FocusView with terminal, header controls, and stats bar"
```

---

## Phase 7: Wire Up App

### Task 7.1: Update App.tsx with Routing

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/index.css`

**Step 1: Update App.tsx**

Modify: `src/App.tsx`
```typescript
import { useAppStore } from '@/stores'
import { TopBar } from '@/components/shared/TopBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { InboxPanel } from '@/components/Overview/InboxPanel'
import { FocusView } from '@/components/Focus/FocusView'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)

  if (viewMode === 'focus') {
    return <FocusView />
  }

  return (
    <div className="app-container">
      <TopBar />
      <div className="main-content">
        <OverviewGrid />
        <InboxPanel />
      </div>
    </div>
  )
}
```

**Step 2: Update index.css**

Modify: `src/index.css`
```css
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #root {
  height: 100%;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #1e1e1e;
  color: #fff;
  overflow: hidden;
}

.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.main-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}
```

**Step 3: Commit**

Run:
```bash
git add src/App.tsx src/index.css
git commit -m "feat: wire up App with Overview and Focus mode routing"
```

---

## Phase 8: New Instance Modal

### Task 8.1: Create New Instance Modal

**Files:**
- Create: `src/components/shared/NewInstanceModal.tsx`
- Create: `src/components/shared/NewInstanceModal.css`
- Modify: `src/App.tsx`

**Step 1: Create NewInstanceModal component**

Create: `src/components/shared/NewInstanceModal.tsx`
```typescript
import { useState } from 'react'
import { useProjectStore, useInstanceStore, useAppStore } from '@/stores'
import './NewInstanceModal.css'

export function NewInstanceModal() {
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const projects = useProjectStore(state => state.projects)
  const addProject = useProjectStore(state => state.addProject)
  const createInstance = useInstanceStore(state => state.createInstance)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const focusInstance = useAppStore(state => state.focusInstance)

  const handleSelectDirectory = async () => {
    const path = await window.electronAPI.selectDirectory()
    if (path) {
      setSelectedPath(path)
    }
  }

  const handleCreateInstance = async (projectPath: string) => {
    const project = addProject(projectPath)
    const instance = createInstance(project.id)
    await window.electronAPI.createInstance(projectPath)
    toggleNewInstanceModal()
    focusInstance(instance.id)
  }

  const handleClose = () => {
    toggleNewInstanceModal()
    setSelectedPath(null)
  }

  return (
    <div className="modal-overlay" onClick={handleClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>New Instance</h2>
          <button className="close-btn" onClick={handleClose}>×</button>
        </div>

        <div className="modal-content">
          {projects.length > 0 && (
            <div className="section">
              <h3>Recent Projects</h3>
              <div className="project-list">
                {projects.map(project => (
                  <button
                    key={project.id}
                    className="project-item"
                    onClick={() => handleCreateInstance(project.path)}
                  >
                    <div
                      className="project-color"
                      style={{ backgroundColor: project.color }}
                    />
                    <div className="project-info">
                      <span className="project-name">{project.displayName}</span>
                      <span className="project-path">{project.path}</span>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          )}

          <div className="section">
            <h3>Browse</h3>
            <button className="browse-btn" onClick={handleSelectDirectory}>
              Select Directory...
            </button>
            {selectedPath && (
              <div className="selected-path">
                <span>{selectedPath}</span>
                <button
                  className="launch-btn"
                  onClick={() => handleCreateInstance(selectedPath)}
                >
                  Launch
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
```

**Step 2: Create NewInstanceModal styles**

Create: `src/components/shared/NewInstanceModal.css`
```css
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal {
  background: #252526;
  border-radius: 8px;
  width: 480px;
  max-height: 80vh;
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid #3c3c3c;
}

.modal-header h2 {
  font-size: 16px;
  font-weight: 600;
  color: #fff;
}

.close-btn {
  background: none;
  border: none;
  color: #808080;
  font-size: 24px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.close-btn:hover {
  color: #fff;
}

.modal-content {
  padding: 20px;
  overflow-y: auto;
  max-height: 60vh;
}

.section {
  margin-bottom: 24px;
}

.section:last-child {
  margin-bottom: 0;
}

.section h3 {
  font-size: 11px;
  font-weight: 600;
  color: #9d9d9d;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 12px;
}

.project-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.project-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: #2d2d2d;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  text-align: left;
  transition: background 0.2s;
}

.project-item:hover {
  background: #3c3c3c;
}

.project-color {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.project-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.project-name {
  font-size: 14px;
  color: #fff;
}

.project-path {
  font-size: 11px;
  color: #808080;
}

.browse-btn {
  width: 100%;
  padding: 12px;
  background: #3c3c3c;
  border: 1px dashed #5a5a5a;
  border-radius: 6px;
  color: #d4d4d4;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s, border-color 0.2s;
}

.browse-btn:hover {
  background: #4a4a4a;
  border-color: #0e639c;
}

.selected-path {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 12px;
  padding: 12px;
  background: #2d2d2d;
  border-radius: 6px;
}

.selected-path span {
  font-size: 12px;
  color: #d4d4d4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.launch-btn {
  padding: 6px 16px;
  background: #0e639c;
  border: none;
  border-radius: 4px;
  color: #fff;
  font-size: 13px;
  cursor: pointer;
  flex-shrink: 0;
}

.launch-btn:hover {
  background: #1177bb;
}
```

**Step 3: Update App.tsx to include modal**

Modify: `src/App.tsx`
```typescript
import { useAppStore } from '@/stores'
import { TopBar } from '@/components/shared/TopBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { InboxPanel } from '@/components/Overview/InboxPanel'
import { FocusView } from '@/components/Focus/FocusView'
import { NewInstanceModal } from '@/components/shared/NewInstanceModal'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)
  const newInstanceModalOpen = useAppStore(state => state.newInstanceModalOpen)

  if (viewMode === 'focus') {
    return (
      <>
        <FocusView />
        {newInstanceModalOpen && <NewInstanceModal />}
      </>
    )
  }

  return (
    <div className="app-container">
      <TopBar />
      <div className="main-content">
        <OverviewGrid />
        <InboxPanel />
      </div>
      {newInstanceModalOpen && <NewInstanceModal />}
    </div>
  )
}
```

**Step 4: Commit**

Run:
```bash
git add src/components/shared/ src/App.tsx
git commit -m "feat: add NewInstanceModal with project list and directory browser"
```

---

## Phase 9: TypeScript Declaration for Electron API

### Task 9.1: Add Window Type Declaration

**Files:**
- Create: `src/types/electron.d.ts`

**Step 1: Create electron type declaration**

Create: `src/types/electron.d.ts`
```typescript
export interface ElectronAPI {
  // Instance management
  createInstance: (projectPath: string) => Promise<string>
  killInstance: (instanceId: string) => Promise<void>
  pauseInstance: (instanceId: string) => Promise<void>
  resumeInstance: (instanceId: string) => Promise<void>
  sendInput: (instanceId: string, input: string) => Promise<void>

  // Event listeners
  onInstanceOutput: (callback: (instanceId: string, data: string) => void) => void
  onInstanceStatusChange: (callback: (instanceId: string, status: string) => void) => void

  // File system
  selectDirectory: () => Promise<string | null>
  readDirectory: (path: string) => Promise<Array<{
    name: string
    isDirectory: boolean
    path: string
  }>>
  readFile: (path: string) => Promise<string | null>
  writeFile: (path: string, content: string) => Promise<boolean>
}

declare global {
  interface Window {
    electronAPI: ElectronAPI
  }
}

export {}
```

**Step 2: Update tsconfig.json to include declaration**

Modify: `tsconfig.json` - add to include array:
```json
{
  "include": ["src", "src/types/electron.d.ts"]
}
```

**Step 3: Commit**

Run:
```bash
git add src/types/electron.d.ts tsconfig.json
git commit -m "feat: add TypeScript declarations for Electron API"
```

---

## Summary

This implementation plan covers the MVP features:

1. **Project Scaffolding** - Electron + React + TypeScript + Vite
2. **Type Definitions** - Core types for Project, Instance, InboxItem
3. **State Management** - Zustand stores with persistence
4. **Instance Manager** - PTY-based Claude CLI process management
5. **Overview Mode** - Grid layout, instance tiles, inbox panel
6. **Focus Mode** - Terminal with xterm.js, header controls
7. **New Instance Modal** - Project selector with persistent list

**Not included in this plan (future phases):**
- Monaco Editor integration
- File Explorer component
- Questions Queue modal
- Settings modal with project management
- Jenklaud Skill and hooks integration
- SSE communication layer

**Total tasks:** 15 tasks across 9 phases
**Estimated commits:** ~15 atomic commits
