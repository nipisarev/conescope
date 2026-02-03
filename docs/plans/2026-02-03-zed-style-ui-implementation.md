# Zed-Style UI Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Redesign navigation to Zed-style bottom activity bar with panel toggles, minimal borders, and streamlined controls.

**Architecture:** Bottom activity bar replaces left sidebar. Panel visibility state in settingsStore. New modal for close confirmation. Hover popup for instance navigation in focus mode.

**Tech Stack:** React, Zustand, CSS

---

### Task 1: Add Panel Visibility State to Settings Store

**Files:**
- Modify: `src/stores/settingsStore.ts`

**Step 1: Add panel visibility to SessionState interface**

In `src/stores/settingsStore.ts`, update the `SessionState` interface:

```typescript
interface SessionState {
  viewMode: 'overview' | 'focus'
  focusedInstanceId: string | null
  terminalHeight: number
  sidebarWidth: number
  instances: Record<string, InstanceSession>
  // Add these:
  folderPanelVisible: boolean
  editorPanelVisible: boolean
  terminalPanelVisible: boolean
}
```

**Step 2: Update defaultSessionState**

```typescript
const defaultSessionState: SessionState = {
  viewMode: 'overview',
  focusedInstanceId: null,
  terminalHeight: 300,
  sidebarWidth: 240,
  instances: {},
  // Add these:
  folderPanelVisible: true,
  editorPanelVisible: true,
  terminalPanelVisible: true,
}
```

**Step 3: Add toggle functions to store interface**

Add to `SettingsStore` interface:

```typescript
toggleFolderPanel: () => Promise<void>
toggleEditorPanel: () => Promise<void>
toggleTerminalPanel: () => Promise<void>
```

**Step 4: Implement toggle functions**

Add after `saveInstanceSession`:

```typescript
toggleFolderPanel: async () => {
  const current = get().sessionState.folderPanelVisible
  await get().saveSessionState({ folderPanelVisible: !current })
},

toggleEditorPanel: async () => {
  const current = get().sessionState.editorPanelVisible
  await get().saveSessionState({ editorPanelVisible: !current })
},

toggleTerminalPanel: async () => {
  const current = get().sessionState.terminalPanelVisible
  await get().saveSessionState({ terminalPanelVisible: !current })
},
```

**Step 5: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 6: Commit**

```bash
git add src/stores/settingsStore.ts
git commit -m "feat: add panel visibility state to settings store"
```

---

### Task 2: Create CloseConfirmModal Component

**Files:**
- Create: `src/components/shared/CloseConfirmModal.tsx`
- Create: `src/components/shared/CloseConfirmModal.css`

**Step 1: Create CloseConfirmModal.css**

```css
.close-confirm-modal {
  background: #252526;
  border-radius: 8px;
  width: 340px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  border: 1px solid #3c3c3c;
}

.close-confirm-header {
  padding: 16px 20px;
  border-bottom: 1px solid #2a2a2a;
}

.close-confirm-header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: #fff;
}

.close-confirm-content {
  padding: 16px 20px;
}

.close-confirm-info {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.close-confirm-number {
  font-size: 18px;
  font-weight: 700;
}

.close-confirm-title {
  font-size: 13px;
  color: #ccc;
}

.close-confirm-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #888;
  margin-bottom: 12px;
}

.close-confirm-status.running {
  color: #4ec9b0;
}

.close-confirm-warning {
  background: rgba(255, 200, 50, 0.1);
  border: 1px solid rgba(255, 200, 50, 0.3);
  border-radius: 4px;
  padding: 10px 12px;
  font-size: 12px;
  color: #ffc832;
  margin-bottom: 16px;
}

.close-confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.close-confirm-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 4px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.close-confirm-btn.secondary {
  background: #3c3c3c;
  color: #ccc;
}

.close-confirm-btn.secondary:hover {
  background: #4c4c4c;
  color: #fff;
}

.close-confirm-btn.danger {
  background: #c53030;
  color: #fff;
}

.close-confirm-btn.danger:hover {
  background: #e53e3e;
}
```

**Step 2: Create CloseConfirmModal.tsx**

