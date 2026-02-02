import { create } from 'zustand'
import { ViewMode } from '@/types'

interface AppStore {
  viewMode: ViewMode
  focusedInstanceId: string | null
  questionsQueueOpen: boolean
  settingsOpen: boolean
  newInstanceModalOpen: boolean

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

  setViewMode: (mode) => set({ viewMode: mode }),

  focusInstance: (instanceId) => set({
    viewMode: 'focus',
    focusedInstanceId: instanceId
  }),

  returnToOverview: () => set({
    viewMode: 'overview',
    focusedInstanceId: null
  }),

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
