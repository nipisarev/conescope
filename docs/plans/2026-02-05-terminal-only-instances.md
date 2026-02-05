# Terminal-Only Instances Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add standalone terminal instances (no project association) with nullable `projectId`, own color, and terminal-only focus mode.

**Architecture:** Extend Instance type with `type: 'project' | 'terminal'` and `color` fields. Make `projectId` nullable. Terminal instances spawn PTY at `$HOME`, run `claude`, and render in Focus mode with only the terminal panel (no folder/editor toggles). Overview tiles show "Shell" + pwd.

**Tech Stack:** SQLite migration, TypeScript, React/Zustand, node-pty, xterm.js

---

### Task 1: Database Migration — nullable project_id, add type + color columns

**Files:**
- Modify: `electron/database.ts:37-101` (migrate method)

**Step 1: Add migration for new columns**

In `database.ts`, add these migration blocks after the existing `terminal_history` migration (line 93-98):

```typescript
// Migration: Make project_id nullable (SQLite can't ALTER column, but new inserts can pass null)
// For new instances, we just allow null in the INSERT statement

// Migration: Add type column to instances
try {
  this.db.exec("ALTER TABLE instances ADD COLUMN type TEXT NOT NULL DEFAULT 'project'")
  logger.info('Added type column to instances')
} catch (e) {
  // Column already exists
}

// Migration: Add color column to instances
try {
  this.db.exec("ALTER TABLE instances ADD COLUMN color TEXT")
  logger.info('Added color column to instances')
} catch (e) {
  // Column already exists
}
```

**Step 2: Update insertInstance to include type and color**

Replace the `insertInstance` method:

```typescript
insertInstance(instance: any): void {
  if (!this.db) throw new Error('Database not initialized')
  this.db.prepare(`
    INSERT INTO instances (id, project_id, title, status, instance_number, tokens_used, cost_estimate, started_at, type, color)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `).run(
    instance.id,
    instance.project_id,
    instance.title,
    instance.status,
    instance.instance_number,
    instance.tokens_used,
    instance.cost_estimate,
    instance.started_at,
    instance.type || 'project',
    instance.color || null
  )
}
```

**Step 3: Update getPendingQuestions to LEFT JOIN for terminal instances**

Replace the query in `getPendingQuestions`:

```typescript
getPendingQuestions(): any[] {
  if (!this.db) throw new Error('Database not initialized')
  return this.db.prepare(`
    SELECT q.*, i.title as instance_title, p.display_name as project_name, p.color as project_color
    FROM questions q
    JOIN instances i ON q.instance_id = i.id
    LEFT JOIN projects p ON i.project_id = p.id
    WHERE q.answered_at IS NULL
    ORDER BY q.asked_at ASC
  `).all()
}
```

**Step 4: Verify**

Run: `npx tsc --noEmit -p tsconfig.node.json`
Expected: No errors

**Step 5: Commit**

```
feat: add type and color columns to instances table
```

---

### Task 2: Update TypeScript types

**Files:**
- Modify: `src/types/index.ts`

**Step 1: Update Instance interface**

```typescript
export type InstanceType = 'project' | 'terminal'

export interface Instance {
  id: string
  projectId: string | null
  title: string
  instanceNumber: number
  status: InstanceStatus
  tokensUsed: number
  costEstimate: number
  startedAt: string
  terminalHistory: string[]
  type: InstanceType
  color: string | null
}
```

**Step 2: Verify**

Run: `npx tsc --noEmit`
Expected: Many errors (expected — other files reference `projectId` as non-null). We'll fix them in subsequent tasks.

**Step 3: Commit**

```
feat: update Instance type with nullable projectId, type, color
```

---

### Task 3: Update instanceStore — add createTerminalInstance

**Files:**
- Modify: `src/stores/instanceStore.ts`

**Step 1: Update loadInstances mapper to include new fields**

