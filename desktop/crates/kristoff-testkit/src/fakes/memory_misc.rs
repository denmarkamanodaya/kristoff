//! `ProjectStore` and `UsageMeter` on `MemoryStore`.

use super::memory_store::{MemoryStore, conflict, duplicate};
use kristoff_core::error::{AppError, AppResult, ErrorCode};
use kristoff_core::model::{Project, Usage, UsageReport, Versioned};
use kristoff_core::ports::{ProjectRename, ProjectStore, UsageMeter};

fn not_found() -> AppError {
    AppError::new(ErrorCode::ProjectNotFound, "no such project")
}

fn add(a: &mut Usage, b: &Usage) {
    a.input += b.input;
    a.output += b.output;
    a.cache_read += b.cache_read;
    a.cache_write += b.cache_write;
}

impl ProjectStore for MemoryStore {
    fn create_project(&self, project: &Project, cap: usize) -> AppResult<()> {
        let mut state = self.state();
        if state.projects.len() >= cap {
            return Err(AppError::new(
                ErrorCode::ProjectLimitReached,
                "too many projects",
            ));
        }
        if state.projects.contains_key(&project.id) {
            return Err(duplicate("project"));
        }
        let stored = Versioned {
            value: project.clone(),
            version: 1,
        };
        state.projects.insert(project.id.clone(), stored);
        Ok(())
    }

    fn get_project(&self, project_id: &str) -> AppResult<Versioned<Project>> {
        self.state()
            .projects
            .get(project_id)
            .cloned()
            .ok_or_else(not_found)
    }

    fn list_projects(&self) -> AppResult<Vec<Project>> {
        Ok(self
            .state()
            .projects
            .values()
            .map(|p| p.value.clone())
            .collect())
    }

    fn rename_project(&self, project_id: &str, change: &ProjectRename) -> AppResult<Project> {
        let mut state = self.state();
        let stored = state.projects.get_mut(project_id).ok_or_else(not_found)?;
        if stored.version != change.version {
            return Err(conflict());
        }
        stored.value.name.clone_from(&change.name);
        stored.value.updated_at_ms = change.updated_at_ms;
        stored.version += 1;
        Ok(stored.value.clone())
    }

    fn delete_project(&self, project_id: &str) -> AppResult<()> {
        self.state()
            .projects
            .remove(project_id)
            .map(|_| ())
            .ok_or_else(not_found)
    }
}

impl UsageMeter for MemoryStore {
    fn admit_question(&self, day: &str, cap: u32) -> AppResult<()> {
        let mut state = self.state();
        let entry = state.days.entry(day.to_owned()).or_default();
        if entry.0 >= cap {
            return Err(AppError::new(
                ErrorCode::DailyCapReached,
                "daily question cap reached",
            ));
        }
        entry.0 += 1;
        Ok(())
    }

    fn add_tokens(&self, day: &str, usage: &Usage) -> AppResult<()> {
        let mut state = self.state();
        add(&mut state.days.entry(day.to_owned()).or_default().1, usage);
        add(&mut state.total, usage);
        Ok(())
    }

    fn read_usage(&self, day: &str) -> AppResult<UsageReport> {
        let state = self.state();
        let today = state.days.get(day).map(|d| d.1).unwrap_or_default();
        Ok(UsageReport {
            today,
            total: state.total,
        })
    }
}
