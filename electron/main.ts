import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import path from 'path'
import fs from 'fs'
import { instanceManager } from './instance-manager'
import { logger } from './logger'
import { database } from './database'

function createWindow() {
  logger.info('Creating main window')

  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    titleBarStyle: 'hidden',
    trafficLightPosition: { x: 12, y: 12 },
    backgroundColor: '#1e1e1e',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  instanceManager.setMainWindow(win)

  if (process.env.NODE_ENV === 'development') {
    logger.info('Loading development URL')
    win.loadURL('http://localhost:5173')
  } else {
    const indexPath = path.join(__dirname, '../dist/index.html')
    logger.info('Loading production file', { path: indexPath })
    win.loadFile(indexPath)
  }

  return win
}

// IPC Handlers
ipcMain.handle('instance:create', async (_, instanceId: string, projectPath: string) => {
  logger.info('IPC: instance:create', { instanceId, projectPath })
  try {
    instanceManager.createInstance(instanceId, projectPath)
    return instanceId
  } catch (error) {
    logger.error('IPC: instance:create failed', {
      instanceId,
      projectPath,
      error: error instanceof Error ? error.message : String(error)
    })
    throw error
  }
})

ipcMain.handle('instance:kill', (_, instanceId: string) => {
  logger.info('IPC: instance:kill', { instanceId })
  instanceManager.killInstance(instanceId)
})

ipcMain.handle('instance:pause', (_, instanceId: string) => {
  logger.info('IPC: instance:pause', { instanceId })
  instanceManager.pauseInstance(instanceId)
})

ipcMain.handle('instance:resume', (_, instanceId: string) => {
  logger.info('IPC: instance:resume', { instanceId })
  instanceManager.resumeInstance(instanceId)
})

ipcMain.handle('instance:input', (_, instanceId: string, input: string) => {
  instanceManager.sendInput(instanceId, input)
})

ipcMain.handle('instance:resize', (_, instanceId: string, cols: number, rows: number) => {
  instanceManager.resizeInstance(instanceId, cols, rows)
})

ipcMain.handle('dialog:selectDirectory', async () => {
  logger.info('IPC: dialog:selectDirectory')
  const result = await dialog.showOpenDialog({
    properties: ['openDirectory']
  })
  logger.info('IPC: dialog:selectDirectory result', { canceled: result.canceled, path: result.filePaths[0] })
  return result.canceled ? null : result.filePaths[0]
})

ipcMain.handle('fs:readDirectory', async (_, dirPath: string) => {
  try {
    const entries = await fs.promises.readdir(dirPath, { withFileTypes: true })
    return entries.map(entry => ({
      name: entry.name,
      isDirectory: entry.isDirectory(),
      path: path.join(dirPath, entry.name)
    }))
  } catch {
    return []
  }
})

ipcMain.handle('fs:readFile', async (_, filePath: string) => {
  try {
    return await fs.promises.readFile(filePath, 'utf-8')
  } catch {
    return null
  }
})

ipcMain.handle('fs:writeFile', async (_, filePath: string, content: string) => {
  try {
    await fs.promises.writeFile(filePath, content, 'utf-8')
    return true
  } catch {
    return false
  }
})

// Database IPC Handlers
ipcMain.handle('db:projects:getAll', () => database.getAllProjects())
ipcMain.handle('db:projects:get', (_, id: string) => database.getProject(id))
ipcMain.handle('db:projects:insert', (_, project) => database.insertProject(project))
ipcMain.handle('db:projects:update', (_, id: string, updates) => database.updateProject(id, updates))
ipcMain.handle('db:projects:delete', (_, id: string) => database.deleteProject(id))

ipcMain.handle('db:instances:getAll', () => database.getAllInstances())
ipcMain.handle('db:instances:get', (_, id: string) => database.getInstance(id))
ipcMain.handle('db:instances:insert', (_, instance) => database.insertInstance(instance))
ipcMain.handle('db:instances:update', (_, id: string, updates) => database.updateInstance(id, updates))
ipcMain.handle('db:instances:delete', (_, id: string) => database.deleteInstance(id))
ipcMain.handle('db:instances:getNextNumber', () => database.getNextInstanceNumber())
ipcMain.handle('db:instances:saveTerminalHistory', (_, id: string, history: string[]) => database.saveTerminalHistory(id, history))
ipcMain.handle('db:instances:getTerminalHistory', (_, id: string) => database.getTerminalHistory(id))

ipcMain.handle('db:settings:get', (_, key: string) => database.getSetting(key))
ipcMain.handle('db:settings:set', (_, key: string, value: string) => database.setSetting(key, value))
ipcMain.handle('db:settings:getAll', () => database.getAllSettings())

ipcMain.handle('db:questions:getPending', () => database.getPendingQuestions())
ipcMain.handle('db:questions:insert', (_, question) => database.insertQuestion(question))
ipcMain.handle('db:questions:answer', (_, id: string, answer: string) => database.answerQuestion(id, answer))

logger.info('Jenklaud starting', {
  nodeVersion: process.version,
  electronVersion: process.versions.electron,
  platform: process.platform,
  arch: process.arch
})

app.whenReady().then(() => {
  logger.info('App ready')
  database.initialize()
  createWindow()
})

app.on('window-all-closed', () => {
  logger.info('All windows closed')
  instanceManager.cleanup()
  database.close()
  logger.close()
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow()
  }
})
