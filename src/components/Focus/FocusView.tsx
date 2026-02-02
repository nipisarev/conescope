import { useState, useCallback, useEffect } from 'react'
import { useInstanceStore, useProjectStore, useAppStore, useEditorStore } from '@/stores'
import { FileTree } from './FileTree'
import { EditorTabs } from './EditorTabs'
import { Editor } from './Editor'
import { Terminal } from './Terminal'
import './FocusView.css'

export function FocusView() {
  const [terminalHeight, setTerminalHeight] = useState(300)
  const [isResizing, setIsResizing] = useState(false)

  const focusedInstanceId = useAppStore(state => state.focusedInstanceId)
  const instance = useInstanceStore(state =>
    focusedInstanceId ? state.getInstance(focusedInstanceId) : undefined
  )
  const project = useProjectStore(state =>
    instance ? state.getProject(instance.projectId) : undefined
  )
  const openFiles = useEditorStore(state => state.openFiles)
  const activeFilePath = useEditorStore(state => state.activeFilePath)
  const closeAllFiles = useEditorStore(state => state.closeAllFiles)

  const activeFile = openFiles.find(f => f.path === activeFilePath) || null

  // Close all files when switching instances
  useEffect(() => {
    closeAllFiles()
  }, [focusedInstanceId, closeAllFiles])

  const handleInput = useCallback((data: string) => {
    if (focusedInstanceId) {
      window.electronAPI.sendInput(focusedInstanceId, data)
    }
  }, [focusedInstanceId])

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsResizing(true)
  }, [])

  useEffect(() => {
    if (!isResizing) return

    const handleMouseMove = (e: MouseEvent) => {
      const container = document.querySelector('.focus-main')
      if (!container) return

      const rect = container.getBoundingClientRect()
      const newHeight = rect.bottom - e.clientY
      setTerminalHeight(Math.max(100, Math.min(newHeight, rect.height - 100)))
    }

    const handleMouseUp = () => {
      setIsResizing(false)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
    }
  }, [isResizing])

  if (!instance || !project) {
    return (
      <div className="focus-view-empty">
        <p>No instance selected</p>
      </div>
    )
  }

  return (
    <div className="focus-view">
      <div className="focus-sidebar">
        <FileTree projectPath={project.path} />
      </div>

      <div className="focus-main">
        <div className="focus-editor-area" style={{ flex: 1 }}>
          <EditorTabs />
          <Editor file={activeFile} />
        </div>

        <div
          className={`focus-resize-handle ${isResizing ? 'active' : ''}`}
          onMouseDown={handleResizeStart}
        />

        <div className="focus-terminal" style={{ height: terminalHeight }}>
          <Terminal instanceId={instance.id} onInput={handleInput} />
        </div>
      </div>
    </div>
  )
}
