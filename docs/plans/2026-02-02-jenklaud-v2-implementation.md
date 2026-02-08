# Jenklaud V2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform Jenklaud into a fully functional Claude Code command center with integrated editor, file explorer, persistent SQLite storage, and redesigned UI.

**Architecture:** Electron main process handles SQLite database and PTY management. React renderer uses Zustand stores synced with SQLite. CodeMirror 6 for editing, react-arborist for file tree, xterm.js for terminal.

**Tech Stack:** CodeMirror 6, react-arborist, better-sqlite3, xterm.js, Zustand, Electron, React, TypeScript

---

## Phase 1: Foundation (Dependencies & Database)

### Task 1: Install New Dependencies

**Files:**
- Modify: `package.json`

**Step 1: Remove Monaco (switching to CodeMirror)**

```bash
npm uninstall @monaco-editor/react monaco-editor
```

**Step 2: Install CodeMirror 6 packages**

```bash
npm install @uiw/react-codemirror @codemirror/lang-javascript @codemirror/lang-typescript @codemirror/lang-json @codemirror/lang-markdown @codemirror/lang-css @codemirror/lang-html @codemirror/lang-python
```

**Step 3: Install react-arborist for file tree**

```bash
npm install react-arborist
```

**Step 4: Install better-sqlite3 for database**

```bash
npm install better-sqlite3
npm install -D @types/better-sqlite3
```

**Step 5: Rebuild native modules for Electron**

```bash
npx electron-rebuild -f -w better-sqlite3 node-pty
```

**Step 6: Verify installation**

Run: `npm run dev`
Expected: App starts without errors

**Step 7: Commit**

```bash
git add package.json package-lock.json
git commit -m "feat: install CodeMirror 6, react-arborist, better-sqlite3"
```

---

### Task 2: Create Database Service

**Files:**
- Create: `electron/database.ts`

**Step 1: Create the database service**

