#![allow(clippy::missing_errors_doc)]

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use rusqlite_migration::{M, Migrations};

use crate::instance::{Instance, InstanceStatus, InstanceType};
use crate::project::Project;
use crate::question::{Question, QuestionWithContext};
use crate::settings;

const MIGRATIONS: &[M<'_>] = &[M::up(
    "CREATE TABLE IF NOT EXISTS projects (
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
    );",
)];

#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open database")?;
        let mut db = Self { conn };
        db.init()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("Failed to open in-memory database")?;
        let mut db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&mut self) -> Result<()> {
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        self.conn.pragma_update(None, "foreign_keys", "ON")?;

        let migrations = Migrations::from_slice(MIGRATIONS);
        migrations
            .to_latest(&mut self.conn)
            .context("Failed to run migrations")?;

        self.insert_default_settings()?;
        Ok(())
    }

    fn insert_default_settings(&self) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")?;
        for (key, value) in settings::DEFAULTS {
            stmt.execute(params![key, value])?;
        }
        Ok(())
    }

    // --- Projects ---

    pub fn get_all_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, display_name, color, created_at, last_used_at FROM projects ORDER BY last_used_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                path: row.get(1)?,
                display_name: row.get(2)?,
                color: row.get(3)?,
                created_at: row.get(4)?,
                last_used_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to get projects")
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, display_name, color, created_at, last_used_at FROM projects WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(Project {
                id: row.get(0)?,
                path: row.get(1)?,
                display_name: row.get(2)?,
                color: row.get(3)?,
                created_at: row.get(4)?,
                last_used_at: row.get(5)?,
            })
        })?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn insert_project(&self, project: &Project) -> Result<()> {
        self.conn.execute(
            "INSERT INTO projects (id, path, display_name, color, created_at, last_used_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                project.id,
                project.path,
                project.display_name,
                project.color,
                project.created_at,
                project.last_used_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_project_last_used(&self, id: &str, last_used_at: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET last_used_at = ?1 WHERE id = ?2",
            params![last_used_at, id],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Instances ---

    pub fn get_all_instances(&self) -> Result<Vec<Instance>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title, status, instance_number, tokens_used, cost_estimate, started_at, ended_at, type, color
             FROM instances WHERE ended_at IS NULL ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(3)?;
            let type_str: String = row.get(9)?;
            Ok(Instance {
                id: row.get(0)?,
                project_id: row.get(1)?,
                title: row.get(2)?,
                status: InstanceStatus::from_str_opt(&status_str)
                    .unwrap_or(InstanceStatus::Starting),
                instance_number: row.get(4)?,
                tokens_used: row.get(5)?,
                cost_estimate: row.get(6)?,
                started_at: row.get(7)?,
                ended_at: row.get(8)?,
                instance_type: InstanceType::from_str_opt(&type_str)
                    .unwrap_or(InstanceType::Project),
                color: row.get(10)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to get instances")
    }

    pub fn insert_instance(&self, inst: &Instance) -> Result<()> {
        self.conn.execute(
            "INSERT INTO instances (id, project_id, title, status, instance_number, tokens_used, cost_estimate, started_at, type, color)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                inst.id,
                inst.project_id,
                inst.title,
                inst.status.as_str(),
                inst.instance_number,
                inst.tokens_used,
                inst.cost_estimate,
                inst.started_at,
                inst.instance_type.as_str(),
                inst.color,
            ],
        )?;
        Ok(())
    }

    pub fn update_instance_status(&self, id: &str, status: InstanceStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        Ok(())
    }

    pub fn end_instance(&self, id: &str, ended_at: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET status = 'stopped', ended_at = ?1 WHERE id = ?2",
            params![ended_at, id],
        )?;
        Ok(())
    }

    pub fn delete_instance(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM instances WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_next_instance_number(&self) -> Result<i64> {
        let n: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(instance_number), 0) + 1 FROM instances WHERE ended_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(n)
    }

    pub fn save_terminal_history(&self, id: &str, history_json: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE instances SET terminal_history = ?1 WHERE id = ?2",
            params![history_json, id],
        )?;
        Ok(())
    }

    pub fn get_terminal_history(&self, id: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT terminal_history FROM instances WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], |row| row.get(0))?;
        match rows.next() {
            Some(row) => Ok(row?),
            None => Ok(None),
        }
    }

    // --- Settings ---

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get(0))?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to get settings")
    }

    // --- Questions ---

    pub fn get_pending_questions(&self) -> Result<Vec<QuestionWithContext>> {
        let mut stmt = self.conn.prepare(
            "SELECT q.id, q.instance_id, q.question_text, q.context, q.asked_at,
                    q.answered_at, q.answer, q.snoozed_until,
                    i.title, p.display_name, p.color
             FROM questions q
             JOIN instances i ON q.instance_id = i.id
             LEFT JOIN projects p ON i.project_id = p.id
             WHERE q.answered_at IS NULL
             ORDER BY q.asked_at ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(QuestionWithContext {
                question: Question {
                    id: row.get(0)?,
                    instance_id: row.get(1)?,
                    question_text: row.get(2)?,
                    context: row.get(3)?,
                    asked_at: row.get(4)?,
                    answered_at: row.get(5)?,
                    answer: row.get(6)?,
                    snoozed_until: row.get(7)?,
                },
                instance_title: row.get(8)?,
                project_name: row.get(9)?,
                project_color: row.get(10)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("Failed to get pending questions")
    }

    pub fn insert_question(&self, q: &Question) -> Result<()> {
        self.conn.execute(
            "INSERT INTO questions (id, instance_id, question_text, context, asked_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![q.id, q.instance_id, q.question_text, q.context, q.asked_at],
        )?;
        Ok(())
    }

    pub fn answer_question(&self, id: &str, answer: &str, answered_at: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE questions SET answered_at = ?1, answer = ?2 WHERE id = ?3",
            params![answered_at, answer, id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().expect("Failed to create test DB")
    }

    #[test]
    fn migrations_create_tables() {
        let db = test_db();
        // Verify tables exist by querying them
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert!(
            count >= 4,
            "Expected at least 4 default settings, got {count}"
        );
    }

    #[test]
    fn project_crud() {
        let db = test_db();
        let project = Project {
            id: "p1".into(),
            path: "/tmp/test".into(),
            display_name: "Test".into(),
            color: "#E57373".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
            last_used_at: "2025-01-01T00:00:00Z".into(),
        };
        db.insert_project(&project).unwrap();

        let all = db.get_all_projects().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].display_name, "Test");

        let found = db.get_project("p1").unwrap();
        assert!(found.is_some());

        db.update_project_last_used("p1", "2025-06-01T00:00:00Z")
            .unwrap();

        db.delete_project("p1").unwrap();
        assert!(db.get_project("p1").unwrap().is_none());
    }

    #[test]
    fn instance_crud() {
        let db = test_db();
        let inst = Instance {
            id: "i1".into(),
            project_id: None,
            title: Some("Test Instance".into()),
            status: InstanceStatus::Starting,
            instance_number: Some(1),
            tokens_used: 0,
            cost_estimate: 0.0,
            started_at: "2025-01-01T00:00:00Z".into(),
            ended_at: None,
            instance_type: InstanceType::Terminal,
            color: None,
        };
        db.insert_instance(&inst).unwrap();

        let all = db.get_all_instances().unwrap();
        assert_eq!(all.len(), 1);

        let next = db.get_next_instance_number().unwrap();
        assert_eq!(next, 2);

        db.update_instance_status("i1", InstanceStatus::Working)
            .unwrap();
        let all = db.get_all_instances().unwrap();
        assert_eq!(all[0].status, InstanceStatus::Working);

        db.end_instance("i1", "2025-01-01T01:00:00Z").unwrap();
        let all = db.get_all_instances().unwrap();
        assert!(
            all.is_empty(),
            "Ended instance should not appear in active list"
        );
    }

    #[test]
    fn terminal_history_roundtrip() {
        let db = test_db();
        let inst = Instance {
            id: "i2".into(),
            project_id: None,
            title: None,
            status: InstanceStatus::Starting,
            instance_number: Some(1),
            tokens_used: 0,
            cost_estimate: 0.0,
            started_at: "2025-01-01T00:00:00Z".into(),
            ended_at: None,
            instance_type: InstanceType::Terminal,
            color: None,
        };
        db.insert_instance(&inst).unwrap();
        db.save_terminal_history("i2", r#"["hello","world"]"#)
            .unwrap();
        let hist = db.get_terminal_history("i2").unwrap();
        assert_eq!(hist.as_deref(), Some(r#"["hello","world"]"#));
    }

    #[test]
    fn settings_crud() {
        let db = test_db();
        assert_eq!(db.get_setting("theme").unwrap().as_deref(), Some("dark"));

        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.get_setting("theme").unwrap().as_deref(), Some("light"));

        let all = db.get_all_settings().unwrap();
        assert!(all.len() >= 4);
    }

    #[test]
    fn question_crud() {
        let db = test_db();
        // Need an instance first
        let inst = Instance {
            id: "i3".into(),
            project_id: None,
            title: Some("Claude".into()),
            status: InstanceStatus::Waiting,
            instance_number: Some(1),
            tokens_used: 0,
            cost_estimate: 0.0,
            started_at: "2025-01-01T00:00:00Z".into(),
            ended_at: None,
            instance_type: InstanceType::Project,
            color: None,
        };
        db.insert_instance(&inst).unwrap();

        let q = Question {
            id: "q1".into(),
            instance_id: "i3".into(),
            question_text: "What should I do?".into(),
            context: Some("Some context".into()),
            asked_at: "2025-01-01T00:00:00Z".into(),
            answered_at: None,
            answer: None,
            snoozed_until: None,
        };
        db.insert_question(&q).unwrap();

        let pending = db.get_pending_questions().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].question.question_text, "What should I do?");
        assert_eq!(pending[0].instance_title.as_deref(), Some("Claude"));

        db.answer_question("q1", "Do this", "2025-01-01T00:01:00Z")
            .unwrap();
        let pending = db.get_pending_questions().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn cascade_delete() {
        let db = test_db();
        let project = Project {
            id: "p2".into(),
            path: "/tmp/cascade".into(),
            display_name: "Cascade".into(),
            color: "#64B5F6".into(),
            created_at: "2025-01-01T00:00:00Z".into(),
            last_used_at: "2025-01-01T00:00:00Z".into(),
        };
        db.insert_project(&project).unwrap();

        let inst = Instance {
            id: "i4".into(),
            project_id: Some("p2".into()),
            title: Some("Will be cascaded".into()),
            status: InstanceStatus::Starting,
            instance_number: Some(1),
            tokens_used: 0,
            cost_estimate: 0.0,
            started_at: "2025-01-01T00:00:00Z".into(),
            ended_at: None,
            instance_type: InstanceType::Project,
            color: None,
        };
        db.insert_instance(&inst).unwrap();
        assert_eq!(db.get_all_instances().unwrap().len(), 1);

        db.delete_project("p2").unwrap();
        assert!(
            db.get_all_instances().unwrap().is_empty(),
            "Instance should be cascade-deleted with project"
        );
    }
}
