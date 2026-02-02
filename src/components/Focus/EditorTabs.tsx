import { useEditorStore } from '@/stores'
import './EditorTabs.css'

export function EditorTabs() {
  const openFiles = useEditorStore(state => state.openFiles)
  const activeFilePath = useEditorStore(state => state.activeFilePath)
  const setActiveFile = useEditorStore(state => state.setActiveFile)
  const closeFile = useEditorStore(state => state.closeFile)

  if (openFiles.length === 0) {
    return null
  }

  const handleClose = (e: React.MouseEvent, path: string) => {
    e.stopPropagation()
    closeFile(path)
  }

  return (
    <div className="editor-tabs">
      {openFiles.map(file => (
        <div
          key={file.path}
          className={`editor-tab ${activeFilePath === file.path ? 'active' : ''} ${file.isModified ? 'modified' : ''}`}
          onClick={() => setActiveFile(file.path)}
          title={file.path}
        >
          <span className="tab-name">{file.name}</span>
          {file.isModified && <span className="tab-modified">●</span>}
          <button
            className="tab-close"
            onClick={(e) => handleClose(e, file.path)}
            title="Close"
          >
            ×
          </button>
        </div>
      ))}
    </div>
  )
}
