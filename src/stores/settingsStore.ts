import { create } from 'zustand'

interface Settings {
  theme: 'dark' | 'light'
  questionsPanelVisible: boolean
  editorFontSize: number
  terminalFontSize: number
}

interface InstanceSession {
  openFiles: string[]  // paths only
  activeFile: string | null
}

interface SessionState {
  viewMode: 'overview' | 'focus'
  focusedInstanceId: string | null
  terminalHeight: number
  sidebarWidth: number
  folderPanelVisible: boolean
  editorPanelVisible: boolean
  terminalPanelVisible: boolean
  instances: Record<string, InstanceSession>
}

interface SettingsStore extends Settings {
  isLoading: boolean
  sessionState: SessionState
  loadSettings: () => Promise<void>
  setSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>
  toggleQuestionsPanel: () => Promise<void>
  saveSessionState: (state: Partial<SessionState>) => Promise<void>
  saveInstanceSession: (instanceId: string, session: InstanceSession) => Promise<void>
  toggleFolderPanel: () => Promise<void>
  toggleEditorPanel: () => Promise<void>
  toggleTerminalPanel: () => Promise<void>
}

const defaultSessionState: SessionState = {
  viewMode: 'overview',
  focusedInstanceId: null,
  terminalHeight: 300,
  sidebarWidth: 240,
  folderPanelVisible: true,
  editorPanelVisible: true,
  terminalPanelVisible: true,
  instances: {},
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  theme: 'dark',
  questionsPanelVisible: true,
  editorFontSize: 13,
  terminalFontSize: 13,
  isLoading: true,
  sessionState: defaultSessionState,

  loadSettings: async () => {
    const settings = await window.electronAPI.dbSettingsGetAll()

    // Parse session state
    let sessionState = defaultSessionState
    if (settings.session_state) {
      try {
        sessionState = { ...defaultSessionState, ...JSON.parse(settings.session_state) }
      } catch (e) {
        console.error('Failed to parse session state:', e)
      }
    }

    set({
      theme: (settings.theme as 'dark' | 'light') || 'dark',
      questionsPanelVisible: settings.questions_panel_visible !== 'false',
      editorFontSize: parseInt(settings.editor_font_size || '13', 10),
      terminalFontSize: parseInt(settings.terminal_font_size || '13', 10),
      sessionState,
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

  saveSessionState: async (updates) => {
    const newState = { ...get().sessionState, ...updates }
    set({ sessionState: newState })
    await window.electronAPI.dbSettingsSet('session_state', JSON.stringify(newState))
  },

  saveInstanceSession: async (instanceId, session) => {
    const newInstances = { ...get().sessionState.instances, [instanceId]: session }
    await get().saveSessionState({ instances: newInstances })
  },

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
}))
