import { create } from 'zustand'
import { ViewMode } from '@/types'
import { useSettingsStore } from './settingsStore'

interface AppStore {
  viewMode: ViewMode
  focusedInstanceId: string | null
  questionsQueueOpen: boolean
  settingsOpen: boolean
  newInstanceModalOpen: boolean

  initFromSession: () => void
  setViewMode: (mode: ViewMode) => void
  focusInstance: (instanceId: string) => void
  returnToOverview: () => void
  toggleQuestionsQueue: () => void
  toggleSettings: () => void
  toggleNewInstanceModal: () => void
}

export const useAppStore = create<AppStore>((set) => ({
  viewMode: 'overview',
  focusedInstanceId: null,
  questionsQueueOpen: false,
  settingsOpen: false,
  newInstanceModalOpen: false,

  initFromSession: () => {
    const { sessionState } = useSettingsStore.getState()
    set({
      viewMode: sessionState.viewMode,
      focusedInstanceId: sessionState.focusedInstanceId,
    })
  },

  setViewMode: (mode) => {
    set({ viewMode: mode })
    useSettingsStore.getState().saveSessionState({ viewMode: mode })
  },

  focusInstance: (instanceId) => {
    set({
      viewMode: 'focus',
      focusedInstanceId: instanceId
    })
    useSettingsStore.getState().saveSessionState({
      viewMode: 'focus',
      focusedInstanceId: instanceId
    })
  },

  returnToOverview: () => {
    set({
      viewMode: 'overview',
      focusedInstanceId: null
    })
    useSettingsStore.getState().saveSessionState({
      viewMode: 'overview',
      focusedInstanceId: null
    })
  },

  toggleQuestionsQueue: () => set(state => ({
    questionsQueueOpen: !state.questionsQueueOpen
  })),

  toggleSettings: () => set(state => ({
    settingsOpen: !state.settingsOpen
  })),

  toggleNewInstanceModal: () => set(state => ({
    newInstanceModalOpen: !state.newInstanceModalOpen
  }))
}))
