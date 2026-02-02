export interface ElectronAPI {
  // Instance management
  createInstance: (projectPath: string) => Promise<string>
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
}

declare global {
  interface Window {
    electronAPI: ElectronAPI
  }
}

export {}