```typescript
loadInstances: async () => {
  const dbInstances = await window.electronAPI.dbInstancesGetAll()
  const instances: Instance[] = dbInstances.map(i => ({
    id: i.id,
    projectId: i.project_id || null,
    title: i.title || '',
    instanceNumber: i.instance_number,
    status: 'starting' as InstanceStatus,
    tokensUsed: i.tokens_used,
    costEstimate: i.cost_estimate,
    startedAt: i.started_at,
    terminalHistory: [],
    type: (i.type || 'project') as InstanceType,
    color: i.color || null,
  }))
  set({ instances, isLoading: false })
},
```

**Step 2: Update createInstance to pass type/color**

```typescript
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
    type: 'project',
    color: null,
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
    type: instance.type,
    color: instance.color,
  })

  set(state => ({ instances: [...state.instances, instance] }))
  return instance
},
```

**Step 3: Add createTerminalInstance method**

Add to InstanceStore interface:
```typescript
createTerminalInstance: () => Promise<Instance>
```

Add implementation:
```typescript
createTerminalInstance: async () => {
  const instanceNumber = await window.electronAPI.dbInstancesGetNextNumber()
  // Pick a color from the palette using instance count
  const currentCount = get().instances.length
  const color = PROJECT_COLORS[currentCount % PROJECT_COLORS.length]

  const instance: Instance = {
    id: uuid(),
    projectId: null,
    title: 'Shell',
    instanceNumber,
    status: 'starting',
    tokensUsed: 0,
    costEstimate: 0,
    startedAt: new Date().toISOString(),
    terminalHistory: [],
    type: 'terminal',
    color,
  }

  await window.electronAPI.dbInstancesInsert({
    id: instance.id,
    project_id: null,
    title: instance.title,
    status: instance.status,
    instance_number: instance.instanceNumber,
    tokens_used: instance.tokensUsed,
    cost_estimate: instance.costEstimate,
    started_at: instance.startedAt,
    type: instance.type,
    color: instance.color,
  })

  set(state => ({ instances: [...state.instances, instance] }))
  return instance
},
```

Add `PROJECT_COLORS` import at top:
```typescript
import { Instance, InstanceStatus, InstanceType, PROJECT_COLORS } from '@/types'
```

**Step 4: Update restoreInstance signature**

The method needs to handle null projectPath for terminal instances:
```typescript
restoreInstance: async (id: string, projectPath: string | null) => {
  const cwd = projectPath || process.env.HOME || '~'
  await window.electronAPI.createInstance(id, cwd)
},
```

Wait — `process.env.HOME` isn't available in renderer. We need a different approach. The main process already knows HOME. Let's pass an empty string and handle in the IPC handler. Actually simpler: just pass `$HOME` path from the renderer side or use a hardcoded `~` expansion approach.

Better approach: Add a new preload/IPC method `getHomePath`, OR just hardcode the homedir resolution in `instance-manager.ts`. Let's handle this in Task 4.

For now, update restoreInstance:
```typescript
restoreInstance: async (id: string, projectPath: string) => {
  await window.electronAPI.createInstance(id, projectPath)
},
```

We'll pass the actual home dir from App.tsx's restore logic.

**Step 5: Verify**

Run: `npx tsc --noEmit`

**Step 6: Commit**

```
feat: add createTerminalInstance to instanceStore
```

---

### Task 4: Backend — support terminal instances in instance-manager + IPC

**Files:**
- Modify: `electron/instance-manager.ts:46-54` (createInstance validation)
- Modify: `electron/main.ts:39-52` (IPC handler)
- Modify: `electron/preload.ts` (add getHomePath)

**Step 1: Add getHomePath IPC**

In `main.ts`, add:
```typescript
ipcMain.handle('app:getHomePath', () => {
  return app.getPath('home')
})
```

In `preload.ts`, add inside the `electronAPI` object:
```typescript
getHomePath: () => ipcRenderer.invoke('app:getHomePath'),
```

**Step 2: Update instance-manager createInstance to allow home dir**

In `instance-manager.ts`, the `createInstance` method currently validates that `projectPath` exists and is a directory. This already works for `$HOME`. No changes needed if we pass the actual home path.

