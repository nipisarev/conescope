import { create } from 'zustand'
import { v4 as uuid } from 'uuid'

export interface TerminalTab {
  id: string
  instanceId: string
  type: 'claude' | 'shell'
  title: string
  history: string[]
}

interface TerminalTabStore {
  tabs: TerminalTab[]
  activeTabId: Record<string, string> // instanceId -> activeTabId

  // Actions
  initializeClaudeTab: (instanceId: string) => void
  addShellTab: (instanceId: string, projectPath: string) => Promise<TerminalTab>
  removeTab: (tabId: string) => void
  setActiveTab: (instanceId: string, tabId: string) => void
  appendOutput: (tabId: string, output: string) => void
  cleanupInstance: (instanceId: string) => void
}

export const useTerminalTabStore = create<TerminalTabStore>((set, get) => ({
  tabs: [],
  activeTabId: {},

  initializeClaudeTab: (instanceId: string) => {
    const existingTab = get().tabs.find(t => t.instanceId === instanceId && t.type === 'claude')
    if (existingTab) return

    const claudeTab: TerminalTab = {
      id: `claude-${instanceId}`,
      instanceId,
      type: 'claude',
      title: 'Claude',
      history: []
    }

    set(state => ({
      tabs: [...state.tabs, claudeTab],
      activeTabId: { ...state.activeTabId, [instanceId]: claudeTab.id }
    }))
  },

  addShellTab: async (instanceId: string, projectPath: string) => {
    const tabId = uuid()
    const shellCount = get().tabs.filter(t => t.instanceId === instanceId && t.type === 'shell').length

    const shellTab: TerminalTab = {
      id: tabId,
      instanceId,
      type: 'shell',
      title: `Shell ${shellCount + 1}`,
      history: []
    }

    // Create the PTY process
    await window.electronAPI.createShellTerminal(tabId, projectPath)

    set(state => ({
      tabs: [...state.tabs, shellTab],
      activeTabId: { ...state.activeTabId, [instanceId]: tabId }
    }))

    return shellTab
  },

  removeTab: (tabId: string) => {
    const tab = get().tabs.find(t => t.id === tabId)
    if (!tab || tab.type === 'claude') return // Can't remove Claude tab

    // Kill the PTY
    window.electronAPI.killShellTerminal(tabId)

    set(state => {
      const newTabs = state.tabs.filter(t => t.id !== tabId)
      const newActiveTabId = { ...state.activeTabId }

      // If removing active tab, switch to Claude tab
      if (state.activeTabId[tab.instanceId] === tabId) {
        const claudeTab = newTabs.find(t => t.instanceId === tab.instanceId && t.type === 'claude')
        if (claudeTab) {
          newActiveTabId[tab.instanceId] = claudeTab.id
        }
      }

      return { tabs: newTabs, activeTabId: newActiveTabId }
    })
  },

  setActiveTab: (instanceId: string, tabId: string) => {
    set(state => ({
      activeTabId: { ...state.activeTabId, [instanceId]: tabId }
    }))
  },

  appendOutput: (tabId: string, output: string) => {
    set(state => ({
      tabs: state.tabs.map(t =>
        t.id === tabId
          ? { ...t, history: [...t.history.slice(-500), output] }
          : t
      )
    }))
  },

  cleanupInstance: (instanceId: string) => {
    // Kill all shell tabs for this instance
    const shellTabs = get().tabs.filter(t => t.instanceId === instanceId && t.type === 'shell')
    for (const tab of shellTabs) {
      window.electronAPI.killShellTerminal(tab.id)
    }

    set(state => ({
      tabs: state.tabs.filter(t => t.instanceId !== instanceId),
      activeTabId: Object.fromEntries(
        Object.entries(state.activeTabId).filter(([key]) => key !== instanceId)
      )
    }))
  }
}))