```typescript
import Database from 'better-sqlite3'
import path from 'path'
import { app } from 'electron'
import { logger } from './logger'

const DB_NAME = 'jenklaud.db'

class DatabaseService {
  private db: Database.Database | null = null

  getDbPath(): string {
    const userDataPath = app.isPackaged
      ? app.getPath('userData')
      : path.join(process.cwd(), 'data')
    return path.join(userDataPath, DB_NAME)
  }

  initialize(): void {
    const dbPath = this.getDbPath()
    logger.info('Initializing database', { path: dbPath })

    // Ensure directory exists
    const fs = require('fs')
    const dir = path.dirname(dbPath)
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true })
    }

    this.db = new Database(dbPath)
    this.db.pragma('journal_mode = WAL')
    this.db.pragma('foreign_keys = ON')

    this.migrate()
    logger.info('Database initialized')
  }

  private migrate(): void {
    if (!this.db) throw new Error('Database not initialized')

    // Create tables
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        path TEXT UNIQUE NOT NULL,
        display_name TEXT NOT NULL,
        color TEXT NOT NULL,
        created_at TEXT NOT NULL,
        last_used_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS instances (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        title TEXT,
        status TEXT NOT NULL DEFAULT 'starting',
        instance_number INTEGER,
        tokens_used INTEGER DEFAULT 0,
        cost_estimate REAL DEFAULT 0,
        started_at TEXT NOT NULL,
        ended_at TEXT,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
      );

      CREATE TABLE IF NOT EXISTS questions (
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

      CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
    `)

    // Insert default settings if not exist
    const insertSetting = this.db.prepare(
      'INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)'
    )
    insertSetting.run('theme', 'dark')
    insertSetting.run('questions_panel_visible', 'true')
    insertSetting.run('editor_font_size', '13')
    insertSetting.run('terminal_font_size', '13')

    logger.info('Database migration complete')
  }

  // Projects
  getAllProjects(): any[] {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM projects ORDER BY last_used_at DESC').all()
  }

  getProject(id: string): any {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM projects WHERE id = ?').get(id)
  }

  insertProject(project: any): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      INSERT INTO projects (id, path, display_name, color, created_at, last_used_at)
      VALUES (?, ?, ?, ?, ?, ?)
    `).run(project.id, project.path, project.display_name, project.color, project.created_at, project.last_used_at)
  }

  updateProject(id: string, updates: any): void {
    if (!this.db) throw new Error('Database not initialized')
    const fields = Object.keys(updates).map(k => `${k} = ?`).join(', ')
    const values = [...Object.values(updates), id]
    this.db.prepare(`UPDATE projects SET ${fields} WHERE id = ?`).run(...values)
  }

  deleteProject(id: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare('DELETE FROM projects WHERE id = ?').run(id)
  }

  // Instances
  getAllInstances(): any[] {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM instances WHERE ended_at IS NULL ORDER BY started_at ASC').all()
  }

  getInstance(id: string): any {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM instances WHERE id = ?').get(id)
  }

  insertInstance(instance: any): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      INSERT INTO instances (id, project_id, title, status, instance_number, tokens_used, cost_estimate, started_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      instance.id,
      instance.project_id,
      instance.title,
      instance.status,
      instance.instance_number,
      instance.tokens_used,
      instance.cost_estimate,
      instance.started_at
    )
  }

  updateInstance(id: string, updates: any): void {
    if (!this.db) throw new Error('Database not initialized')
    const fields = Object.keys(updates).map(k => `${k} = ?`).join(', ')
    const values = [...Object.values(updates), id]
    this.db.prepare(`UPDATE instances SET ${fields} WHERE id = ?`).run(...values)
  }

  deleteInstance(id: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare('DELETE FROM instances WHERE id = ?').run(id)
  }

  getNextInstanceNumber(): number {
    if (!this.db) throw new Error('Database not initialized')
    const result = this.db.prepare(
      'SELECT COALESCE(MAX(instance_number), 0) + 1 as next FROM instances WHERE ended_at IS NULL'
    ).get() as { next: number }
    return result.next
  }

  // Settings
  getSetting(key: string): string | null {
    if (!this.db) throw new Error('Database not initialized')
    const row = this.db.prepare('SELECT value FROM settings WHERE key = ?').get(key) as { value: string } | undefined
    return row?.value ?? null
  }

  setSetting(key: string, value: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare('INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)').run(key, value)
  }

  getAllSettings(): Record<string, string> {
    if (!this.db) throw new Error('Database not initialized')
    const rows = this.db.prepare('SELECT key, value FROM settings').all() as { key: string; value: string }[]
    return Object.fromEntries(rows.map(r => [r.key, r.value]))
  }

  // Questions
  getPendingQuestions(): any[] {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare(`
      SELECT q.*, i.title as instance_title, p.display_name as project_name, p.color as project_color
      FROM questions q
      JOIN instances i ON q.instance_id = i.id
      JOIN projects p ON i.project_id = p.id
      WHERE q.answered_at IS NULL
      ORDER BY q.asked_at ASC
    `).all()
  }

  insertQuestion(question: any): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      INSERT INTO questions (id, instance_id, question_text, context, asked_at)
      VALUES (?, ?, ?, ?, ?)
    `).run(question.id, question.instance_id, question.question_text, question.context, question.asked_at)
  }

  answerQuestion(id: string, answer: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      UPDATE questions SET answered_at = ?, answer = ? WHERE id = ?
    `).run(new Date().toISOString(), answer, id)
  }

  close(): void {
    if (this.db) {
      this.db.close()
      this.db = null
      logger.info('Database closed')
    }
  }
}

export const database = new DatabaseService()
```

**Step 2: Verify file created**

Run: `cat electron/database.ts | head -20`
Expected: Shows start of database service

**Step 3: Commit**

```bash
git add electron/database.ts
git commit -m "feat: add SQLite database service with migrations"
```

---

### Task 3: Add Database IPC Handlers

**Files:**
- Modify: `electron/main.ts`
- Modify: `electron/preload.ts`
- Modify: `src/types/electron.d.ts`

**Step 1: Update main.ts to initialize database and add IPC handlers**

Add imports at top of `electron/main.ts`:
```typescript
import { database } from './database'
```

Add database initialization in app.whenReady():
```typescript
app.whenReady().then(() => {
  logger.info('App ready')
  database.initialize()  // Add this line
  createWindow()
})
```

Add cleanup on quit:
```typescript
app.on('window-all-closed', () => {
  logger.info('All windows closed')
  instanceManager.cleanup()
  database.close()  // Add this line
  logger.close()
  if (process.platform !== 'darwin') {
    app.quit()
  }
})
```

Add IPC handlers after existing ones:
```typescript
// Database IPC Handlers
ipcMain.handle('db:projects:getAll', () => database.getAllProjects())
ipcMain.handle('db:projects:get', (_, id: string) => database.getProject(id))
ipcMain.handle('db:projects:insert', (_, project) => database.insertProject(project))
ipcMain.handle('db:projects:update', (_, id: string, updates) => database.updateProject(id, updates))
ipcMain.handle('db:projects:delete', (_, id: string) => database.deleteProject(id))

ipcMain.handle('db:instances:getAll', () => database.getAllInstances())
ipcMain.handle('db:instances:get', (_, id: string) => database.getInstance(id))
ipcMain.handle('db:instances:insert', (_, instance) => database.insertInstance(instance))
ipcMain.handle('db:instances:update', (_, id: string, updates) => database.updateInstance(id, updates))
ipcMain.handle('db:instances:delete', (_, id: string) => database.deleteInstance(id))
ipcMain.handle('db:instances:getNextNumber', () => database.getNextInstanceNumber())

ipcMain.handle('db:settings:get', (_, key: string) => database.getSetting(key))
ipcMain.handle('db:settings:set', (_, key: string, value: string) => database.setSetting(key, value))
ipcMain.handle('db:settings:getAll', () => database.getAllSettings())

ipcMain.handle('db:questions:getPending', () => database.getPendingQuestions())
ipcMain.handle('db:questions:insert', (_, question) => database.insertQuestion(question))
ipcMain.handle('db:questions:answer', (_, id: string, answer: string) => database.answerQuestion(id, answer))
```

**Step 2: Update preload.ts to expose database APIs**

Add to the contextBridge.exposeInMainWorld object in `electron/preload.ts`:
```typescript
  // Database - Projects
  dbProjectsGetAll: () => ipcRenderer.invoke('db:projects:getAll'),
  dbProjectsGet: (id: string) => ipcRenderer.invoke('db:projects:get', id),
  dbProjectsInsert: (project: any) => ipcRenderer.invoke('db:projects:insert', project),
  dbProjectsUpdate: (id: string, updates: any) => ipcRenderer.invoke('db:projects:update', id, updates),
  dbProjectsDelete: (id: string) => ipcRenderer.invoke('db:projects:delete', id),

  // Database - Instances
  dbInstancesGetAll: () => ipcRenderer.invoke('db:instances:getAll'),
  dbInstancesGet: (id: string) => ipcRenderer.invoke('db:instances:get', id),
  dbInstancesInsert: (instance: any) => ipcRenderer.invoke('db:instances:insert', instance),
  dbInstancesUpdate: (id: string, updates: any) => ipcRenderer.invoke('db:instances:update', id, updates),
  dbInstancesDelete: (id: string) => ipcRenderer.invoke('db:instances:delete', id),
  dbInstancesGetNextNumber: () => ipcRenderer.invoke('db:instances:getNextNumber'),

  // Database - Settings
  dbSettingsGet: (key: string) => ipcRenderer.invoke('db:settings:get', key),
  dbSettingsSet: (key: string, value: string) => ipcRenderer.invoke('db:settings:set', key, value),
  dbSettingsGetAll: () => ipcRenderer.invoke('db:settings:getAll'),

  // Database - Questions
  dbQuestionsGetPending: () => ipcRenderer.invoke('db:questions:getPending'),
  dbQuestionsInsert: (question: any) => ipcRenderer.invoke('db:questions:insert', question),
  dbQuestionsAnswer: (id: string, answer: string) => ipcRenderer.invoke('db:questions:answer', id, answer),
```

**Step 3: Update electron.d.ts with new API types**

Replace content of `src/types/electron.d.ts`:
```typescript
export interface DbProject {
  id: string
  path: string
  display_name: string
  color: string
  created_at: string
  last_used_at: string
}

export interface DbInstance {
  id: string
  project_id: string
  title: string | null
  status: string
  instance_number: number
  tokens_used: number
  cost_estimate: number
  started_at: string
  ended_at: string | null
}

export interface DbQuestion {
  id: string
  instance_id: string
  question_text: string
  context: string | null
  asked_at: string
  answered_at: string | null
  answer: string | null
  snoozed_until: string | null
  instance_title?: string
  project_name?: string
  project_color?: string
}

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

  // Database - Projects
  dbProjectsGetAll: () => Promise<DbProject[]>
  dbProjectsGet: (id: string) => Promise<DbProject | undefined>
  dbProjectsInsert: (project: DbProject) => Promise<void>
  dbProjectsUpdate: (id: string, updates: Partial<DbProject>) => Promise<void>
  dbProjectsDelete: (id: string) => Promise<void>

  // Database - Instances
  dbInstancesGetAll: () => Promise<DbInstance[]>
  dbInstancesGet: (id: string) => Promise<DbInstance | undefined>
  dbInstancesInsert: (instance: DbInstance) => Promise<void>
  dbInstancesUpdate: (id: string, updates: Partial<DbInstance>) => Promise<void>
  dbInstancesDelete: (id: string) => Promise<void>
  dbInstancesGetNextNumber: () => Promise<number>

  // Database - Settings
  dbSettingsGet: (key: string) => Promise<string | null>
  dbSettingsSet: (key: string, value: string) => Promise<void>
  dbSettingsGetAll: () => Promise<Record<string, string>>

  // Database - Questions
  dbQuestionsGetPending: () => Promise<DbQuestion[]>
  dbQuestionsInsert: (question: Omit<DbQuestion, 'answered_at' | 'answer' | 'snoozed_until'>) => Promise<void>
  dbQuestionsAnswer: (id: string, answer: string) => Promise<void>
}

declare global {
  interface Window {
    electronAPI: ElectronAPI
  }
}

export {}
```

**Step 4: Compile and verify**

Run: `npm run dev:electron`
Expected: No TypeScript errors, app starts

**Step 5: Commit**

```bash
git add electron/main.ts electron/preload.ts src/types/electron.d.ts
git commit -m "feat: add database IPC handlers and TypeScript types"
```

---

### Task 4: Update Zustand Stores to Sync with SQLite

**Files:**
- Modify: `src/stores/projectStore.ts`
- Modify: `src/stores/instanceStore.ts`
- Create: `src/stores/settingsStore.ts`
- Modify: `src/stores/index.ts`
- Modify: `src/types/index.ts`

**Step 1: Update types in `src/types/index.ts`**

Replace entire file:
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

export interface Instance {
  id: string
  projectId: string
  title: string
  instanceNumber: number
  status: InstanceStatus
  tokensUsed: number
  costEstimate: number
  startedAt: string
  terminalHistory: string[]
}

export interface Question {
  id: string
  instanceId: string
  questionText: string
  context?: string
  askedAt: string
  answeredAt?: string
  answer?: string
  snoozedUntil?: string
  // Joined fields
  instanceTitle?: string
  projectName?: string
  projectColor?: string
}

export const PROJECT_COLORS = [
  '#E57373', '#64B5F6', '#81C784', '#FFB74D', '#BA68C8',
  '#4DD0E1', '#FFD54F', '#A1887F', '#90A4AE',
] as const

export type ViewMode = 'overview' | 'focus'
```

**Step 2: Rewrite `src/stores/projectStore.ts`**

```typescript
import { create } from 'zustand'
import { v4 as uuid } from 'uuid'
import { Project, PROJECT_COLORS } from '@/types'

interface ProjectStore {
  projects: Project[]
  isLoading: boolean

  loadProjects: () => Promise<void>
  addProject: (path: string, displayName?: string) => Promise<Project>
  updateProject: (id: string, updates: Partial<Project>) => Promise<void>
  deleteProject: (id: string) => Promise<void>
  getProject: (id: string) => Project | undefined
  getNextColor: () => string
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  projects: [],
  isLoading: true,

  loadProjects: async () => {
    const dbProjects = await window.electronAPI.dbProjectsGetAll()
    const projects: Project[] = dbProjects.map(p => ({
      id: p.id,
      path: p.path,
      displayName: p.display_name,
      color: p.color,
      createdAt: p.created_at,
      lastUsedAt: p.last_used_at,
    }))
    set({ projects, isLoading: false })
  },

  addProject: async (path: string, displayName?: string) => {
    const existing = get().projects.find(p => p.path === path)
    if (existing) {
      const updates = { lastUsedAt: new Date().toISOString() }
      await window.electronAPI.dbProjectsUpdate(existing.id, { last_used_at: updates.lastUsedAt })
      set(state => ({
        projects: state.projects.map(p =>
          p.id === existing.id ? { ...p, ...updates } : p
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
      lastUsedAt: new Date().toISOString(),
    }

    await window.electronAPI.dbProjectsInsert({
      id: project.id,
      path: project.path,
      display_name: project.displayName,
      color: project.color,
      created_at: project.createdAt,
      last_used_at: project.lastUsedAt,
    })

    set(state => ({ projects: [...state.projects, project] }))
    return project
  },

  updateProject: async (id, updates) => {
    const dbUpdates: Record<string, any> = {}
    if (updates.displayName) dbUpdates.display_name = updates.displayName
    if (updates.color) dbUpdates.color = updates.color
    if (updates.lastUsedAt) dbUpdates.last_used_at = updates.lastUsedAt

    await window.electronAPI.dbProjectsUpdate(id, dbUpdates)
    set(state => ({
      projects: state.projects.map(p => p.id === id ? { ...p, ...updates } : p)
    }))
  },

  deleteProject: async (id) => {
    await window.electronAPI.dbProjectsDelete(id)
    set(state => ({ projects: state.projects.filter(p => p.id !== id) }))
  },

  getProject: (id) => get().projects.find(p => p.id === id),

  getNextColor: () => {
    const usedColors = get().projects.map(p => p.color)
    const available = PROJECT_COLORS.filter(c => !usedColors.includes(c))
    return available[0] || PROJECT_COLORS[get().projects.length % PROJECT_COLORS.length]
  },
}))
```

**Step 3: Rewrite `src/stores/instanceStore.ts`**

```typescript
import { create } from 'zustand'
import { v4 as uuid } from 'uuid'
import { Instance, InstanceStatus } from '@/types'

interface InstanceStore {
  instances: Instance[]
  isLoading: boolean

  loadInstances: () => Promise<void>
  createInstance: (projectId: string, projectName: string) => Promise<Instance>
  updateInstance: (id: string, updates: Partial<Instance>) => void
  updateInstanceDb: (id: string, updates: Partial<Instance>) => Promise<void>
  removeInstance: (id: string) => Promise<void>
  getInstance: (id: string) => Instance | undefined
  getInstanceByNumber: (num: number) => Instance | undefined

  appendTerminalOutput: (id: string, output: string) => void
  setStatus: (id: string, status: InstanceStatus) => void
}

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: [],
  isLoading: true,

  loadInstances: async () => {
    const dbInstances = await window.electronAPI.dbInstancesGetAll()
    const instances: Instance[] = dbInstances.map(i => ({
      id: i.id,
      projectId: i.project_id,
      title: i.title || '',
      instanceNumber: i.instance_number,
      status: i.status as InstanceStatus,
      tokensUsed: i.tokens_used,
      costEstimate: i.cost_estimate,
      startedAt: i.started_at,
      terminalHistory: [],
    }))
    set({ instances, isLoading: false })
  },

  createInstance: async (projectId: string, projectName: string) => {
    const instanceNumber = await window.electronAPI.dbInstancesGetNextNumber()
    const instance: Instance = {
      id: uuid(),
      projectId,
      title: projectName,
      instanceNumber,
      status: 'starting',
      tokensUsed: 0,
      costEstimate: 0,
      startedAt: new Date().toISOString(),
      terminalHistory: [],
    }

    await window.electronAPI.dbInstancesInsert({
      id: instance.id,
      project_id: instance.projectId,
      title: instance.title,
      status: instance.status,
      instance_number: instance.instanceNumber,
      tokens_used: instance.tokensUsed,
      cost_estimate: instance.costEstimate,
      started_at: instance.startedAt,
      ended_at: null,
    })

    set(state => ({ instances: [...state.instances, instance] }))
    return instance
  },

  updateInstance: (id, updates) => {
    set(state => ({
      instances: state.instances.map(i => i.id === id ? { ...i, ...updates } : i)
    }))
  },

  updateInstanceDb: async (id, updates) => {
    const dbUpdates: Record<string, any> = {}
    if (updates.title !== undefined) dbUpdates.title = updates.title
    if (updates.status !== undefined) dbUpdates.status = updates.status
    if (updates.tokensUsed !== undefined) dbUpdates.tokens_used = updates.tokensUsed
    if (updates.costEstimate !== undefined) dbUpdates.cost_estimate = updates.costEstimate

    await window.electronAPI.dbInstancesUpdate(id, dbUpdates)
    get().updateInstance(id, updates)
  },

  removeInstance: async (id) => {
    await window.electronAPI.dbInstancesUpdate(id, { ended_at: new Date().toISOString() })
    set(state => ({ instances: state.instances.filter(i => i.id !== id) }))
  },

  getInstance: (id) => get().instances.find(i => i.id === id),

  getInstanceByNumber: (num) => get().instances.find(i => i.instanceNumber === num),

  appendTerminalOutput: (id, output) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id
          ? { ...i, terminalHistory: [...i.terminalHistory.slice(-500), output] }
          : i
      )
    }))
  },

  setStatus: (id, status) => {
    get().updateInstance(id, { status })
    // Don't await, fire and forget for performance
    window.electronAPI.dbInstancesUpdate(id, { status })
  },
}))
```

**Step 4: Create `src/stores/settingsStore.ts`**

```typescript
import { create } from 'zustand'

