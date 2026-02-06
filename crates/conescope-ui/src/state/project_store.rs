use conescope_core::project::{PROJECT_COLORS, Project};

use super::db_worker::DbHandle;

#[derive(Debug)]
pub struct ProjectStore {
    projects: Vec<Project>,
    db: DbHandle,
    loaded: bool,
}

impl ProjectStore {
    #[must_use]
    pub fn new(db: DbHandle) -> Self {
        Self {
            projects: Vec::new(),
            db,
            loaded: false,
        }
    }

    pub fn load(&mut self, projects: Vec<Project>) {
        self.projects = projects;
        self.loaded = true;
    }

    #[must_use]
    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    #[must_use]
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    #[must_use]
    pub fn next_color(&self) -> &'static str {
        let used: Vec<&str> = self.projects.iter().map(|p| p.color.as_str()).collect();
        PROJECT_COLORS
            .iter()
            .find(|c| !used.contains(c))
            .copied()
            .unwrap_or(PROJECT_COLORS[self.projects.len() % PROJECT_COLORS.len()])
    }

    pub fn add(&mut self, project: Project) {
        self.db.insert_project(project.clone());
        self.projects.push(project);
    }

    pub fn update_last_used(&mut self, id: &str, timestamp: &str) {
        if let Some(p) = self.projects.iter_mut().find(|p| p.id == id) {
            timestamp.clone_into(&mut p.last_used_at);
        }
        self.db
            .update_project_last_used(id.to_owned(), timestamp.to_owned());
    }

    pub fn delete(&mut self, id: &str) {
        self.projects.retain(|p| p.id != id);
        self.db.delete_project(id.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_project(id: &str, color: &str) -> Project {
        Project {
            id: id.into(),
            path: format!("/tmp/{id}"),
            display_name: id.into(),
            color: color.into(),
            created_at: "2025-01-01T00:00:00Z".into(),
            last_used_at: "2025-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn next_color_avoids_used() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = ProjectStore::new(db);
        store.load(vec![
            make_project("p1", PROJECT_COLORS[0]),
            make_project("p2", PROJECT_COLORS[1]),
        ]);

        let next = store.next_color();
        assert_eq!(next, PROJECT_COLORS[2]);
    }

    #[test]
    fn next_color_cycles_when_exhausted() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = ProjectStore::new(db);
        let all_used: Vec<Project> = PROJECT_COLORS
            .iter()
            .enumerate()
            .map(|(i, c)| make_project(&format!("p{i}"), c))
            .collect();
        store.load(all_used);

        // Should cycle back
        let next = store.next_color();
        assert!(PROJECT_COLORS.contains(&next));
    }

    #[test]
    fn get_finds_by_id() {
        let db = DbHandle::spawn(":memory:").unwrap();
        let mut store = ProjectStore::new(db);
        store.load(vec![make_project("abc", "#E57373")]);

        assert!(store.get("abc").is_some());
        assert!(store.get("missing").is_none());
    }
}
