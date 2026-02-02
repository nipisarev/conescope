import { create } from 'zustand'
import { v4 as uuid } from 'uuid'
import { Project, PROJECT_COLORS } from '@/types'

interface ProjectStore {
  projects: Project[]
  isLoading: boolean

  loadProjects: () => Promise<void>
  addProject: (path: string, displayName?: string) => Promise<Project>
  updateProject: (id: string, updates: Partial<Project>) => Promise<void>
  deleteProject: (id: string) => Promise<void>
  getProject: (id: string) => Project | undefined
  getNextColor: () => string
}

export const useProjectStore = create<ProjectStore>((set, get) => ({
  projects: [],
  isLoading: true,

  loadProjects: async () => {
    const dbProjects = await window.electronAPI.dbProjectsGetAll()
    const projects: Project[] = dbProjects.map(p => ({
      id: p.id,
      path: p.path,
      displayName: p.display_name,
      color: p.color,
      createdAt: p.created_at,
      lastUsedAt: p.last_used_at,
    }))
    set({ projects, isLoading: false })
  },

  addProject: async (path: string, displayName?: string) => {
    const existing = get().projects.find(p => p.path === path)
    if (existing) {
      const updates = { lastUsedAt: new Date().toISOString() }
      await window.electronAPI.dbProjectsUpdate(existing.id, { last_used_at: updates.lastUsedAt })
      set(state => ({
        projects: state.projects.map(p =>
          p.id === existing.id ? { ...p, ...updates } : p
        )
      }))
      return existing
    }

    const project: Project = {
      id: uuid(),
      path,
      displayName: displayName || path.split('/').pop() || path,
      color: get().getNextColor(),
      createdAt: new Date().toISOString(),
      lastUsedAt: new Date().toISOString(),
    }

    await window.electronAPI.dbProjectsInsert({
      id: project.id,
      path: project.path,
      display_name: project.displayName,
      color: project.color,
      created_at: project.createdAt,
      last_used_at: project.lastUsedAt,
    })

    set(state => ({ projects: [...state.projects, project] }))
    return project
  },

  updateProject: async (id, updates) => {
    const dbUpdates: Record<string, any> = {}
    if (updates.displayName) dbUpdates.display_name = updates.displayName
    if (updates.color) dbUpdates.color = updates.color
    if (updates.lastUsedAt) dbUpdates.last_used_at = updates.lastUsedAt

    await window.electronAPI.dbProjectsUpdate(id, dbUpdates)
    set(state => ({
      projects: state.projects.map(p => p.id === id ? { ...p, ...updates } : p)
    }))
  },

  deleteProject: async (id) => {
    await window.electronAPI.dbProjectsDelete(id)
    set(state => ({ projects: state.projects.filter(p => p.id !== id) }))
  },

  getProject: (id) => get().projects.find(p => p.id === id),

  getNextColor: () => {
    const usedColors = get().projects.map(p => p.color)
    const available = PROJECT_COLORS.filter(c => !usedColors.includes(c))
    return available[0] || PROJECT_COLORS[get().projects.length % PROJECT_COLORS.length]
  },
}))
