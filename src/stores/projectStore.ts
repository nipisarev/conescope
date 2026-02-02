import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { v4 as uuid } from 'uuid'
import { Project, PROJECT_COLORS } from '@/types'

interface ProjectStore {
  projects: Project[]
  addProject: (path: string, displayName?: string) => Project
  updateProject: (id: string, updates: Partial<Project>) => void
  deleteProject: (id: string) => void
  getProject: (id: string) => Project | undefined
  getNextColor: () => string
}

export const useProjectStore = create<ProjectStore>()(
  persist(
    (set, get) => ({
      projects: [],

      addProject: (path: string, displayName?: string) => {
        const existing = get().projects.find(p => p.path === path)
        if (existing) {
          set(state => ({
            projects: state.projects.map(p =>
              p.id === existing.id
                ? { ...p, lastUsedAt: new Date().toISOString() }
                : p
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
          lastUsedAt: new Date().toISOString()
        }

        set(state => ({ projects: [...state.projects, project] }))
        return project
      },

      updateProject: (id, updates) => {
        set(state => ({
          projects: state.projects.map(p =>
            p.id === id ? { ...p, ...updates } : p
          )
        }))
      },

      deleteProject: (id) => {
        set(state => ({
          projects: state.projects.filter(p => p.id !== id)
        }))
      },

      getProject: (id) => get().projects.find(p => p.id === id),

      getNextColor: () => {
        const usedColors = get().projects.map(p => p.color)
        const available = PROJECT_COLORS.filter(c => !usedColors.includes(c))
        return available[0] || PROJECT_COLORS[get().projects.length % PROJECT_COLORS.length]
      }
    }),
    {
      name: 'jenklaud-projects'
    }
  )
)
