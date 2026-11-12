//! Project commands: `list_projects`, `create_project`, `rename_project`, `delete_project`.

use super::{CommandResult, Shared, blocking, unlocked};
use crate::composition::Composition;
use kristoff_contracts::envelopes::{DeletedResponse, ProjectResponse, ProjectsResponse};
use kristoff_contracts::requests;
use kristoff_contracts::shapes::ProjectDto;
use kristoff_core::usecases::projects;
use tauri::State;

/// Every project in name order.
///
/// # Errors
/// `UNAUTHENTICATED`.
pub fn list(app: &Composition) -> CommandResult<ProjectsResponse> {
    unlocked(app)?;
    let all = projects::list_projects(&app.projects())?;
    Ok(ProjectsResponse {
        items: all.iter().map(ProjectDto::from).collect(),
    })
}

/// Creates a project (cap 50).
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `PROJECT_LIMIT_REACHED`.
pub fn create(app: &Composition, name: &str) -> CommandResult<ProjectResponse> {
    unlocked(app)?;
    let project = projects::create_project(&app.projects(), requests::project_name(name)?)?;
    Ok(ProjectResponse {
        project: ProjectDto::from(&project),
    })
}

/// Renames a project.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `PROJECT_NOT_FOUND`.
pub fn rename(app: &Composition, project_id: &str, name: &str) -> CommandResult<ProjectResponse> {
    unlocked(app)?;
    let (id, name) = (requests::id(project_id)?, requests::project_name(name)?);
    let project = projects::rename_project(&app.projects(), id, name)?;
    Ok(ProjectResponse {
        project: ProjectDto::from(&project),
    })
}

/// Deletes a project after ungrouping its chats.
///
/// # Errors
/// `UNAUTHENTICATED`, `VALIDATION_FAILED`, `PROJECT_NOT_FOUND`.
pub fn delete(app: &Composition, project_id: &str) -> CommandResult<DeletedResponse> {
    unlocked(app)?;
    projects::delete_project(&app.projects(), requests::id(project_id)?)?;
    Ok(DeletedResponse { deleted: true })
}

/// `list_projects`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn list_projects(app: State<'_, Shared>) -> CommandResult<ProjectsResponse> {
    blocking(&app, list).await
}

/// `create_project {name}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn create_project(
    app: State<'_, Shared>,
    name: String,
) -> CommandResult<ProjectResponse> {
    blocking(&app, move |c| create(c, &name)).await
}

/// `rename_project {projectId, name}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn rename_project(
    app: State<'_, Shared>,
    project_id: String,
    name: String,
) -> CommandResult<ProjectResponse> {
    blocking(&app, move |c| rename(c, &project_id, &name)).await
}

/// `delete_project {projectId}`.
///
/// # Errors
/// Those of the plain function it runs, as an `ErrorDto`.
#[tauri::command]
pub async fn delete_project(
    app: State<'_, Shared>,
    project_id: String,
) -> CommandResult<DeletedResponse> {
    blocking(&app, move |c| delete(c, &project_id)).await
}