**Step 3: Verify**

Run: `npx tsc --noEmit -p tsconfig.node.json`

**Step 4: Commit**

```
feat: add getHomePath IPC for terminal instances
```

---

### Task 5: NewInstanceModal — add Terminal button at bottom

**Files:**
- Modify: `src/components/shared/NewInstanceModal.tsx`
- Modify: `src/components/shared/NewInstanceModal.css`

**Step 1: Add handleCreateTerminal handler**

```typescript
const createTerminalInstance = useInstanceStore(state => state.createTerminalInstance)

const handleCreateTerminal = async () => {
  const homePath = await window.electronAPI.getHomePath()
  const instance = await createTerminalInstance()
  await window.electronAPI.createInstance(instance.id, homePath)
  toggleNewInstanceModal()
  focusInstance(instance.id)
}
```

**Step 2: Add Terminal button section at bottom of modal-content**

After the Browse section, add:
```tsx
<div className="section">
  <div className="section-divider" />
  <button className="terminal-btn" onClick={handleCreateTerminal}>
    <TerminalTag width={16} height={16} />
    <span>Open Terminal</span>
  </button>
</div>
```

Import `TerminalTag` from `iconoir-react`.

**Step 3: Add CSS for terminal button**

```css
.section-divider {
  height: 1px;
  background: #3c3c3c;
  margin-bottom: 16px;
}

.terminal-btn {
  width: 100%;
  padding: 12px;
  background: #2d2d2d;
  border: 1px solid #3c3c3c;
  border-radius: 6px;
  color: #d4d4d4;
  font-size: 13px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: background 0.2s, border-color 0.2s;
}

.terminal-btn:hover {
  background: #3c3c3c;
  border-color: #0e639c;
}
```

**Step 4: Verify**

Run: `npx tsc --noEmit`

**Step 5: Commit**

```
feat: add Terminal button to NewInstanceModal
```

---

### Task 6: FocusView — terminal-only mode (no folder/editor)

**Files:**
- Modify: `src/components/Focus/FocusView.tsx`

**Step 1: Detect terminal instance and render terminal-only**

The key change: when `instance.type === 'terminal'`, skip FileTree and Editor entirely, render terminal at 100%.

Update the guard check (line 106):
```typescript
if (!instance) {
  return (
    <div className="focus-view-empty">
      <p>No instance selected</p>
    </div>
  )
}

const isTerminalOnly = instance.type === 'terminal'
```

Remove the `!project` check from the guard — terminal instances won't have a project.

Update the return JSX: wrap folder/editor sections in `!isTerminalOnly` checks:

```tsx
return (
  <div className="focus-view">
    {!isTerminalOnly && folderPanelVisible && (
      <>
        <div className="focus-sidebar" style={{ width: sidebarWidth }}>
          <FileTree projectPath={project!.path} />
        </div>
        <div
          className={`focus-sidebar-resize ${isResizing && resizeType === 'sidebar' ? 'active' : ''}`}
          onMouseDown={handleSidebarResizeStart}
        />
      </>
    )}

    <div className="focus-main">
      {!isTerminalOnly && editorPanelVisible && (
        <div className="focus-editor-area" style={{ flex: terminalPanelVisible ? 1 : undefined }}>
          <EditorTabs />
          <Editor file={activeFile} />
        </div>
      )}

      {!isTerminalOnly && editorPanelVisible && terminalPanelVisible && (
        <div
          className={`focus-resize-handle ${isResizing && resizeType === 'terminal' ? 'active' : ''}`}
          onMouseDown={handleTerminalResizeStart}
        />
      )}

      <div
        className="focus-terminal"
        style={{ height: isTerminalOnly ? '100%' : (editorPanelVisible ? terminalHeight : '100%') }}
      >
        <TerminalTabs instanceId={instance.id} />
        <div className="focus-terminal-content">
          {activeTab?.type === 'claude' ? (
            <Terminal instanceId={instance.id} onInput={handleInput} />
          ) : activeTab?.type === 'shell' ? (
            <ShellTerminal tabId={activeTab.id} />
          ) : null}
        </div>
      </div>

      {!isTerminalOnly && !editorPanelVisible && !terminalPanelVisible && (
        <div className="focus-empty-state">
          <p>All panels hidden</p>
          <p className="focus-empty-hint">Use the panel toggles in the activity bar</p>
        </div>
      )}
    </div>
  </div>
)
```

