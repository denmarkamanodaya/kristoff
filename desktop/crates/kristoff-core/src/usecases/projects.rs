//! Project use cases: create (cap 50), list (name order, tie-break id), rename and delete (every
//! chat of the project is ungrouped first, then the project goes). Ported from `create-project`,
//! `list-projects`, `rename-project` and `delete-project`.

use crate::domain::names::project_name;
use crate::error::AppResult;
use crate::limits::MAX_PROJECTS;
use crate::model::Project;
use crate::ports::{ChatStore, Clock, IdGenerator, ProjectRename, ProjectStore};
use crate::usecases::chats::each_chat;

/// What the project use cases need.
pub struct ProjectDeps<'a> {
    /// Projects.
    pub projects: &'a dyn ProjectStore,
    /// Chats (delete ungroups them).
    pub chats: &'a dyn ChatStore,
    /// Time.
    pub clock: &'a dyn Clock,
    /// Ids.
    pub ids: &'a dyn IdGenerator,
}

/// Creates a project.
///
/// # Errors
/// `VALIDATION_FAILED` (blank or long name), `PROJECT_LIMIT_REACHED`, store failures.
pub fn create_project(deps: &ProjectDeps<'_>, name: &str) -> AppResult<Project> {
    let name = project_name(name)?;
    let now = deps.clock.now_ms();
    let project = Project {
        id: deps.ids.new_id(),
        name,
        created_at_ms: now,
        updated_at_ms: now,
    };
    deps.projects.create_project(&project, MAX_PROJECTS)?;
    Ok(project)
}

/// Every project in name order (byte order, tie-break on id, so the order is total).
///
/// # Errors
/// Store failures.
pub fn list_projects(deps: &ProjectDeps<'_>) -> AppResult<Vec<Project>> {
    let mut all = deps.projects.list_projects()?;
    all.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    Ok(all)
}

/// Renames a project.
///
/// # Errors
/// `VALIDATION_FAILED`, `PROJECT_NOT_FOUND`, `VERSION_CONFLICT`.
pub fn rename_project(deps: &ProjectDeps<'_>, project_id: &str, name: &str) -> AppResult<Project> {
    let name = project_name(name)?;
    let found = deps.projects.get_project(project_id)?;
    let change = ProjectRename {
        name,
        updated_at_ms: deps.clock.now_ms(),
        version: found.version,
    };
    deps.projects.rename_project(project_id, &change)
}

/// Deletes a project after ungrouping each of its chats; a failure midway leaves the project, so
/// the delete can be repeated.
///
/// # Errors
/// `PROJECT_NOT_FOUND` and store failures.
pub fn delete_project(deps: &ProjectDeps<'_>, project_id: &str) -> AppResult<()> {
    deps.projects.get_project(project_id)?;
    let grouped = each_chat(deps.chats, |c| c.project_id.as_deref() == Some(project_id))?;
    for chat_id in grouped {
        deps.chats.clear_project(&chat_id, project_id)?;
    }
    deps.projects.delete_project(project_id)
}
