import { useCallback, useMemo } from 'react'
import CodeMirror from '@uiw/react-codemirror'
import { oneDark } from '@codemirror/theme-one-dark'
import { EditorView } from '@codemirror/view'
import { loadLanguage, LanguageName } from '@uiw/codemirror-extensions-langs'
import { useEditorStore, OpenFile } from '@/stores/editorStore'
import { useSettingsStore } from '@/stores'
import './Editor.css'

interface EditorProps {
  file: OpenFile | null
}

export function Editor({ file }: EditorProps) {
  const updateFileContent = useEditorStore(state => state.updateFileContent)
  const saveFile = useEditorStore(state => state.saveFile)
  const editorFontSize = useSettingsStore(state => state.editorFontSize)

  const extensions = useMemo(() => {
    const exts = [
      EditorView.theme({
        '&': { fontSize: `${editorFontSize}px` },
        '.cm-content': { fontFamily: 'Menlo, "SF Mono", Monaco, "Cascadia Code", monospace' },
        '.cm-gutters': { fontSize: `${editorFontSize}px` },
      })
    ]
    if (file) {
      const lang = loadLanguage(file.language as LanguageName)
      if (lang) exts.push(lang)
    }
    return exts
  }, [file?.language, editorFontSize])

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
        theme={oneDark}
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