interface Settings {
  theme: 'dark' | 'light'
  questionsPanelVisible: boolean
  editorFontSize: number
  terminalFontSize: number
}

interface SettingsStore extends Settings {
  isLoading: boolean
  loadSettings: () => Promise<void>
  setSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>
  toggleQuestionsPanel: () => Promise<void>
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  theme: 'dark',
  questionsPanelVisible: true,
  editorFontSize: 13,
  terminalFontSize: 13,
  isLoading: true,

  loadSettings: async () => {
    const settings = await window.electronAPI.dbSettingsGetAll()
    set({
      theme: (settings.theme as 'dark' | 'light') || 'dark',
      questionsPanelVisible: settings.questions_panel_visible !== 'false',
      editorFontSize: parseInt(settings.editor_font_size || '13', 10),
      terminalFontSize: parseInt(settings.terminal_font_size || '13', 10),
      isLoading: false,
    })
  },

  setSetting: async (key, value) => {
    const dbKey = key.replace(/([A-Z])/g, '_$1').toLowerCase()
    await window.electronAPI.dbSettingsSet(dbKey, String(value))
    set({ [key]: value } as any)
  },

  toggleQuestionsPanel: async () => {
    const newValue = !get().questionsPanelVisible
    await get().setSetting('questionsPanelVisible', newValue)
  },
}))
```

**Step 5: Update `src/stores/index.ts`**

```typescript
export { useProjectStore } from './projectStore'
export { useInstanceStore } from './instanceStore'
export { useAppStore } from './appStore'
export { useSettingsStore } from './settingsStore'
```

**Step 6: Compile and verify**

Run: `npm run dev`
Expected: No TypeScript errors

**Step 7: Commit**

```bash
git add src/stores/ src/types/index.ts
git commit -m "feat: update Zustand stores to sync with SQLite"
```

---

### Task 5: Initialize Stores on App Load

**Files:**
- Modify: `src/App.tsx`

**Step 1: Update App.tsx to load data on mount**

```typescript
import { useEffect } from 'react'
import { useAppStore, useProjectStore, useInstanceStore, useSettingsStore } from '@/stores'
import { TopBar } from '@/components/shared/TopBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { InboxPanel } from '@/components/Overview/InboxPanel'
import { FocusView } from '@/components/Focus/FocusView'
import { NewInstanceModal } from '@/components/shared/NewInstanceModal'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)
  const newInstanceModalOpen = useAppStore(state => state.newInstanceModalOpen)

  const loadProjects = useProjectStore(state => state.loadProjects)
  const loadInstances = useInstanceStore(state => state.loadInstances)
  const loadSettings = useSettingsStore(state => state.loadSettings)
  const projectsLoading = useProjectStore(state => state.isLoading)
  const instancesLoading = useInstanceStore(state => state.isLoading)
  const settingsLoading = useSettingsStore(state => state.isLoading)

  useEffect(() => {
    loadProjects()
    loadInstances()
    loadSettings()
  }, [])

  const isLoading = projectsLoading || instancesLoading || settingsLoading

  if (isLoading) {
    return (
      <div className="app-loading">
        <div className="loading-spinner">Loading...</div>
      </div>
    )
  }

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

