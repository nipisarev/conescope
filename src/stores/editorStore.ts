import { create } from 'zustand'

export interface OpenFile {
  path: string
  name: string
  content: string
  isModified: boolean
  language: string
}

interface EditorStore {
  openFiles: OpenFile[]
  activeFilePath: string | null

  openFile: (path: string) => Promise<void>
  closeFile: (path: string) => void
  setActiveFile: (path: string) => void
  updateFileContent: (path: string, content: string) => void
  saveFile: (path: string) => Promise<void>
  closeAllFiles: () => void
}

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || ''
  const languageMap: Record<string, string> = {
    js: 'javascript',
    jsx: 'javascript',
    ts: 'typescript',
    tsx: 'typescript',
    json: 'json',
    md: 'markdown',
    css: 'css',
    scss: 'css',
    html: 'html',
    htm: 'html',
    py: 'python',
    yml: 'yaml',
    yaml: 'yaml',
    sh: 'shell',
    bash: 'shell',
    zsh: 'shell',
  }
  return languageMap[ext] || 'text'
}

export const useEditorStore = create<EditorStore>((set, get) => ({
  openFiles: [],
  activeFilePath: null,

  openFile: async (path: string) => {
    const { openFiles } = get()

    // Check if already open
    const existing = openFiles.find(f => f.path === path)
    if (existing) {
      set({ activeFilePath: path })
      return
    }

    // Read file content
    const content = await window.electronAPI.readFile(path)
    if (content === null) {
      console.error('Failed to read file:', path)
      return
    }

    const name = path.split('/').pop() || path
    const language = getLanguageFromPath(path)

    set(state => ({
      openFiles: [...state.openFiles, { path, name, content, isModified: false, language }],
      activeFilePath: path,
    }))
  },

  closeFile: (path: string) => {
    set(state => {
      const newFiles = state.openFiles.filter(f => f.path !== path)
      let newActivePath = state.activeFilePath

      // If closing active file, switch to another
      if (state.activeFilePath === path) {
        const closingIndex = state.openFiles.findIndex(f => f.path === path)
        if (newFiles.length > 0) {
          // Try to select the file at the same index, or the last one
          const newIndex = Math.min(closingIndex, newFiles.length - 1)
          newActivePath = newFiles[newIndex].path
        } else {
          newActivePath = null
        }
      }

      return { openFiles: newFiles, activeFilePath: newActivePath }
    })
  },

  setActiveFile: (path: string) => {
    set({ activeFilePath: path })
  },

  updateFileContent: (path: string, content: string) => {
    set(state => ({
      openFiles: state.openFiles.map(f =>
        f.path === path ? { ...f, content, isModified: true } : f
      ),
    }))
  },

  saveFile: async (path: string) => {
    const file = get().openFiles.find(f => f.path === path)
    if (!file) return

    const success = await window.electronAPI.writeFile(path, file.content)
    if (success) {
      set(state => ({
        openFiles: state.openFiles.map(f =>
          f.path === path ? { ...f, isModified: false } : f
        ),
      }))
    }
  },

  closeAllFiles: () => {
    set({ openFiles: [], activeFilePath: null })
  },
}))