**Step 2: Verify**

Run: `npx tsc --noEmit`

**Step 3: Commit**

```
feat: terminal-only focus mode without folder/editor panels
```

---

### Task 7: NavSidebar — hide panel toggles for terminal instances, use instance color

**Files:**
- Modify: `src/components/shared/NavSidebar.tsx`

**Step 1: Get focused instance and determine if terminal-only**

Add at top of component:
```typescript
const focusedInstance = useInstanceStore(state =>
  focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
)
const isTerminalInstance = focusedInstance?.type === 'terminal'
```

**Step 2: Conditionally render panel toggles**

Replace the focus mode section (lines 70-97):
```tsx
{isFocusMode ? (
  <>
    {!isTerminalInstance && (
      <>
        <div className="activity-bar-divider" />
        <div className="activity-bar-section">
          <button
            className={`activity-btn panel-toggle ${sessionState.folderPanelVisible ? 'panel-active' : ''}`}
            onClick={toggleFolderPanel}
            title="Toggle Folder Panel"
          >
            <Folder width={14} height={14} />
          </button>
          <button
            className={`activity-btn panel-toggle ${sessionState.editorPanelVisible ? 'panel-active' : ''}`}
            onClick={toggleEditorPanel}
            title="Toggle Editor Panel"
          >
            <PageEdit width={14} height={14} />
          </button>
          <button
            className={`activity-btn panel-toggle ${sessionState.terminalPanelVisible ? 'panel-active' : ''}`}
            onClick={toggleTerminalPanel}
            title="Toggle Terminal Panel"
          >
            <TerminalTag width={14} height={14} />
          </button>
        </div>
      </>
    )}
  </>
) : (
  /* Overview mode: instance numbers */
  ...
)}
```

**Step 3: Update overview mode instance colors**

In the overview mode section, update color resolution to use instance color for terminal instances:
```typescript
const color = instance.type === 'terminal'
  ? instance.color || '#888'
  : (getProject(instance.projectId!)?.color || '#888')
```

**Step 4: Verify**

Run: `npx tsc --noEmit`

**Step 5: Commit**

```
feat: hide panel toggles for terminal instances in NavSidebar
```

---

### Task 8: InstancePopup — show title + path for all instances

**Files:**
- Modify: `src/components/shared/InstancePopup.tsx`
- Modify: `src/components/shared/InstancePopup.css`

**Step 1: Update popup to show path info**

```tsx
export function InstancePopup({ onClose, onMouseEnter, onMouseLeave }: InstancePopupProps) {
  const instances = useInstanceStore(state => state.instances)
  const getProject = useProjectStore(state => state.getProject)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const focusInstance = useAppStore(state => state.focusInstance)

  const handleClick = (instanceId: string) => {
    focusInstance(instanceId)
    onClose()
  }

  const sortedInstances = [...instances].sort((a, b) => a.instanceNumber - b.instanceNumber)

  const shortenPath = (fullPath: string) => {
    return fullPath.replace(/^\/Users\/[^/]+/, '~')
  }

  return (
    <div className="instance-popup" onMouseEnter={onMouseEnter} onMouseLeave={onMouseLeave}>
      {sortedInstances.map((instance, index) => {
        const project = instance.projectId ? getProject(instance.projectId) : null
        const color = instance.type === 'terminal'
          ? instance.color || '#888'
          : (project?.color || '#888')
        const isActive = focusedInstanceId === instance.id
        const displayNumber = index + 1
        const pathStr = project ? shortenPath(project.path) : '~'

        return (
          <button
            key={instance.id}
            className={`instance-popup-btn ${isActive ? 'active' : ''}`}
            onClick={() => handleClick(instance.id)}
          >
            <span className="popup-number" style={{ color }}>{displayNumber}</span>
            <div className="popup-info">
              <span className="popup-title">{instance.title}</span>
              <span className="popup-path">{pathStr}</span>
            </div>
          </button>
        )
      })}
    </div>
  )
}
```

