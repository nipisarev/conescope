import * as pty from 'node-pty'
import { BrowserWindow } from 'electron'
import { logger } from './logger'
import fs from 'fs'
import path from 'path'

interface ManagedInstance {
  id: string
  projectPath: string
  pty: pty.IPty
  isPaused: boolean
}

function getDefaultShell(): string {
  if (process.platform === 'win32') {
    return process.env.COMSPEC || 'cmd.exe'
  }

  // Try common shell paths
  const shells = [
    process.env.SHELL,
    '/bin/zsh',
    '/bin/bash',
    '/bin/sh'
  ].filter(Boolean) as string[]

  for (const shell of shells) {
    if (fs.existsSync(shell)) {
      return shell
    }
  }

  return '/bin/sh'
}

class InstanceManager {
  private instances: Map<string, ManagedInstance> = new Map()
  private mainWindow: BrowserWindow | null = null

  setMainWindow(window: BrowserWindow) {
    this.mainWindow = window
    logger.info('Main window set')
  }

  createInstance(id: string, projectPath: string): void {
    logger.info('Creating instance', { id, projectPath })

    // Validate project path
    if (!fs.existsSync(projectPath)) {
      const error = `Project path does not exist: ${projectPath}`
      logger.error(error)
      throw new Error(error)
    }

    const stat = fs.statSync(projectPath)
    if (!stat.isDirectory()) {
      const error = `Project path is not a directory: ${projectPath}`
      logger.error(error)
      throw new Error(error)
    }

    const shell = getDefaultShell()
    logger.info('Using shell', { shell })

    // Get environment with PATH properly set
    const env = { ...process.env } as { [key: string]: string }

    // Ensure common paths are in PATH for macOS
    if (process.platform === 'darwin') {
      const additionalPaths = [
        '/usr/local/bin',
        '/opt/homebrew/bin',
        '/usr/bin',
        '/bin',
        path.join(process.env.HOME || '', '.local/bin'),
        path.join(process.env.HOME || '', '.nvm/versions/node', process.version, 'bin')
      ]
      const currentPath = env.PATH || ''
      env.PATH = [...new Set([...additionalPaths, ...currentPath.split(':')])].join(':')
    }

    try {
      const ptyProcess = pty.spawn(shell, [], {
        name: 'xterm-256color',
        cols: 120,
        rows: 30,
        cwd: projectPath,
        env
      })

      logger.info('PTY process spawned', { id, pid: ptyProcess.pid })

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

      ptyProcess.onExit(({ exitCode, signal }) => {
        logger.info('PTY process exited', { id, exitCode, signal })
        if (this.mainWindow) {
          this.mainWindow.webContents.send('instance:status', id, 'stopped')
        }
        this.instances.delete(id)
      })

      this.instances.set(id, instance)

      // Start Claude Code CLI
      logger.info('Starting Claude CLI', { id })
      ptyProcess.write('claude\r')

      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'working')
      }
    } catch (error) {
      logger.error('Failed to spawn PTY', {
        id,
        projectPath,
        shell,
        error: error instanceof Error ? error.message : String(error),
        stack: error instanceof Error ? error.stack : undefined
      })
      throw error
    }
  }

  sendInput(id: string, input: string): void {
    const instance = this.instances.get(id)
    if (instance && !instance.isPaused) {
      instance.pty.write(input)
      logger.debug('Input sent', { id, inputLength: input.length })
    }
  }

  pauseInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.isPaused = true
      logger.info('Instance paused', { id })
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'paused')
      }
    }
  }

  resumeInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.isPaused = false
      logger.info('Instance resumed', { id })
      if (this.mainWindow) {
        this.mainWindow.webContents.send('instance:status', id, 'working')
      }
    }
  }

  killInstance(id: string): void {
    const instance = this.instances.get(id)
    if (instance) {
      logger.info('Killing instance', { id })
      instance.pty.kill()
      this.instances.delete(id)
    }
  }

  resizeInstance(id: string, cols: number, rows: number): void {
    const instance = this.instances.get(id)
    if (instance) {
      instance.pty.resize(cols, rows)
      logger.debug('Instance resized', { id, cols, rows })
    }
  }

  cleanup(): void {
    logger.info('Cleaning up all instances', { count: this.instances.size })
    for (const [id, instance] of this.instances) {
      instance.pty.kill()
    }
    this.instances.clear()
  }

  getInstanceCount(): number {
    return this.instances.size
  }
}

export const instanceManager = new InstanceManager()