**Step 2: Add loading styles to `src/index.css`**

Add at end of file:
```css
.app-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background: #1e1e1e;
  color: #888;
}

.loading-spinner {
  font-size: 18px;
}
```

**Step 3: Verify app loads**

Run: `npm run dev`
Expected: App shows "Loading..." briefly then displays UI

**Step 4: Commit**

```bash
git add src/App.tsx src/index.css
git commit -m "feat: initialize stores from SQLite on app load"
```

---

## Phase 2: Navigation & Layout

### Task 6: Create Navigation Sidebar Component

**Files:**
- Create: `src/components/shared/NavSidebar.tsx`
- Create: `src/components/shared/NavSidebar.css`

**Step 1: Create NavSidebar.tsx**

```typescript
import { useAppStore, useInstanceStore, useSettingsStore } from '@/stores'
import './NavSidebar.css'

export function NavSidebar() {
  const viewMode = useAppStore(state => state.viewMode)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const focusInstance = useAppStore(state => state.focusInstance)
  const toggleSettings = useAppStore(state => state.toggleSettings)
  const instances = useInstanceStore(state => state.instances)

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'working': return '#81C784'
      case 'waiting': return '#FFB74D'
      case 'paused': return '#90A4AE'
      default: return '#666'
    }
  }

  return (
    <nav className="nav-sidebar">
      <div className="nav-top">
        <button
          className={`nav-btn ${viewMode === 'overview' ? 'active' : ''}`}
          onClick={returnToOverview}
          title="Overview"
        >
          <span className="nav-icon">▣</span>
        </button>

        <div className="nav-divider" />

        {instances.map(instance => (
          <button
            key={instance.id}
            className={`nav-btn ${focusedInstanceId === instance.id ? 'active' : ''}`}
            onClick={() => focusInstance(instance.id)}
            title={`#${instance.instanceNumber} ${instance.title}`}
          >
            <span className="nav-instance-num">{instance.instanceNumber}</span>
            <span
              className="nav-status-dot"
              style={{ backgroundColor: getStatusColor(instance.status) }}
            />
          </button>
        ))}
      </div>

      <div className="nav-bottom">
        <button
          className="nav-btn"
          onClick={toggleSettings}
          title="Settings"
        >
          <span className="nav-icon">⚙</span>
        </button>
      </div>
    </nav>
  )
}
```

**Step 2: Create NavSidebar.css**

```css
.nav-sidebar {
  width: 48px;
  background: #252526;
  border-right: 1px solid #3c3c3c;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 8px 0;
  flex-shrink: 0;
}

