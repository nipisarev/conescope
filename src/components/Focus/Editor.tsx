import { useCallback, useMemo } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { json } from '@codemirror/lang-json'
import { markdown } from '@codemirror/lang-markdown'
import { css } from '@codemirror/lang-css'
import { html } from '@codemirror/lang-html'
import { python } from '@codemirror/lang-python'
import { useEditorStore, OpenFile } from '@/stores/editorStore'
import './Editor.css'

interface EditorProps {
  file: OpenFile | null
}

export function Editor({ file }: EditorProps) {
  const updateFileContent = useEditorStore(state => state.updateFileContent)
  const saveFile = useEditorStore(state => state.saveFile)

  const extensions = useMemo(() => {
    if (!file) return []

    switch (file.language) {
      case 'javascript':
        return [javascript({ jsx: true })]
      case 'typescript':
        return [javascript({ jsx: true, typescript: true })]
      case 'json':
        return [json()]
      case 'markdown':
        return [markdown()]
      case 'css':
        return [css()]
      case 'html':
        return [html()]
      case 'python':
        return [python()]
      default:
        return []
    }
  }, [file?.language])

  const handleChange = useCallback((value: string) => {
    if (file) {
      updateFileContent(file.path, value)
    }
  }, [file, updateFileContent])

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    // Cmd+S / Ctrl+S to save
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault()
      if (file) {
        saveFile(file.path)
      }
    }
  }, [file, saveFile])

  if (!file) {
    return (
      <div className="editor-empty">
        <div className="editor-empty-content">
          <span className="editor-empty-icon">📄</span>
          <p>Select a file to edit</p>
        </div>
      </div>
    )
  }

  return (
    <div className="editor" onKeyDown={handleKeyDown}>
      <CodeMirror
        value={file.content}
        height="100%"
        theme="dark"
        extensions={extensions}
        onChange={handleChange}
        basicSetup={{
          lineNumbers: true,
          highlightActiveLineGutter: true,
          highlightActiveLine: true,
          foldGutter: true,
          dropCursor: true,
          allowMultipleSelections: true,
          indentOnInput: true,
          bracketMatching: true,
          closeBrackets: true,
          autocompletion: true,
          rectangularSelection: true,
          crosshairCursor: false,
          highlightSelectionMatches: true,
        }}
      />
    </div>
  )
}
