export interface Project {
  id: string
  path: string
  displayName: string
  color: string
  createdAt: string
  lastUsedAt: string
}

export type InstanceStatus = 'starting' | 'working' | 'waiting' | 'paused' | 'stopped'

export type InstanceType = 'project' | 'terminal'

export interface Instance {
  id: string
  projectId: string | null
  title: string
  instanceNumber: number
  status: InstanceStatus
  tokensUsed: number
  costEstimate: number
  startedAt: string
  terminalHistory: string[]
  type: InstanceType
  color: string | null
}

export interface Question {
  id: string
  instanceId: string
  questionText: string
  context?: string
  askedAt: string
  answeredAt?: string
  answer?: string
  snoozedUntil?: string
  // Joined fields
  instanceTitle?: string
  projectName?: string
  projectColor?: string
}

export const PROJECT_COLORS = [
  '#E57373', '#64B5F6', '#81C784', '#FFB74D', '#BA68C8',
  '#4DD0E1', '#FFD54F', '#A1887F', '#90A4AE',
] as const

export type ViewMode = 'overview' | 'focus'