.nav-top,
.nav-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.nav-btn {
  width: 40px;
  height: 40px;
  border: none;
  background: transparent;
  color: #888;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  font-size: 14px;
  transition: all 0.15s ease;
}

.nav-btn:hover {
  background: #3c3c3c;
  color: #fff;
}

.nav-btn.active {
  background: #0e639c;
  color: #fff;
}

.nav-icon {
  font-size: 18px;
}

.nav-instance-num {
  font-weight: 600;
  font-size: 14px;
}

.nav-status-dot {
  position: absolute;
  bottom: 6px;
  right: 6px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.nav-divider {
  width: 24px;
  height: 1px;
  background: #3c3c3c;
  margin: 4px 0;
}
```

**Step 3: Commit**

```bash
git add src/components/shared/NavSidebar.tsx src/components/shared/NavSidebar.css
git commit -m "feat: add navigation sidebar component"
```

---

### Task 7: Configure Frameless Window

**Files:**
- Modify: `electron/main.ts`

**Step 1: Update BrowserWindow config in main.ts**

Find the `createWindow` function and update the BrowserWindow options:

```typescript
function createWindow() {
  logger.info('Creating main window')

  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    titleBarStyle: 'hidden',
    trafficLightPosition: { x: 12, y: 12 },
    backgroundColor: '#1e1e1e',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  // ... rest of function
}
```

**Step 2: Recompile electron**

Run: `npx tsc -p tsconfig.node.json`
Expected: No errors

**Step 3: Commit**

```bash
git add electron/main.ts
git commit -m "feat: configure frameless window with hidden titlebar"
```

---

### Task 8: Create New Top Bar (Compact)

**Files:**
- Rewrite: `src/components/shared/TopBar.tsx`
- Rewrite: `src/components/shared/TopBar.css`

**Step 1: Rewrite TopBar.tsx**

```typescript
import { useAppStore, useSettingsStore } from '@/stores'
import './TopBar.css'

export function TopBar() {
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const toggleQuestionsPanel = useSettingsStore(state => state.toggleQuestionsPanel)
  const questionsPanelVisible = useSettingsStore(state => state.questionsPanelVisible)

  return (
    <header className="top-bar">
      <div className="top-bar-drag-region" />

      <div className="top-bar-left">
        <span className="app-title">◉ Jenklaud</span>
      </div>

      <div className="top-bar-right">
        <button className="top-bar-btn primary" onClick={toggleNewInstanceModal}>
          + New
        </button>
        <button
          className={`top-bar-btn ${questionsPanelVisible ? 'active' : ''}`}
          onClick={toggleQuestionsPanel}
        >
          Questions
        </button>
      </div>
    </header>
  )
}
```

**Step 2: Rewrite TopBar.css**

```css
.top-bar {
  height: 44px;
  background: #1e1e1e;
  border-bottom: 1px solid #3c3c3c;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  padding-left: 80px; /* Space for traffic lights */
  position: relative;
  -webkit-app-region: drag;
}

.top-bar-drag-region {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 100%;
  -webkit-app-region: drag;
}

.top-bar-left,
.top-bar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  -webkit-app-region: no-drag;
  z-index: 1;
}

