import * as pty from 'node-pty'
import { BrowserWindow } from 'electron'

interface ManagedInstance {
  id: string
  projectPath: string
  pty: pty.IPty
  isPaused: boolean
}

class InstanceManager {
  private instances: Map<string, ManagedInstance> = new Map()
  private mainWindow: BrowserWindow | null = null

  setMainWindow(window: BrowserWindow) {
    this.mainWindow = window
  }

  createInstance(id: string, projectPath: string): void {
    const shell = process.platform === 'win32' ? 'powershell.exe' : 'zsh'

    const ptyProcess = pty.spawn(shell, [], {
      name: 'xterm-256color',
      cols: 120,
      rows: 30,
      cwd: projectPath,
      env: process.env as { [key: string]: string }
    })

    const instance: ManagedInstance = {
      id,
      projectPath,
      pty: ptyProcess,
      isPaused: false
    }

    ptyProcess.onData((data) => {
      if (this.mainWindow && !instance.isPaused) {
        this.mainWindow.webContents.send('instance:output', id, data)
      }
    })

    ptyProcess.onExit(({ exitCode }) => {
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'stopped')
      }
      this.instances.delete(id)
    })

    this.instances.set(id, instance)

    // Start Claude Code CLI
    ptyProcess.write('claude\r')

    if (this.mainWindow) {
      this.mainWindow.webContents.send('instance:status', id, 'working')
    }
  }

  sendInput(id: string, input: string): void {
    const instance = this.instances.get(id)
    if (instance && !instance.isPaused) {
      instance.pty.write(input)
    }
  }

  pauseInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.isPaused = true
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'paused')
      }
    }
  }

  resumeInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.isPaused = false
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'working')
      }
    }
  }

  killInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.pty.kill()
      this.instances.delete(id)
    }
  }

  resizeInstance(id: string, cols: number, rows: number): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.pty.resize(cols, rows)
    }
  }

  cleanup(): void {
    for (const [id, instance] of this.instances) {
      instance.pty.kill()
    }
    this.instances.clear()
  }
}

export const instanceManager = new InstanceManager()
