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
  writeFile: (path: string, content: string) => ipcRenderer.invoke('fs:writeFile', path, content),

  // Database - Projects
  dbProjectsGetAll: () => ipcRenderer.invoke('db:projects:getAll'),
  dbProjectsGet: (id: string) => ipcRenderer.invoke('db:projects:get', id),
  dbProjectsInsert: (project: any) => ipcRenderer.invoke('db:projects:insert', project),
  dbProjectsUpdate: (id: string, updates: any) => ipcRenderer.invoke('db:projects:update', id, updates),
  dbProjectsDelete: (id: string) => ipcRenderer.invoke('db:projects:delete', id),

  // Database - Instances
  dbInstancesGetAll: () => ipcRenderer.invoke('db:instances:getAll'),
  dbInstancesGet: (id: string) => ipcRenderer.invoke('db:instances:get', id),
  dbInstancesInsert: (instance: any) => ipcRenderer.invoke('db:instances:insert', instance),
  dbInstancesUpdate: (id: string, updates: any) => ipcRenderer.invoke('db:instances:update', id, updates),
  dbInstancesDelete: (id: string) => ipcRenderer.invoke('db:instances:delete', id),
  dbInstancesGetNextNumber: () => ipcRenderer.invoke('db:instances:getNextNumber'),

  // Database - Settings
  dbSettingsGet: (key: string) => ipcRenderer.invoke('db:settings:get', key),
  dbSettingsSet: (key: string, value: string) => ipcRenderer.invoke('db:settings:set', key, value),
  dbSettingsGetAll: () => ipcRenderer.invoke('db:settings:getAll'),

  // Database - Questions
  dbQuestionsGetPending: () => ipcRenderer.invoke('db:questions:getPending'),
  dbQuestionsInsert: (question: any) => ipcRenderer.invoke('db:questions:insert', question),
  dbQuestionsAnswer: (id: string, answer: string) => ipcRenderer.invoke('db:questions:answer', id, answer),
})
