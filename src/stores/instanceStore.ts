import { create } from 'zustand'
import { v4 as uuid } from 'uuid'
import { Instance, InstanceStatus, PendingQuestion, InboxItem, Urgency } from '@/types'

interface InstanceStore {
  instances: Instance[]

  createInstance: (projectId: string, instanceId?: string) => Instance
  updateInstance: (id: string, updates: Partial<Instance>) => void
  removeInstance: (id: string) => void
  getInstance: (id: string) => Instance | undefined

  appendTerminalOutput: (id: string, output: string) => void
  setStatus: (id: string, status: InstanceStatus) => void
  setPendingQuestion: (id: string, question: PendingQuestion | undefined) => void

  getInboxItems: () => InboxItem[]
}

function calculateUrgency(askedAt: string): Urgency {
  const waitMs = Date.now() - new Date(askedAt).getTime()
  const waitMinutes = waitMs / 1000 / 60
  if (waitMinutes > 10) return 'urgent'
  if (waitMinutes > 5) return 'elevated'
  return 'normal'
}

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: [],

  createInstance: (projectId: string, instanceId?: string) => {
    const instance: Instance = {
      id: instanceId || uuid(),
      projectId,
      pid: null,
      status: 'starting',
      tokensUsed: 0,
      costEstimate: 0,
      startedAt: new Date().toISOString(),
      terminalHistory: []
    }
    set(state => ({ instances: [...state.instances, instance] }))
    return instance
  },

  updateInstance: (id, updates) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id ? { ...i, ...updates } : i
      )
    }))
  },

  removeInstance: (id) => {
    set(state => ({
      instances: state.instances.filter(i => i.id !== id)
    }))
  },

  getInstance: (id) => get().instances.find(i => i.id === id),

  appendTerminalOutput: (id, output) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id
          ? {
              ...i,
              terminalHistory: [...i.terminalHistory.slice(-500), output]
            }
          : i
      )
    }))
  },

  setStatus: (id, status) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id ? { ...i, status } : i
      )
    }))
  },

  setPendingQuestion: (id, question) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id
          ? {
              ...i,
              pendingQuestion: question,
              status: question ? 'waiting' : i.status
            }
          : i
      )
    }))
  },

  getInboxItems: () => {
    return get()
      .instances
      .filter(i => i.pendingQuestion)
      .map(i => ({
        instanceId: i.id,
        projectId: i.projectId,
        question: i.pendingQuestion!.text,
        askedAt: i.pendingQuestion!.askedAt,
        urgency: calculateUrgency(i.pendingQuestion!.askedAt),
        snoozed: false
      }))
      .sort((a, b) => new Date(a.askedAt).getTime() - new Date(b.askedAt).getTime())
  }
}))