.app-title {
  font-weight: 600;
  font-size: 14px;
  color: #fff;
}

.top-bar-btn {
  padding: 6px 12px;
  border: none;
  background: #3c3c3c;
  color: #ccc;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: all 0.15s ease;
}

.top-bar-btn:hover {
  background: #4c4c4c;
  color: #fff;
}

.top-bar-btn.primary {
  background: #0e639c;
  color: #fff;
}

.top-bar-btn.primary:hover {
  background: #1177bb;
}

.top-bar-btn.active {
  background: #0e639c;
  color: #fff;
}
```

**Step 3: Commit**

```bash
git add src/components/shared/TopBar.tsx src/components/shared/TopBar.css
git commit -m "feat: redesign top bar for frameless window"
```

---

### Task 9: Create Bottom Status Bar

**Files:**
- Create: `src/components/shared/StatusBar.tsx`
- Create: `src/components/shared/StatusBar.css`

**Step 1: Create StatusBar.tsx**

```typescript
import { useInstanceStore } from '@/stores'
import './StatusBar.css'

export function StatusBar() {
  const instances = useInstanceStore(state => state.instances)

  const totalTokens = instances.reduce((sum, i) => sum + i.tokensUsed, 0)
  const totalCost = instances.reduce((sum, i) => sum + i.costEstimate, 0)
  const workingCount = instances.filter(i => i.status === 'working').length
  const waitingCount = instances.filter(i => i.status === 'waiting').length

  return (
    <footer className="status-bar">
      <div className="status-item">
        <span className="status-label">{instances.length}</span>
        <span className="status-text">instances</span>
      </div>

      {workingCount > 0 && (
        <div className="status-item">
          <span className="status-dot working" />
          <span className="status-label">{workingCount}</span>
          <span className="status-text">working</span>
        </div>
      )}

      {waitingCount > 0 && (
        <div className="status-item">
          <span className="status-dot waiting" />
          <span className="status-label">{waitingCount}</span>
          <span className="status-text">waiting</span>
        </div>
      )}

      <div className="status-spacer" />

      <div className="status-item">
        <span className="status-label">{(totalTokens / 1000).toFixed(1)}k</span>
        <span className="status-text">tokens</span>
      </div>

      <div className="status-item">
        <span className="status-label">${totalCost.toFixed(2)}</span>
      </div>
    </footer>
  )
}
```

**Step 2: Create StatusBar.css**

```css
.status-bar {
  height: 24px;
  background: #007acc;
  display: flex;
  align-items: center;
  padding: 0 12px;
  gap: 16px;
  font-size: 12px;
  color: #fff;
  flex-shrink: 0;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.status-label {
  font-weight: 600;
}

.status-text {
  opacity: 0.8;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.status-dot.working {
  background: #81C784;
}

.status-dot.waiting {
  background: #FFB74D;
}

.status-spacer {
  flex: 1;
}
```

**Step 3: Commit**

```bash
git add src/components/shared/StatusBar.tsx src/components/shared/StatusBar.css
git commit -m "feat: add bottom status bar component"
```

---

### Task 10: Update App Layout with New Components

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/index.css`

**Step 1: Update App.tsx with new layout**

```typescript
import { useEffect } from 'react'
import { useAppStore, useProjectStore, useInstanceStore, useSettingsStore } from '@/stores'
import { NavSidebar } from '@/components/shared/NavSidebar'
import { TopBar } from '@/components/shared/TopBar'
import { StatusBar } from '@/components/shared/StatusBar'
import { OverviewGrid } from '@/components/Overview/OverviewGrid'
import { QuestionsPanel } from '@/components/Overview/QuestionsPanel'
import { FocusView } from '@/components/Focus/FocusView'
import { NewInstanceModal } from '@/components/shared/NewInstanceModal'
import './index.css'

export default function App() {
  const viewMode = useAppStore(state => state.viewMode)
  const newInstanceModalOpen = useAppStore(state => state.newInstanceModalOpen)
  const questionsPanelVisible = useSettingsStore(state => state.questionsPanelVisible)

  const loadProjects = useProjectStore(state => state.loadProjects)
  const loadInstances = useInstanceStore(state => state.loadInstances)
  const loadSettings = useSettingsStore(state => state.loadSettings)
  const projectsLoading = useProjectStore(state => state.isLoading)
  const instancesLoading = useInstanceStore(state => state.isLoading)
  const settingsLoading = useSettingsStore(state => state.isLoading)

  useEffect(() => {
    loadProjects()
    loadInstances()
    loadSettings()
  }, [])

  const isLoading = projectsLoading || instancesLoading || settingsLoading

  if (isLoading) {
    return (
      <div className="app-loading">
        <div className="loading-spinner">Loading...</div>
      </div>
    )
  }

  return (
    <div className="app-container">
      <NavSidebar />
      <div className="app-main">
        <TopBar />
        <div className="app-content">
          {viewMode === 'focus' ? (
            <FocusView />
          ) : (
            <>
              <OverviewGrid />
              {questionsPanelVisible && <QuestionsPanel />}
            </>
          )}
        </div>
        <StatusBar />
      </div>
      {newInstanceModalOpen && <NewInstanceModal />}
    </div>
  )
}
```

**Step 2: Update index.css**

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
  height: 100vh;
}

.app-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}

