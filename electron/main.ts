import { app, BrowserWindow, ipcMain, dialog } from 'electron'
import path from 'path'
import fs from 'fs'
import { instanceManager } from './instance-manager'

function createWindow() {
  const win = new BrowserWindow({
    width: 1400,
    height: 900,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    }
  })

  instanceManager.setMainWindow(win)

  if (process.env.NODE_ENV === 'development') {
    win.loadURL('http://localhost:5173')
  } else {
    win.loadFile(path.join(__dirname, '../dist/index.html'))
  }

  return win
}

// IPC Handlers
ipcMain.handle('instance:create', (_, projectPath: string) => {
  const id = crypto.randomUUID()
  instanceManager.createInstance(id, projectPath)
  return id
})

ipcMain.handle('instance:kill', (_, instanceId: string) => {
  instanceManager.killInstance(instanceId)
})

ipcMain.handle('instance:pause', (_, instanceId: string) => {
  instanceManager.pauseInstance(instanceId)
})

ipcMain.handle('instance:resume', (_, instanceId: string) => {
  instanceManager.resumeInstance(instanceId)
})

ipcMain.handle('instance:input', (_, instanceId: string, input: string) => {
  instanceManager.sendInput(instanceId, input)
})

ipcMain.handle('dialog:selectDirectory', async () => {
  const result = await dialog.showOpenDialog({
    properties: ['openDirectory']
  })
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

app.whenReady().then(createWindow)

app.on('window-all-closed', () => {
  instanceManager.cleanup()
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow()
  }
})
