import fs from 'fs'
import path from 'path'
import { app } from 'electron'

const MAX_LOG_SIZE = 5 * 1024 * 1024 // 5MB
const MAX_LOG_FILES = 5

class Logger {
  private logDir: string
  private logFile: string
  private stream: fs.WriteStream | null = null

  constructor() {
    // Use app.getPath('userData') for logs - works in packaged app
    this.logDir = app.isPackaged
      ? path.join(app.getPath('userData'), 'logs')
      : path.join(process.cwd(), 'logs')

    this.logFile = path.join(this.logDir, 'jenklaud.log')
    this.ensureLogDir()
    this.openStream()
  }

  private ensureLogDir(): void {
    if (!fs.existsSync(this.logDir)) {
      fs.mkdirSync(this.logDir, { recursive: true })
    }
  }

  private openStream(): void {
    this.stream = fs.createWriteStream(this.logFile, { flags: 'a' })
  }

  private rotate(): void {
    if (!fs.existsSync(this.logFile)) return

    const stats = fs.statSync(this.logFile)
    if (stats.size < MAX_LOG_SIZE) return

    // Close current stream
    this.stream?.end()

    // Rotate files
    for (let i = MAX_LOG_FILES - 1; i >= 1; i--) {
      const oldFile = `${this.logFile}.${i}`
      const newFile = `${this.logFile}.${i + 1}`
      if (fs.existsSync(oldFile)) {
        if (i === MAX_LOG_FILES - 1) {
          fs.unlinkSync(oldFile)
        } else {
          fs.renameSync(oldFile, newFile)
        }
      }
    }

    // Rename current log to .1
    fs.renameSync(this.logFile, `${this.logFile}.1`)

    // Open new stream
    this.openStream()
  }

  private formatMessage(level: string, message: string, meta?: object): string {
    const timestamp = new Date().toISOString()
    const metaStr = meta ? ` ${JSON.stringify(meta)}` : ''
    return `[${timestamp}] [${level}] ${message}${metaStr}\n`
  }

  private write(level: string, message: string, meta?: object): void {
    this.rotate()
    const formatted = this.formatMessage(level, message, meta)
    this.stream?.write(formatted)

    // Also log to console in development
    if (!app.isPackaged) {
      const consoleFn = level === 'ERROR' ? console.error : console.log
      consoleFn(`[${level}] ${message}`, meta || '')
    }
  }

  info(message: string, meta?: object): void {
    this.write('INFO', message, meta)
  }

  warn(message: string, meta?: object): void {
    this.write('WARN', message, meta)
  }

  error(message: string, meta?: object): void {
    this.write('ERROR', message, meta)
  }

  debug(message: string, meta?: object): void {
    this.write('DEBUG', message, meta)
  }

  getLogPath(): string {
    return this.logFile
  }

  close(): void {
    this.stream?.end()
  }
}

export const logger = new Logger()