.app-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.app-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background: #1e1e1e;
  color: #888;
}

.loading-spinner {
  font-size: 18px;
}
```

**Step 3: Rename InboxPanel to QuestionsPanel**

```bash
mv src/components/Overview/InboxPanel.tsx src/components/Overview/QuestionsPanel.tsx
mv src/components/Overview/InboxPanel.css src/components/Overview/QuestionsPanel.css
```

**Step 4: Update QuestionsPanel.tsx** (rename references)

Change class names from `inbox-*` to `questions-*` and update imports.

**Step 5: Verify layout**

Run: `npm run dev`
Expected: App shows nav sidebar on left, top bar, main content, status bar at bottom

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: implement new app layout with nav sidebar and status bar"
```

---

## Phase 3: Overview Mode (Continued in next tasks...)

### Task 11: Redesign Instance Tiles

**Files:**
- Rewrite: `src/components/Overview/InstanceTile.tsx`
- Rewrite: `src/components/Overview/InstanceTile.css`

**Step 1: Rewrite InstanceTile.tsx**

```typescript
import { useState } from 'react'
import { useProjectStore, useInstanceStore, useAppStore } from '@/stores'
import { Instance } from '@/types'
import './InstanceTile.css'

interface InstanceTileProps {
  instance: Instance
}

export function InstanceTile({ instance }: InstanceTileProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editTitle, setEditTitle] = useState(instance.title)

  const project = useProjectStore(state => state.getProject(instance.projectId))
  const updateInstanceDb = useInstanceStore(state => state.updateInstanceDb)
  const focusInstance = useAppStore(state => state.focusInstance)

  if (!project) return null

  const shortenPath = (fullPath: string) => {
    const home = process.env.HOME || ''
    return fullPath.replace(home, '~')
  }

  const handleSaveTitle = async () => {
    if (editTitle.trim()) {
      await updateInstanceDb(instance.id, { title: editTitle.trim() })
    }
    setIsEditing(false)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSaveTitle()
    if (e.key === 'Escape') {
      setEditTitle(instance.title)
      setIsEditing(false)
    }
  }

  const lastOutput = instance.terminalHistory.slice(-5).join('')

  const statusColor = {
    working: '#81C784',
    waiting: '#FFB74D',
    paused: '#90A4AE',
    starting: '#64B5F6',
    stopped: '#666',
  }[instance.status] || '#666'

  return (
    <div className="instance-tile" onClick={() => focusInstance(instance.id)}>
      <div className="tile-header">
        <div className="tile-color" style={{ backgroundColor: project.color }} />
        <div className="tile-title-row">
          {isEditing ? (
            <input
              className="tile-title-input"
              value={editTitle}
              onChange={e => setEditTitle(e.target.value)}
              onBlur={handleSaveTitle}
              onKeyDown={handleKeyDown}
              onClick={e => e.stopPropagation()}
              autoFocus
            />
          ) : (
            <>
              <span className="tile-number" style={{ color: project.color }}>
                #{instance.instanceNumber}
              </span>
              <span className="tile-title">{instance.title}</span>
              <button
                className="tile-edit-btn"
                onClick={e => {
                  e.stopPropagation()
                  setIsEditing(true)
                }}
              >
                ✎
              </button>
            </>
          )}
        </div>
        <div className="tile-path">{shortenPath(project.path)}</div>
      </div>

      <div className="tile-preview">
        <pre className="tile-terminal">{lastOutput || '$ claude\n> Starting...'}</pre>
      </div>

      <div className="tile-footer">
        <span className="tile-status">
          <span className="tile-status-dot" style={{ backgroundColor: statusColor }} />
          {instance.status}
        </span>
        <span className="tile-tokens">{(instance.tokensUsed / 1000).toFixed(1)}k</span>
      </div>
    </div>
  )
}
```

**Step 2: Create InstanceTile.css**

