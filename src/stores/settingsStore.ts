import { create } from 'zustand'

interface Settings {
  theme: 'dark' | 'light'
  questionsPanelVisible: boolean
  editorFontSize: number
  terminalFontSize: number
}

interface SettingsStore extends Settings {
  isLoading: boolean
  loadSettings: () => Promise<void>
  setSetting: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>
  toggleQuestionsPanel: () => Promise<void>
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  theme: 'dark',
  questionsPanelVisible: true,
  editorFontSize: 13,
  terminalFontSize: 13,
  isLoading: true,

  loadSettings: async () => {
    const settings = await window.electronAPI.dbSettingsGetAll()
    set({
      theme: (settings.theme as 'dark' | 'light') || 'dark',
      questionsPanelVisible: settings.questions_panel_visible !== 'false',
      editorFontSize: parseInt(settings.editor_font_size || '13', 10),
      terminalFontSize: parseInt(settings.terminal_font_size || '13', 10),
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
}))
