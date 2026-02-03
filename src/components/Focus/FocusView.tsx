import { useState, useCallback, useEffect } from 'react'
import { useInstanceStore, useProjectStore, useAppStore, useEditorStore, useSettingsStore } from '@/stores'
import { FileTree } from './FileTree'
import { EditorTabs } from './EditorTabs'
import { Editor } from './Editor'
import { Terminal } from './Terminal'
import './FocusView.css'

export function FocusView() {
  const sessionState = useSettingsStore(state => state.sessionState)
  const saveSessionState = useSettingsStore(state => state.saveSessionState)
  const folderPanelVisible = useSettingsStore(state => state.sessionState.folderPanelVisible)
  const editorPanelVisible = useSettingsStore(state => state.sessionState.editorPanelVisible)
  const terminalPanelVisible = useSettingsStore(state => state.sessionState.terminalPanelVisible)

  const [terminalHeight, setTerminalHeight] = useState(sessionState.terminalHeight)
  const [sidebarWidth, setSidebarWidth] = useState(sessionState.sidebarWidth)
  const [isResizing, setIsResizing] = useState(false)
  const [resizeType, setResizeType] = useState<'terminal' | 'sidebar' | null>(null)

  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const instance = useInstanceStore(state =>
    focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
  )
  const project = useProjectStore(state =>
    instance ? state.getProject(instance.projectId) : undefined
  )
  const openFiles = useEditorStore(state => state.openFiles)
  const activeFilePath = useEditorStore(state => state.activeFilePath)
  const switchInstance = useEditorStore(state => state.switchInstance)

  const activeFile = openFiles.find(f => f.path === activeFilePath) || null

  // Restore files when switching instances
  useEffect(() => {
    if (focusedInstanceId) {
      switchInstance(focusedInstanceId)
    }
  }, [focusedInstanceId, switchInstance])

  const handleInput = useCallback((data: string) => {
    if (focusedInstanceId) {
      window.electronAPI.sendInput(focusedInstanceId, data)
    }
  }, [focusedInstanceId])

  const handleTerminalResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsResizing(true)
    setResizeType('terminal')
  }, [])

  const handleSidebarResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsResizing(true)
    setResizeType('sidebar')
  }, [])

  useEffect(() => {
    if (!isResizing) return

    const handleMouseMove = (e: MouseEvent) => {
      if (resizeType === 'terminal') {
        const container = document.querySelector('.focus-main')
        if (!container) return
        const rect = container.getBoundingClientRect()
        const newHeight = rect.bottom - e.clientY
        setTerminalHeight(Math.max(100, Math.min(newHeight, rect.height - 100)))
      } else if (resizeType === 'sidebar') {
        const container = document.querySelector('.focus-view')
        if (!container) return
        const rect = container.getBoundingClientRect()
        const newWidth = e.clientX - rect.left
        setSidebarWidth(Math.max(150, Math.min(newWidth, 500)))
      }
    }

    const handleMouseUp = () => {
      setIsResizing(false)
      // Save dimensions after resize ends
      if (resizeType === 'terminal') {
        saveSessionState({ terminalHeight })
      } else if (resizeType === 'sidebar') {
        saveSessionState({ sidebarWidth })
      }
      setResizeType(null)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
  }, [isResizing, resizeType, terminalHeight, sidebarWidth, saveSessionState])

  if (!instance || !project) {
    return (
      <div className="focus-view-empty">
        <p>No instance selected</p>
      </div>
    )
  }

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
}