```tsx
import './CloseConfirmModal.css'

interface CloseConfirmModalProps {
  instanceNumber: number
  instanceTitle: string
  projectColor: string
  status: string
  onConfirm: () => void
  onCancel: () => void
}

export function CloseConfirmModal({
  instanceNumber,
  instanceTitle,
  projectColor,
  status,
  onConfirm,
  onCancel,
}: CloseConfirmModalProps) {
  const isRunning = status === 'running'

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="close-confirm-modal" onClick={e => e.stopPropagation()}>
        <div className="close-confirm-header">
          <h3>Close Instance?</h3>
        </div>

        <div className="close-confirm-content">
          <div className="close-confirm-info">
            <span className="close-confirm-number" style={{ color: projectColor }}>
              #{instanceNumber}
            </span>
            <span className="close-confirm-title">{instanceTitle}</span>
          </div>

          <div className={`close-confirm-status ${isRunning ? 'running' : ''}`}>
            <span>{isRunning ? '●' : '○'}</span>
            <span>{isRunning ? 'Running' : 'Idle'}</span>
          </div>

          {isRunning && (
            <div className="close-confirm-warning">
              This instance is actively running. Closing it will terminate the process.
            </div>
          )}

          <div className="close-confirm-actions">
            <button className="close-confirm-btn secondary" onClick={onCancel}>
              No
            </button>
            <button className="close-confirm-btn danger" onClick={onConfirm}>
              Yes
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
```

**Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/shared/CloseConfirmModal.tsx src/components/shared/CloseConfirmModal.css
git commit -m "feat: add close confirmation modal component"
```

---

### Task 3: Create InstancePopup Component

**Files:**
- Create: `src/components/shared/InstancePopup.tsx`
- Create: `src/components/shared/InstancePopup.css`

**Step 1: Create InstancePopup.css**

```css
.instance-popup {
  position: absolute;
  bottom: 100%;
  left: 0;
  margin-bottom: 8px;
  background: #252526;
  border: 1px solid #3c3c3c;
  border-radius: 6px;
  padding: 6px;
  display: flex;
  gap: 4px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  z-index: 100;
}

.instance-popup-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 13px;
  transition: all 0.15s ease;
}

.instance-popup-btn:hover {
  background: #3c3c3c;
}

.instance-popup-btn.active {
  background: #0e639c;
}
```

**Step 2: Create InstancePopup.tsx**

```tsx
import { useInstanceStore, useProjectStore, useAppStore } from '@/stores'
import './InstancePopup.css'

interface InstancePopupProps {
  onClose: () => void
}

export function InstancePopup({ onClose }: InstancePopupProps) {
  const instances = useInstanceStore(state => state.instances)
  const getProject = useProjectStore(state => state.getProject)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const focusInstance = useAppStore(state => state.focusInstance)

  const handleClick = (instanceId: string) => {
    focusInstance(instanceId)
    onClose()
  }

  return (
    <div className="instance-popup">
      {[...instances]
        .sort((a, b) => a.instanceNumber - b.instanceNumber)
        .map(instance => {
          const project = getProject(instance.projectId)
          const color = project?.color || '#888'
          const isActive = focusedInstanceId === instance.id

          return (
            <button
              key={instance.id}
              className={`instance-popup-btn ${isActive ? 'active' : ''}`}
              style={{ color: isActive ? '#fff' : color }}
              onClick={() => handleClick(instance.id)}
              title={`#${instance.instanceNumber} ${instance.title}`}
            >
              {instance.instanceNumber}
            </button>
          )
        })}
    </div>
  )
}
```

**Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/shared/InstancePopup.tsx src/components/shared/InstancePopup.css
git commit -m "feat: add instance popup component for focus mode navigation"
```

---

### Task 4: Rewrite NavSidebar as Bottom Activity Bar

**Files:**
- Modify: `src/components/shared/NavSidebar.tsx`
- Modify: `src/components/shared/NavSidebar.css`

**Step 1: Replace NavSidebar.css entirely**