**Step 2: Add CSS for path display**

```css
.popup-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  overflow: hidden;
}

.popup-path {
  font-size: 10px;
  color: #808080;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
```

**Step 3: Verify**

Run: `npx tsc --noEmit`

**Step 4: Commit**

```
feat: show title and path in InstancePopup for all instances
```

---

### Task 9: InstanceTile — "Shell" title + pwd, use instance color

**Files:**
- Modify: `src/components/Overview/InstanceTile.tsx`

**Step 1: Handle terminal instances**

Update the project resolution and color logic:

```typescript
const project = instance.projectId
  ? useProjectStore(state => state.getProject(instance.projectId!))
  : null

const tileColor = instance.type === 'terminal'
  ? instance.color || '#888'
  : (project?.color || '#888')
```

Remove the early return `if (!project) return null` — terminal instances won't have a project.

Update `shortenPath` usage: for terminal instances, show `~` (the home dir).

In the tile-meta span:
```tsx
<span className="tile-path">
  {project ? shortenPath(project.path) : '~'}
</span>
```

Update color references from `project.color` to `tileColor`:
```tsx
<span className="tile-number" style={{ color: tileColor }}>
  #{getDisplayNumber(instance.id)}
</span>
<span className="tile-title" style={{ color: tileColor }}>{instance.title}</span>
```

**Step 2: Verify**

Run: `npx tsc --noEmit`

**Step 3: Commit**

```
feat: terminal instance support in OverviewTile
```

---

### Task 10: App.tsx — restore terminal instances on app restart

**Files:**
- Modify: `src/App.tsx:60-76`

**Step 1: Update instance restore logic**

```typescript
// Restore each instance's PTY process
instances.forEach(async (instance) => {
  try {
    if (instance.type === 'terminal') {
      const homePath = await window.electronAPI.getHomePath()
      await restoreInstance(instance.id, homePath)
    } else {
      const project = getProject(instance.projectId!)
      if (project) {
        await restoreInstance(instance.id, project.path)
      }
    }
  } catch (err) {
    console.error('Failed to restore instance:', instance.id, err)
  }
})
```

**Step 2: Verify**

Run: `npx tsc --noEmit`

**Step 3: Commit**

```
feat: restore terminal instances on app restart
```

---

### Task 11: Fix TypeScript errors across codebase

After all changes, there will be remaining TS errors where code assumes `projectId` is always a string. Fix null checks:

**Files to audit:**
- Any file that does `instance.projectId` without null check
- Any file that does `getProject(instance.projectId)` — needs `instance.projectId ? getProject(instance.projectId) : null`

**Step 1: Fix all remaining TS errors**

Run `npx tsc --noEmit` and fix each error.

**Step 2: Verify clean build**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 3: Commit**

```
fix: null safety for terminal instances across codebase
```

---

### Task 12: Manual smoke test

**Step 1: Run the app**

```bash
npm run dev
```

**Step 2: Test creating a terminal instance**

1. Open New Instance modal
2. Click "Open Terminal" at bottom
3. Verify: terminal opens in focus mode, no folder/editor toggles in sidebar
4. Verify: claude starts running in terminal

**Step 3: Test overview tile**

1. Return to overview
2. Verify: terminal tile shows "Shell" title with `~` path and assigned color

**Step 4: Test instance popup**

1. Go to focus mode on any instance
2. Hover overview icon
3. Verify: all instances show title + path

**Step 5: Test creating a project instance still works**

1. Create a normal project instance
2. Verify: folder/editor/terminal toggles appear
3. Verify: FileTree loads correctly

**Step 6: Test app restart**

1. Quit and relaunch
2. Verify: both project and terminal instances restore correctly
