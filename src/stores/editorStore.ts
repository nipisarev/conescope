import { create } from 'zustand'
import { useSettingsStore } from './settingsStore'

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
  currentInstanceId: string | null

  openFile: (path: string) => Promise<void>
  closeFile: (path: string) => void
  setActiveFile: (path: string) => void
  updateFileContent: (path: string, content: string) => void
  saveFile: (path: string) => Promise<void>
  closeAllFiles: () => void
  switchInstance: (instanceId: string) => Promise<void>
  saveCurrentSession: () => void
}

function getLanguageFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() || ''
  const languageMap: Record<string, string> = {
    // JavaScript/TypeScript
    js: 'javascript',
    mjs: 'javascript',
    cjs: 'javascript',
    jsx: 'jsx',
    ts: 'typescript',
    tsx: 'tsx',
    // Web
    html: 'html',
    htm: 'html',
    css: 'css',
    scss: 'sass',
    sass: 'sass',
    less: 'less',
    // Data formats
    json: 'json',
    xml: 'xml',
    yaml: 'yaml',
    yml: 'yaml',
    toml: 'toml',
    // Markdown
    md: 'markdown',
    mdx: 'markdown',
    // Shell
    sh: 'shell',
    bash: 'shell',
    zsh: 'shell',
    fish: 'shell',
    // Programming languages
    py: 'python',
    rb: 'ruby',
    php: 'php',
    go: 'go',
    rs: 'rust',
    java: 'java',
    kt: 'kotlin',
    kts: 'kotlin',
    swift: 'swift',
    c: 'c',
    h: 'c',
    cpp: 'cpp',
    cc: 'cpp',
    cxx: 'cpp',
    hpp: 'cpp',
    cs: 'csharp',
    scala: 'scala',
    clj: 'clojure',
    cljs: 'clojure',
    ex: 'elixir',
    exs: 'elixir',
    erl: 'erlang',
    hs: 'haskell',
    lua: 'lua',
    r: 'r',
    R: 'r',
    jl: 'julia',
    dart: 'dart',
    // Database
    sql: 'sql',
    mysql: 'mysql',
    pgsql: 'pgsql',
    // Config/DevOps
    dockerfile: 'dockerfile',
    tf: 'hcl',
    hcl: 'hcl',
    nginx: 'nginx',
    // Other
    vue: 'vue',
    svelte: 'svelte',
    graphql: 'graphql',
    gql: 'graphql',
    proto: 'protobuf',
    wasm: 'wast',
    wat: 'wast',
  }

  // Handle Dockerfile (no extension)
  const filename = path.split('/').pop()?.toLowerCase() || ''
  if (filename === 'dockerfile') return 'dockerfile'

  return languageMap[ext] || 'text'
}

export const useEditorStore = create<EditorStore>((set, get) => ({
  openFiles: [],
  activeFilePath: null,
  currentInstanceId: null,

  openFile: async (path: string) => {
    const { openFiles } = get()

    // Check if already open
    const existing = openFiles.find(f => f.path === path)
    if (existing) {
      set({ activeFilePath: path })
      get().saveCurrentSession()
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
    get().saveCurrentSession()
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
    get().saveCurrentSession()
  },

  setActiveFile: (path: string) => {
    set({ activeFilePath: path })
    get().saveCurrentSession()
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

  switchInstance: async (instanceId: string) => {
    const { currentInstanceId } = get()

    // Save current session before switching
    if (currentInstanceId) {
      get().saveCurrentSession()
    }

    // Load session for new instance
    const { sessionState } = useSettingsStore.getState()
    const instanceSession = sessionState.instances[instanceId]

    if (instanceSession && instanceSession.openFiles.length > 0) {
      // Restore files
      const files: OpenFile[] = []
      for (const filePath of instanceSession.openFiles) {
        const content = await window.electronAPI.readFile(filePath)
        if (content !== null) {
          const name = filePath.split('/').pop() || filePath
          const language = getLanguageFromPath(filePath)
          files.push({ path: filePath, name, content, isModified: false, language })
        }
      }
      set({
        openFiles: files,
        activeFilePath: instanceSession.activeFile,
        currentInstanceId: instanceId,
      })
    } else {
      set({
        openFiles: [],
        activeFilePath: null,
        currentInstanceId: instanceId,
      })
    }
  },

  saveCurrentSession: () => {
    const { openFiles, activeFilePath, currentInstanceId } = get()
    if (!currentInstanceId) return

    useSettingsStore.getState().saveInstanceSession(currentInstanceId, {
      openFiles: openFiles.map(f => f.path),
      activeFile: activeFilePath,
    })
  },
}))