```css
.activity-bar {
  position: fixed;
  bottom: 0;
  left: 0;
  height: 36px;
  background: #252526;
  border-top: 1px solid #2a2a2a;
  display: flex;
  align-items: center;
  padding: 0 8px;
  gap: 2px;
  z-index: 50;
}

.activity-bar-section {
  display: flex;
  align-items: center;
  gap: 2px;
}

.activity-bar-divider {
  width: 1px;
  height: 20px;
  background: #3c3c3c;
  margin: 0 6px;
}

.activity-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: #666;
  border-radius: 4px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  transition: all 0.15s ease;
  position: relative;
}

.activity-btn:hover {
  background: #3c3c3c;
  color: #ccc;
}

.activity-btn.active {
  background: #0e639c;
  color: #fff;
}

.activity-btn-icon {
  font-size: 16px;
}

.activity-btn-num {
  font-weight: 600;
  font-size: 13px;
}

/* Panel toggle states */
.activity-btn.panel-toggle {
  color: #555;
}

.activity-btn.panel-toggle:hover {
  color: #888;
}

.activity-btn.panel-toggle.panel-active {
  color: #0e639c;
  font-weight: 700;
}

.activity-btn.panel-toggle.panel-active:hover {
  color: #1177bb;
}

/* Overview button wrapper for popup */
.overview-btn-wrapper {
  position: relative;
}
```

**Step 2: Replace NavSidebar.tsx entirely**

```tsx
import { useState } from 'react'
import { useAppStore, useInstanceStore, useProjectStore, useSettingsStore } from '@/stores'
import { InstancePopup } from './InstancePopup'
import './NavSidebar.css'

export function NavSidebar() {
  const viewMode = useAppStore(state => state.viewMode)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const focusInstance = useAppStore(state => state.focusInstance)
  const instances = useInstanceStore(state => state.instances)
  const getProject = useProjectStore(state => state.getProject)

  const sessionState = useSettingsStore(state => state.sessionState)
  const toggleFolderPanel = useSettingsStore(state => state.toggleFolderPanel)
  const toggleEditorPanel = useSettingsStore(state => state.toggleEditorPanel)
  const toggleTerminalPanel = useSettingsStore(state => state.toggleTerminalPanel)

  const [showPopup, setShowPopup] = useState(false)

  const isFocusMode = viewMode === 'focus'

  return (
    <nav className="activity-bar">
      {/* Overview button */}
      <div
        className="overview-btn-wrapper"
        onMouseEnter={() => isFocusMode && setShowPopup(true)}
        onMouseLeave={() => setShowPopup(false)}
      >
        <button
          className={`activity-btn ${!isFocusMode ? 'active' : ''}`}
          onClick={returnToOverview}
          title="Overview"
        >
          <span className="activity-btn-icon">▣</span>
        </button>
        {isFocusMode && showPopup && instances.length > 0 && (
          <InstancePopup onClose={() => setShowPopup(false)} />
        )}
      </div>

      {isFocusMode ? (
        /* Focus mode: panel toggles */
        <>
          <div className="activity-bar-divider" />
          <div className="activity-bar-section">
            <button
              className={`activity-btn panel-toggle ${sessionState.folderPanelVisible ? 'panel-active' : ''}`}
              onClick={toggleFolderPanel}
              title="Toggle Folder Panel"
            >
              <span className="activity-btn-icon">📁</span>
            </button>
            <button
              className={`activity-btn panel-toggle ${sessionState.editorPanelVisible ? 'panel-active' : ''}`}
              onClick={toggleEditorPanel}
              title="Toggle Editor Panel"
            >
              <span className="activity-btn-icon">📄</span>
            </button>
            <button
              className={`activity-btn panel-toggle ${sessionState.terminalPanelVisible ? 'panel-active' : ''}`}
              onClick={toggleTerminalPanel}
              title="Toggle Terminal Panel"
            >
              <span className="activity-btn-icon">⌨</span>
            </button>
          </div>
        </>
      ) : (
        /* Overview mode: instance numbers */
        instances.length > 0 && (
          <div className="activity-bar-section">
            {[...instances]
              .sort((a, b) => a.instanceNumber - b.instanceNumber)
              .map(instance => {
                const project = getProject(instance.projectId)
                const color = project?.color || '#888'
                const isActive = focusedInstanceId === instance.id

                return (
                  <button
                    key={instance.id}
                    className={`activity-btn ${isActive ? 'active' : ''}`}
                    style={{ color: isActive ? '#fff' : color }}
                    onClick={() => focusInstance(instance.id)}
                    title={`#${instance.instanceNumber} ${instance.title}`}
                  >
                    <span className="activity-btn-num">{instance.instanceNumber}</span>
                  </button>
                )
              })}
          </div>
        )
      )}
    </nav>
  )
}
```

**Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/shared/NavSidebar.tsx src/components/shared/NavSidebar.css
git commit -m "feat: convert NavSidebar to bottom activity bar"
```

