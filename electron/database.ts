import Database from 'better-sqlite3'
import path from 'path'
import { app } from 'electron'
import { logger } from './logger'

const DB_NAME = 'conescope.db'

class DatabaseService {
  private db: Database.Database | null = null

  getDbPath(): string {
    const userDataPath = app.isPackaged
      ? app.getPath('userData')
      : path.join(process.cwd(), 'data')
    return path.join(userDataPath, DB_NAME)
  }

  initialize(): void {
    const dbPath = this.getDbPath()
    logger.info('Initializing database', { path: dbPath })

    // Ensure directory exists
    const fs = require('fs')
    const dir = path.dirname(dbPath)
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true })
    }

    this.db = new Database(dbPath)
    this.db.pragma('journal_mode = WAL')
    this.db.pragma('foreign_keys = ON')

    this.migrate()
    logger.info('Database initialized')
  }

  private migrate(): void {
    if (!this.db) throw new Error('Database not initialized')

    // Create tables
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        path TEXT UNIQUE NOT NULL,
        display_name TEXT NOT NULL,
        color TEXT NOT NULL,
        created_at TEXT NOT NULL,
        last_used_at TEXT NOT NULL
      );

      CREATE TABLE IF NOT EXISTS instances (
        id TEXT PRIMARY KEY,
        project_id TEXT,
        title TEXT,
        status TEXT NOT NULL DEFAULT 'starting',
        instance_number INTEGER,
        tokens_used INTEGER DEFAULT 0,
        cost_estimate REAL DEFAULT 0,
        started_at TEXT NOT NULL,
        ended_at TEXT,
        terminal_history TEXT,
        type TEXT NOT NULL DEFAULT 'project',
        color TEXT,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
      );

      CREATE TABLE IF NOT EXISTS questions (
        id TEXT PRIMARY KEY,
        instance_id TEXT NOT NULL,
        question_text TEXT NOT NULL,
        context TEXT,
        asked_at TEXT NOT NULL,
        answered_at TEXT,
        answer TEXT,
        snoozed_until TEXT,
        FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
      );

      CREATE TABLE IF NOT EXISTS settings (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
    `)

    // Insert default settings if not exist
    const insertSetting = this.db.prepare(
      'INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)'
    )
    insertSetting.run('theme', 'dark')
    insertSetting.run('questions_panel_visible', 'true')
    insertSetting.run('editor_font_size', '13')
    insertSetting.run('terminal_font_size', '13')

    // Migration: Add terminal_history column if it doesn't exist
    try {
      this.db.exec('ALTER TABLE instances ADD COLUMN terminal_history TEXT')
      logger.info('Added terminal_history column')
    } catch (e) {
      // Column already exists, ignore
    }

    // Migration: Add type/color columns and make project_id nullable
    try {
      this.db.exec("ALTER TABLE instances ADD COLUMN type TEXT NOT NULL DEFAULT 'project'")
      logger.info('Added type column to instances')
    } catch (e) {
      // Column already exists
    }
    try {
      this.db.exec("ALTER TABLE instances ADD COLUMN color TEXT")
      logger.info('Added color column to instances')
    } catch (e) {
      // Column already exists
    }

    // Migration: Rebuild instances table to make project_id nullable (for existing DBs)
    const colInfo = this.db.pragma('table_info(instances)') as { name: string; notnull: number }[]
    const projectIdCol = colInfo.find(c => c.name === 'project_id')
    if (projectIdCol && projectIdCol.notnull === 1) {
      logger.info('Rebuilding instances table to make project_id nullable')
      this.db.exec(`
        CREATE TABLE instances_new (
          id TEXT PRIMARY KEY,
          project_id TEXT,
          title TEXT,
          status TEXT NOT NULL DEFAULT 'starting',
          instance_number INTEGER,
          tokens_used INTEGER DEFAULT 0,
          cost_estimate REAL DEFAULT 0,
          started_at TEXT NOT NULL,
          ended_at TEXT,
          terminal_history TEXT,
          type TEXT NOT NULL DEFAULT 'project',
          color TEXT,
          FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        INSERT INTO instances_new SELECT id, project_id, title, status, instance_number, tokens_used, cost_estimate, started_at, ended_at, terminal_history, type, color FROM instances;
        DROP TABLE instances;
        ALTER TABLE instances_new RENAME TO instances;
      `)
      logger.info('Instances table rebuilt')
    }

    logger.info('Database migration complete')
  }

  // Projects
  getAllProjects(): any[] {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM projects ORDER BY last_used_at DESC').all()
  }

  getProject(id: string): any {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM projects WHERE id = ?').get(id)
  }

  insertProject(project: any): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      INSERT INTO projects (id, path, display_name, color, created_at, last_used_at)
      VALUES (?, ?, ?, ?, ?, ?)
    `).run(project.id, project.path, project.display_name, project.color, project.created_at, project.last_used_at)
  }

  updateProject(id: string, updates: any): void {
    if (!this.db) throw new Error('Database not initialized')
    const fields = Object.keys(updates).map(k => `${k} = ?`).join(', ')
    const values = [...Object.values(updates), id]
    this.db.prepare(`UPDATE projects SET ${fields} WHERE id = ?`).run(...values)
  }

  deleteProject(id: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare('DELETE FROM projects WHERE id = ?').run(id)
  }

  // Instances
  getAllInstances(): any[] {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM instances WHERE ended_at IS NULL ORDER BY started_at DESC').all()
  }

  getInstance(id: string): any {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare('SELECT * FROM instances WHERE id = ?').get(id)
  }

  insertInstance(instance: any): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      INSERT INTO instances (id, project_id, title, status, instance_number, tokens_used, cost_estimate, started_at, type, color)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      instance.id,
      instance.project_id,
      instance.title,
      instance.status,
      instance.instance_number,
      instance.tokens_used,
      instance.cost_estimate,
      instance.started_at,
      instance.type || 'project',
      instance.color || null
    )
  }

  updateInstance(id: string, updates: any): void {
    if (!this.db) throw new Error('Database not initialized')
    const fields = Object.keys(updates).map(k => `${k} = ?`).join(', ')
    const values = [...Object.values(updates), id]
    this.db.prepare(`UPDATE instances SET ${fields} WHERE id = ?`).run(...values)
  }

  deleteInstance(id: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare('DELETE FROM instances WHERE id = ?').run(id)
  }

  getNextInstanceNumber(): number {
    if (!this.db) throw new Error('Database not initialized')
    const result = this.db.prepare(
      'SELECT COALESCE(MAX(instance_number), 0) + 1 as next FROM instances WHERE ended_at IS NULL'
    ).get() as { next: number }
    return result.next
  }

  saveTerminalHistory(id: string, history: string[]): void {
    if (!this.db) throw new Error('Database not initialized')
    const json = JSON.stringify(history.slice(-500)) // Keep last 500 chunks
    this.db.prepare('UPDATE instances SET terminal_history = ? WHERE id = ?').run(json, id)
  }

  getTerminalHistory(id: string): string[] {
    if (!this.db) throw new Error('Database not initialized')
    const row = this.db.prepare('SELECT terminal_history FROM instances WHERE id = ?').get(id) as { terminal_history: string | null } | undefined
    if (!row?.terminal_history) return []
    try {
      return JSON.parse(row.terminal_history)
    } catch {
      return []
    }
  }

  // Settings
  getSetting(key: string): string | null {
    if (!this.db) throw new Error('Database not initialized')
    const row = this.db.prepare('SELECT value FROM settings WHERE key = ?').get(key) as { value: string } | undefined
    return row?.value ?? null
  }

  setSetting(key: string, value: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare('INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)').run(key, value)
  }

  getAllSettings(): Record<string, string> {
    if (!this.db) throw new Error('Database not initialized')
    const rows = this.db.prepare('SELECT key, value FROM settings').all() as { key: string; value: string }[]
    return Object.fromEntries(rows.map(r => [r.key, r.value]))
  }

  // Questions
  getPendingQuestions(): any[] {
    if (!this.db) throw new Error('Database not initialized')
    return this.db.prepare(`
      SELECT q.*, i.title as instance_title, p.display_name as project_name, p.color as project_color
      FROM questions q
      JOIN instances i ON q.instance_id = i.id
      LEFT JOIN projects p ON i.project_id = p.id
      WHERE q.answered_at IS NULL
      ORDER BY q.asked_at ASC
    `).all()
  }

  insertQuestion(question: any): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      INSERT INTO questions (id, instance_id, question_text, context, asked_at)
      VALUES (?, ?, ?, ?, ?)
    `).run(question.id, question.instance_id, question.question_text, question.context, question.asked_at)
  }

  answerQuestion(id: string, answer: string): void {
    if (!this.db) throw new Error('Database not initialized')
    this.db.prepare(`
      UPDATE questions SET answered_at = ?, answer = ? WHERE id = ?
    `).run(new Date().toISOString(), answer, id)
  }

  close(): void {
    if (this.db) {
      this.db.close()
      this.db = null
      logger.info('Database closed')
    }
  }
}

export const database = new DatabaseService()
