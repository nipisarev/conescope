import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('electronAPI', {
  // Instance management
  createInstance: (projectPath: string) =>
    ipcRenderer.invoke('instance:create', projectPath),
  killInstance: (instanceId: string) =>
    ipcRenderer.invoke('instance:kill', instanceId),
  pauseInstance: (instanceId: string) =>
    ipcRenderer.invoke('instance:pause', instanceId),
  resumeInstance: (instanceId: string) =>
    ipcRenderer.invoke('instance:resume', instanceId),
  sendInput: (instanceId: string, input: string) =>
    ipcRenderer.invoke('instance:input', instanceId, input),

  // Event listeners
  onInstanceOutput: (callback: (instanceId: string, data: string) => void) => {
    ipcRenderer.on('instance:output', (_, instanceId, data) => callback(instanceId, data))
  },
  onInstanceStatusChange: (callback: (instanceId: string, status: string) => void) => {
    ipcRenderer.on('instance:status', (_, instanceId, status) => callback(instanceId, status))
  },

  // File system
  selectDirectory: () => ipcRenderer.invoke('dialog:selectDirectory'),
  readDirectory: (path: string) => ipcRenderer.invoke('fs:readDirectory', path),
  readFile: (path: string) => ipcRenderer.invoke('fs:readFile', path),
  writeFile: (path: string, content: string) => ipcRenderer.invoke('fs:writeFile', path, content)
})
