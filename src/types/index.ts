export interface Project {
  id: string
  path: string
  displayName: string
  color: string
  createdAt: string
  lastUsedAt: string
}

export type InstanceStatus = 'starting' | 'working' | 'waiting' | 'paused' | 'stopped'

export interface PendingQuestion {
  text: string
  askedAt: string
  context?: string
}

export interface Instance {
  id: string
  projectId: string
  pid: number | null
  status: InstanceStatus
  tokensUsed: number
  costEstimate: number
  startedAt: string
  pendingQuestion?: PendingQuestion
  terminalHistory: string[]
}

export type Urgency = 'normal' | 'elevated' | 'urgent'

export interface InboxItem {
  instanceId: string
  projectId: string
  question: string
  askedAt: string
  urgency: Urgency
  snoozed: boolean
}

export const PROJECT_COLORS = [
  '#E57373', // red
  '#64B5F6', // blue
  '#81C784', // green
  '#FFB74D', // orange
  '#BA68C8', // purple
  '#4DD0E1', // cyan
  '#FFD54F', // yellow
  '#A1887F', // brown
  '#90A4AE', // gray
] as const

export type ViewMode = 'overview' | 'focus'

export interface AppState {
  viewMode: ViewMode
  focusedInstanceId: string | null
  questionsQueueOpen: boolean
  settingsOpen: boolean
}
