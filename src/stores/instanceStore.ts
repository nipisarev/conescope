import { create } from 'zustand'
import { v4 as uuid } from 'uuid'
import { Instance, InstanceStatus, InstanceType, PROJECT_COLORS } from '@/types'

interface InstanceStore {
  instances: Instance[]
  isLoading: boolean

  loadInstances: () => Promise<void>
  restoreInstance: (id: string, projectPath: string) => Promise<void>
  createInstance: (projectId: string, projectName: string) => Promise<Instance>
  createTerminalInstance: () => Promise<Instance>
  updateInstance: (id: string, updates: Partial<Instance>) => void
  updateInstanceDb: (id: string, updates: Partial<Instance>) => Promise<void>
  removeInstance: (id: string) => Promise<void>
  getInstance: (id: string) => Instance | undefined
  getInstanceByNumber: (num: number) => Instance | undefined
  getDisplayNumber: (id: string) => number

  appendTerminalOutput: (id: string, output: string) => void
  setStatus: (id: string, status: InstanceStatus) => void
}

export const useInstanceStore = create<InstanceStore>((set, get) => ({
  instances: [],
  isLoading: true,

  loadInstances: async () => {
    const dbInstances = await window.electronAPI.dbInstancesGetAll()
    const instances: Instance[] = dbInstances.map(i => ({
      id: i.id,
      projectId: i.project_id || null,
      title: i.title || '',
      instanceNumber: i.instance_number,
      status: 'starting' as InstanceStatus,
      tokensUsed: i.tokens_used,
      costEstimate: i.cost_estimate,
      startedAt: i.started_at,
      terminalHistory: [],
      type: (i.type || 'project') as InstanceType,
      color: i.color || null,
    }))
    set({ instances, isLoading: false })
  },

  restoreInstance: async (id: string, projectPath: string) => {
    await window.electronAPI.createInstance(id, projectPath)
  },

  createInstance: async (projectId: string, projectName: string) => {
    const instanceNumber = await window.electronAPI.dbInstancesGetNextNumber()
    const instance: Instance = {
      id: uuid(),
      projectId,
      title: projectName,
      instanceNumber,
      status: 'starting',
      tokensUsed: 0,
      costEstimate: 0,
      startedAt: new Date().toISOString(),
      terminalHistory: [],
      type: 'project',
      color: null,
    }

    await window.electronAPI.dbInstancesInsert({
      id: instance.id,
      project_id: instance.projectId,
      title: instance.title,
      status: instance.status,
      instance_number: instance.instanceNumber,
      tokens_used: instance.tokensUsed,
      cost_estimate: instance.costEstimate,
      started_at: instance.startedAt,
      type: instance.type,
      color: instance.color,
    })

    set(state => ({ instances: [...state.instances, instance] }))
    return instance
  },

  createTerminalInstance: async () => {
    const instanceNumber = await window.electronAPI.dbInstancesGetNextNumber()
    const currentCount = get().instances.length
    const color = PROJECT_COLORS[currentCount % PROJECT_COLORS.length]

    const instance: Instance = {
      id: uuid(),
      projectId: null,
      title: 'Shell',
      instanceNumber,
      status: 'starting',
      tokensUsed: 0,
      costEstimate: 0,
      startedAt: new Date().toISOString(),
      terminalHistory: [],
      type: 'terminal',
      color,
    }

    await window.electronAPI.dbInstancesInsert({
      id: instance.id,
      project_id: null,
      title: instance.title,
      status: instance.status,
      instance_number: instance.instanceNumber,
      tokens_used: instance.tokensUsed,
      cost_estimate: instance.costEstimate,
      started_at: instance.startedAt,
      type: instance.type,
      color: instance.color,
    })

    set(state => ({ instances: [...state.instances, instance] }))
    return instance
  },

  updateInstance: (id, updates) => {
    set(state => ({
      instances: state.instances.map(i => i.id === id ? { ...i, ...updates } : i)
    }))
  },

  updateInstanceDb: async (id, updates) => {
    const dbUpdates: Record<string, any> = {}
    if (updates.title !== undefined) dbUpdates.title = updates.title
    if (updates.status !== undefined) dbUpdates.status = updates.status
    if (updates.tokensUsed !== undefined) dbUpdates.tokens_used = updates.tokensUsed
    if (updates.costEstimate !== undefined) dbUpdates.cost_estimate = updates.costEstimate

    await window.electronAPI.dbInstancesUpdate(id, dbUpdates)
    get().updateInstance(id, updates)
  },

  removeInstance: async (id) => {
    await window.electronAPI.dbInstancesUpdate(id, { ended_at: new Date().toISOString() })
    set(state => ({ instances: state.instances.filter(i => i.id !== id) }))
  },

  getInstance: (id) => get().instances.find(i => i.id === id),

  getInstanceByNumber: (num) => get().instances.find(i => i.instanceNumber === num),

  getDisplayNumber: (id) => {
    const sorted = [...get().instances].sort((a, b) => a.instanceNumber - b.instanceNumber)
    const index = sorted.findIndex(i => i.id === id)
    return index + 1
  },

  appendTerminalOutput: (id, output) => {
    set(state => ({
      instances: state.instances.map(i =>
        i.id === id
          ? { ...i, terminalHistory: [...i.terminalHistory.slice(-500), output] }
          : i
      )
    }))
  },

  setStatus: (id, status) => {
    get().updateInstance(id, { status })
    // Don't await, fire and forget for performance
    window.electronAPI.dbInstancesUpdate(id, { status })
  },
}))