---

### Task 5: Update App Layout for Bottom Activity Bar

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/index.css`

**Step 1: Update App.tsx - move NavSidebar outside app-middle**

Replace the return statement in App.tsx (lines 98-122):

```tsx
return (
  <ErrorBoundary>
    <div className="app-container">
      <TopBar />
      <div className="app-main">
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
      </div>
      <NavSidebar />
      {newInstanceModalOpen && <NewInstanceModal />}
      {settingsOpen && <SettingsModal />}
    </div>
  </ErrorBoundary>
)
```

**Step 2: Update index.css - remove app-middle, add bottom padding**

Replace entire file:

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
  padding-bottom: 36px; /* Space for activity bar */
}

.app-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.app-content {
  display: flex;
  flex: 1;
  min-height: 0;
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

/* Shared modal overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid #2a2a2a;
}

.modal-header h2 {
  font-size: 16px;
  font-weight: 600;
}

.close-btn {
  background: none;
  border: none;
  color: #888;
  font-size: 20px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.close-btn:hover {
  color: #fff;
}
```

**Step 3: Remove StatusBar import and usage from App.tsx**

Remove line 6:
```tsx
import { StatusBar } from '@/components/shared/StatusBar'
```

Remove `<StatusBar />` from the JSX (it's no longer needed with bottom activity bar).

**Step 4: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 5: Commit**

```bash
git add src/App.tsx src/index.css
git commit -m "feat: update app layout for bottom activity bar"
```

---

### Task 6: Update TopBar for Both Modes

**Files:**
- Modify: `src/components/shared/TopBar.tsx`
- Modify: `src/components/shared/TopBar.css`

**Step 1: Update TopBar.tsx**

Replace entire file:

```tsx
import { useState } from 'react'
import {
  useAppStore,
  useSettingsStore,
  useInstanceStore,
  useProjectStore,
} from '@/stores'
import { CloseConfirmModal } from './CloseConfirmModal'
import './TopBar.css'

export function TopBar() {
  const viewMode = useAppStore(state => state.viewMode)
  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const returnToOverview = useAppStore(state => state.returnToOverview)
  const toggleNewInstanceModal = useAppStore(state => state.toggleNewInstanceModal)
  const toggleSettings = useAppStore(state => state.toggleSettings)
  const toggleQuestionsPanel = useSettingsStore(state => state.toggleQuestionsPanel)
  const questionsPanelVisible = useSettingsStore(state => state.questionsPanelVisible)

  const instance = useInstanceStore(state =>
    focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
  )
  const removeInstance = useInstanceStore(state => state.removeInstance)
  const project = useProjectStore(state =>
    instance ? state.getProject(instance.projectId) : undefined
  )

  const [showCloseModal, setShowCloseModal] = useState(false)

  const handleMinimize = () => {
    returnToOverview()
  }

  const handleCloseClick = () => {
    setShowCloseModal(true)
  }

  const handleCloseConfirm = () => {
    if (focusedInstanceId) {
      window.electronAPI.killInstance(focusedInstanceId)
      removeInstance(focusedInstanceId)
      returnToOverview()
    }
    setShowCloseModal(false)
  }

  const handleCloseCancel = () => {
    setShowCloseModal(false)
  }

  const isFocusMode = viewMode === 'focus' && instance && project

  return (
    <>
      <header className="top-bar">
        <div className="top-bar-drag-region" />

        <div className="top-bar-left" />

        <div className="top-bar-center">
          {isFocusMode ? (
            <span className="app-title" style={{ color: project.color }}>
              #{instance.instanceNumber} {instance.title}
            </span>
          ) : (
            <span className="app-title">Jenklaud</span>
          )}
        </div>

        <div className="top-bar-right">
          {isFocusMode ? (
            <div className="instance-controls">
              <button
                className="control-btn icon-btn"
                onClick={handleMinimize}
                title="Minimize (return to overview)"
              >
                −
              </button>
              <button
                className="control-btn icon-btn"
                onClick={handleCloseClick}
                title="Close Instance"
              >
                ×
              </button>
            </div>
          ) : (
            <>
              <button
                className="top-bar-btn primary"
                onClick={toggleNewInstanceModal}
              >
                + New
              </button>
              <button
                className={`top-bar-btn ${questionsPanelVisible ? 'active' : ''}`}
                onClick={toggleQuestionsPanel}
              >
                Questions
              </button>
              <button
                className="top-bar-btn icon-btn"
                onClick={toggleSettings}
                title="Settings"
              >
                ⚙
              </button>
            </>
          )}
        </div>
      </header>

      {showCloseModal && instance && project && (
        <CloseConfirmModal
          instanceNumber={instance.instanceNumber}
          instanceTitle={instance.title}
          projectColor={project.color}
          status={instance.status}
          onConfirm={handleCloseConfirm}
          onCancel={handleCloseCancel}
        />
      )}
    </>
  )
}
```

**Step 2: Update TopBar.css**

Replace entire file:

```css
.top-bar {
  height: 40px;
  background: #1e1e1e;
  border-bottom: 1px solid #2a2a2a;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  padding-left: 80px;
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
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  -webkit-app-region: no-drag;
  z-index: 1;
}

.top-bar-right {
  justify-content: flex-end;
}

.top-bar-center {
  display: flex;
  align-items: center;
  -webkit-app-region: no-drag;
  z-index: 1;
}

.app-title {
  font-weight: 600;
  font-size: 13px;
  line-height: 1;
  color: #fff;
}

.top-bar-btn {
  padding: 5px 10px;
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

.top-bar-btn.icon-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
}

.instance-controls {
  display: flex;
  gap: 4px;
}

.control-btn {
  padding: 4px 8px;
  border: none;
  background: #3c3c3c;
  color: #ccc;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s ease;
}

.control-btn:hover {
  background: #4c4c4c;
  color: #fff;
}

.control-btn.icon-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.control-btn.icon-btn:last-child:hover {
  background: #c53030;
  color: #fff;
}
```

**Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 4: Commit**

```bash
git add src/components/shared/TopBar.tsx src/components/shared/TopBar.css
git commit -m "feat: update TopBar with settings icon and minimize/close controls"
```

---

### Task 7: Update FocusView to Use Panel Visibility

**Files:**
- Modify: `src/components/Focus/FocusView.tsx`
- Modify: `src/components/Focus/FocusView.css`

**Step 1: Update FocusView.tsx imports and state**

Add to imports at line 1:
```tsx
import { useState, useCallback, useEffect } from 'react'
import { useInstanceStore, useProjectStore, useAppStore, useEditorStore, useSettingsStore } from '@/stores'
```

**Step 2: Get panel visibility from store**

After line 10 (`const saveSessionState = ...`), add:
```tsx
const folderPanelVisible = useSettingsStore(state => state.sessionState.folderPanelVisible)
const editorPanelVisible = useSettingsStore(state => state.sessionState.editorPanelVisible)
const terminalPanelVisible = useSettingsStore(state => state.sessionState.terminalPanelVisible)
```

**Step 3: Update the JSX return statement**

Replace the return statement (starting at line 103) with:

```tsx
return (
  <div className="focus-view">
    {folderPanelVisible && (
      <>
        <div className="focus-sidebar" style={{ width: sidebarWidth }}>
          <FileTree projectPath={project.path} />
        </div>
        <div
          className={`focus-sidebar-resize ${isResizing && resizeType === 'sidebar' ? 'active' : ''}`}
          onMouseDown={handleSidebarResizeStart}
        />
      </>
    )}

    <div className="focus-main">
      {editorPanelVisible && (
        <div className="focus-editor-area" style={{ flex: terminalPanelVisible ? 1 : undefined }}>
          <EditorTabs />
          <Editor file={activeFile} />
        </div>
      )}

      {editorPanelVisible && terminalPanelVisible && (
        <div
          className={`focus-resize-handle ${isResizing && resizeType === 'terminal' ? 'active' : ''}`}
          onMouseDown={handleTerminalResizeStart}
        />
      )}

      {terminalPanelVisible && (
        <div
          className="focus-terminal"
          style={{ height: editorPanelVisible ? terminalHeight : '100%' }}
        >
          <Terminal instanceId={instance.id} onInput={handleInput} />
        </div>
      )}

      {!editorPanelVisible && !terminalPanelVisible && (
        <div className="focus-empty-state">
          <p>All panels hidden</p>
          <p className="focus-empty-hint">Use the panel toggles in the activity bar</p>
        </div>
      )}
    </div>
  </div>
)
```

**Step 4: Update FocusView.css - reduce borders and add empty state**

Replace entire file:

```css
.focus-view {
  display: flex;
  flex: 1;
  height: 100%;
  min-height: 0;
  background: #1e1e1e;
  overflow: hidden;
}

.focus-view-empty {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  color: #666;
  font-size: 14px;
}

.focus-sidebar {
  align-self: stretch;
  min-height: 0;
  background: #252526;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  overflow: hidden;
}

.focus-sidebar-resize {
  width: 2px;
  background: #2a2a2a;
  cursor: ew-resize;
  flex-shrink: 0;
  transition: background 0.15s ease;
}

.focus-sidebar-resize:hover,
.focus-sidebar-resize.active {
  background: #0e639c;
}

.focus-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.focus-editor-area {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.focus-resize-handle {
  height: 2px;
  background: #2a2a2a;
  cursor: ns-resize;
  flex-shrink: 0;
  transition: background 0.15s ease;
}

.focus-resize-handle:hover,
.focus-resize-handle.active {
  background: #0e639c;
}

.focus-terminal {
  flex-shrink: 0;
  min-height: 100px;
  width: 100%;
  overflow: hidden;
}

.focus-empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #555;
  font-size: 14px;
  gap: 8px;
}

.focus-empty-hint {
  font-size: 12px;
  color: #444;
}
```

**Step 5: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 6: Commit**

```bash
git add src/components/Focus/FocusView.tsx src/components/Focus/FocusView.css
git commit -m "feat: add panel visibility toggles to FocusView"
```

---

### Task 8: Reduce Borders Globally

**Files:**
- Modify: `src/components/Overview/OverviewGrid.css` (if exists)
- Modify: `src/components/Overview/InstanceTile.css`
- Modify: `src/components/Focus/Terminal.css`
- Modify: `src/components/Focus/Editor.css`
- Modify: `src/components/Focus/FileTree.css`

**Step 1: Read current CSS files**

Read each file to understand current border styles.

**Step 2: Update border colors from #3c3c3c to #2a2a2a**

In each file, replace:
- `border: 1px solid #3c3c3c` → `border: 1px solid #2a2a2a`
- `border-color: #3c3c3c` → `border-color: #2a2a2a`
- `border-bottom: 1px solid #3c3c3c` → `border-bottom: 1px solid #2a2a2a`
- etc.

**Step 3: Verify app runs**

Run: `npm run dev`
Expected: App starts, new layout visible

**Step 4: Commit**

```bash
git add -A
git commit -m "style: reduce border visibility globally"
```

---

### Task 9: Final Cleanup and Testing

**Step 1: Delete StatusBar if no longer needed**

Check if StatusBar is used elsewhere. If not:
```bash
rm src/components/shared/StatusBar.tsx src/components/shared/StatusBar.css
```

**Step 2: Run TypeScript check**

Run: `npx tsc --noEmit`
Expected: No errors

**Step 3: Manual testing checklist**

- [ ] Overview mode shows instance numbers in activity bar
- [ ] Clicking instance number focuses it
- [ ] Focus mode shows only overview icon + panel toggles
- [ ] Hovering overview in focus mode shows instance popup
- [ ] Panel toggles show/hide folder, editor, terminal
- [ ] Active panel toggle has accent color
- [ ] Minimize button returns to overview
- [ ] Close button shows confirmation modal
- [ ] Modal shows instance info and warns if running
- [ ] Settings icon in top-right works
- [ ] Borders are subtle (#2a2a2a)

**Step 4: Final commit**

```bash
git add -A
git commit -m "chore: cleanup and finalize zed-style UI"
```
