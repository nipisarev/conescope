export interface DbProject {
  id: string
  path: string
  display_name: string
  color: string
  created_at: string
  last_used_at: string
}

export interface DbInstance {
  id: string
  project_id: string
  title: string | null
  status: string
  instance_number: number
  tokens_used: number
  cost_estimate: number
  started_at: string
  ended_at: string | null
}

export interface DbQuestion {
  id: string
  instance_id: string
  question_text: string
  context: string | null
  asked_at: string
  answered_at: string | null
  answer: string | null
  snoozed_until: string | null
  instance_title?: string
  project_name?: string
  project_color?: string
}

export interface ElectronAPI {
  // Instance management
  createInstance: (instanceId: string, projectPath: string) => Promise<string>
  killInstance: (instanceId: string) => Promise<void>
  pauseInstance: (instanceId: string) => Promise<void>
  resumeInstance: (instanceId: string) => Promise<void>
  sendInput: (instanceId: string, input: string) => Promise<void>

  // Event listeners
  onInstanceOutput: (callback: (instanceId: string, data: string) => void) => void
  onInstanceStatusChange: (callback: (instanceId: string, status: string) => void) => void

  // File system
  selectDirectory: () => Promise<string | null>
  readDirectory: (path: string) => Promise<Array<{
    name: string
    isDirectory: boolean
    path: string
  }>>
  readFile: (path: string) => Promise<string | null>
  writeFile: (path: string, content: string) => Promise<boolean>

  // Database - Projects
  dbProjectsGetAll: () => Promise<DbProject[]>
  dbProjectsGet: (id: string) => Promise<DbProject | undefined>
  dbProjectsInsert: (project: DbProject) => Promise<void>
  dbProjectsUpdate: (id: string, updates: Partial<DbProject>) => Promise<void>
  dbProjectsDelete: (id: string) => Promise<void>

  // Database - Instances
  dbInstancesGetAll: () => Promise<DbInstance[]>
  dbInstancesGet: (id: string) => Promise<DbInstance | undefined>
  dbInstancesInsert: (instance: DbInstance) => Promise<void>
  dbInstancesUpdate: (id: string, updates: Partial<DbInstance>) => Promise<void>
  dbInstancesDelete: (id: string) => Promise<void>
  dbInstancesGetNextNumber: () => Promise<number>

  // Database - Settings
  dbSettingsGet: (key: string) => Promise<string | null>
  dbSettingsSet: (key: string, value: string) => Promise<void>
  dbSettingsGetAll: () => Promise<Record<string, string>>

  // Database - Questions
  dbQuestionsGetPending: () => Promise<DbQuestion[]>
  dbQuestionsInsert: (question: Omit<DbQuestion, 'answered_at' | 'answer' | 'snoozed_until'>) => Promise<void>
  dbQuestionsAnswer: (id: string, answer: string) => Promise<void>
}

declare global {
  interface Window {
    electronAPI: ElectronAPI
  }
}

export {}