```css
.instance-tile {
  background: #252526;
  border-radius: 8px;
  border: 1px solid #3c3c3c;
  overflow: hidden;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  transition: border-color 0.15s ease;
}

.instance-tile:hover {
  border-color: #0e639c;
}

.tile-header {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  position: relative;
}

.tile-color {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 4px;
}

.tile-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding-left: 8px;
}

.tile-number {
  font-weight: 700;
  font-size: 14px;
}

.tile-title {
  font-weight: 500;
  font-size: 14px;
  color: #fff;
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tile-edit-btn {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.instance-tile:hover .tile-edit-btn {
  opacity: 1;
}

.tile-edit-btn:hover {
  background: #3c3c3c;
  color: #fff;
}

.tile-title-input {
  flex: 1;
  background: #1e1e1e;
  border: 1px solid #0e639c;
  color: #fff;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 14px;
  outline: none;
}

.tile-path {
  font-size: 11px;
  color: #888;
  padding-left: 8px;
}

.tile-preview {
  flex: 1;
  padding: 0 12px;
  overflow: hidden;
}

.tile-terminal {
  font-family: 'SF Mono', Monaco, monospace;
  font-size: 11px;
  color: #aaa;
  white-space: pre-wrap;
  word-break: break-all;
  line-height: 1.4;
  max-height: 80px;
  overflow: hidden;
}

.tile-footer {
  padding: 8px 12px;
  background: #1e1e1e;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
}

.tile-status {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #888;
}

.tile-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.tile-tokens {
  color: #888;
}
```

**Step 3: Commit**

```bash
git add src/components/Overview/InstanceTile.tsx src/components/Overview/InstanceTile.css
git commit -m "feat: redesign instance tiles with editable title"
```

---

### Task 12: Update Questions Panel

**Files:**
- Rewrite: `src/components/Overview/QuestionsPanel.tsx`
- Rewrite: `src/components/Overview/QuestionsPanel.css`

**Step 1: Rewrite QuestionsPanel.tsx**

```typescript
import { useMemo } from 'react'
import { useInstanceStore, useAppStore } from '@/stores'
import './QuestionsPanel.css'

export function QuestionsPanel() {
  const instances = useInstanceStore(state => state.instances)
  const focusInstance = useAppStore(state => state.focusInstance)

  const waitingInstances = useMemo(() => {
    return instances.filter(i => i.status === 'waiting')
  }, [instances])

  const getTimeAgo = (date: string) => {
    const mins = Math.floor((Date.now() - new Date(date).getTime()) / 60000)
    if (mins < 1) return 'just now'
    if (mins === 1) return '1m ago'
    return `${mins}m ago`
  }

  return (
    <aside className="questions-panel">
      <div className="questions-header">
        QUESTIONS ({waitingInstances.length})
      </div>

      <div className="questions-list">
        {waitingInstances.length === 0 ? (
          <div className="questions-empty">No pending questions</div>
        ) : (
          waitingInstances.map(instance => (
            <div
              key={instance.id}
              className="question-item"
              onClick={() => focusInstance(instance.id)}
            >
              <div className="question-header">
                <span className="question-instance">#{instance.instanceNumber}</span>
                <span className="question-title">{instance.title}</span>
              </div>
              <div className="question-preview">
                {instance.terminalHistory.slice(-1)[0] || 'Waiting for response...'}
              </div>
              <div className="question-time">{getTimeAgo(instance.startedAt)}</div>
            </div>
          ))
        )}
      </div>
    </aside>
  )
}
```

**Step 2: Create QuestionsPanel.css**

```css
.questions-panel {
  width: 280px;
  background: #252526;
  border-left: 1px solid #3c3c3c;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.questions-header {
  padding: 12px 16px;
  font-size: 11px;
  font-weight: 600;
  color: #888;
  letter-spacing: 0.5px;
  border-bottom: 1px solid #3c3c3c;
}

.questions-list {
  flex: 1;
  overflow-y: auto;
}

.questions-empty {
  padding: 24px 16px;
  text-align: center;
  color: #666;
  font-size: 13px;
}

.question-item {
  padding: 12px 16px;
  border-bottom: 1px solid #3c3c3c;
  cursor: pointer;
  transition: background 0.15s ease;
}

.question-item:hover {
  background: #2d2d2d;
}

.question-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.question-instance {
  font-weight: 600;
  color: #0e639c;
  font-size: 12px;
}

.question-title {
  font-size: 13px;
  color: #fff;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.question-preview {
  font-size: 12px;
  color: #aaa;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  margin-bottom: 6px;
}

.question-time {
  font-size: 11px;
  color: #666;
}
```

**Step 3: Commit**

```bash
git add src/components/Overview/QuestionsPanel.tsx src/components/Overview/QuestionsPanel.css
git commit -m "feat: update questions panel design"
```

---

## Phase 4: Focus Mode (Editor & File Tree)

### Task 13-20: Focus Mode Implementation

*Due to length constraints, the remaining tasks follow the same pattern:*

- **Task 13**: Create FileTree component using react-arborist
- **Task 14**: Create Editor component using CodeMirror 6
- **Task 15**: Create EditorTabs component for file tabs
- **Task 16**: Update Terminal component for Focus mode
- **Task 17**: Create resizable panels layout
- **Task 18**: Update FocusView to integrate all components
- **Task 19**: Add file watching for auto-reload
- **Task 20**: Add keyboard shortcuts

---

## Phase 5: Polish

### Task 21-25: Polish & Edge Cases

- **Task 21**: Settings panel/modal
- **Task 22**: Error boundaries
- **Task 23**: Keyboard navigation (Cmd+1-9 for instances)
- **Task 24**: Persist window size/position
- **Task 25**: Final testing & cleanup

---

## Verification Checklist

After completing all tasks, verify:

- [ ] App starts without errors
- [ ] Projects persist after restart
- [ ] Instances show in nav sidebar
- [ ] Clicking instance number switches to Focus mode
- [ ] Overview button returns to Overview
- [ ] Questions panel toggles
- [ ] Instance tiles show live terminal output
- [ ] Titles are editable
- [ ] File tree loads project files
- [ ] Editor opens files with syntax highlighting
- [ ] Terminal is interactive
- [ ] Status bar shows aggregate stats
- [ ] Settings persist after restart
