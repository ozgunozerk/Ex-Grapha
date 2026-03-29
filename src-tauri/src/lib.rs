use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use ex_grapha_core::{
    model::Node,
    node::NodeParams,
    project::{self, InitOptions, KnowledgeBase, LoadWarning},
};
use tauri::{AppHandle, State};

mod watcher;
use watcher::WatcherHandle;

/// Shared application state: the currently open knowledge base (if any).
struct AppState {
    /// The in-memory knowledge base, shared between Tauri commands and the
    /// file watcher callback via `Arc`.
    kb: Arc<Mutex<Option<KnowledgeBase>>>,
    /// Handle to the running file watcher (if a project is open).
    watcher: Mutex<Option<WatcherHandle>>,
}

/// Helper: lock the state and get a mutable ref to the KB, or return an error
/// if no project is open.
fn with_kb<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&mut KnowledgeBase) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state.kb.lock().unwrap();
    let kb = guard.as_mut().ok_or("no project is open")?;
    f(kb)
}

/// Start the file watcher for the current project and store the handle.
fn start_project_watcher(app_handle: &AppHandle, state: &State<'_, AppState>, project_root: &Path) {
    let handle = watcher::start_watcher(
        app_handle.clone(),
        Arc::clone(&state.kb),
        project_root.to_path_buf(),
    );
    match handle {
        Ok(h) => {
            *state.watcher.lock().unwrap() = Some(h);
        }
        Err(e) => {
            eprintln!("warning: failed to start file watcher: {e}");
        }
    }
}

// ── DTOs ──────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct InitResult {
    root: PathBuf,
}

#[derive(serde::Serialize)]
struct OpenResult {
    root: PathBuf,
    warnings: Vec<WarningDto>,
}

#[derive(serde::Serialize)]
struct WarningDto {
    path: String,
    message: String,
}

impl From<LoadWarning> for WarningDto {
    fn from(w: LoadWarning) -> Self {
        Self {
            path: w.path.display().to_string(),
            message: w.message,
        }
    }
}

#[derive(serde::Serialize)]
struct NodeDto {
    id: String,
    title: String,
    node_type: String,
    tags: Vec<String>,
    status: String,
    status_updated_at: String,
    status_updated_by: String,
    created_at: String,
    created_by: String,
    dependencies: Vec<DependencyDto>,
    relation: Option<String>,
    content: String,
}

#[derive(serde::Serialize)]
struct DependencyDto {
    node_id: String,
}

#[derive(serde::Serialize)]
struct EdgeDeletionCheckDto {
    is_last_dependency: bool,
    node_title: String,
}

impl From<&Node> for NodeDto {
    fn from(node: &Node) -> Self {
        let fm = &node.frontmatter;
        Self {
            id: fm.id.clone(),
            title: fm.title.clone(),
            node_type: serde_json::to_value(&fm.node_type)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            tags: fm.tags.clone(),
            status: serde_json::to_value(&fm.status)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            status_updated_at: fm.status_updated_at.clone(),
            status_updated_by: fm.status_updated_by.clone(),
            created_at: fm.created_at.clone(),
            created_by: fm.created_by.clone(),
            dependencies: fm
                .dependencies
                .iter()
                .map(|d| DependencyDto {
                    node_id: d.node_id.clone(),
                })
                .collect(),
            relation: fm.relation.clone(),
            content: node.content.clone(),
        }
    }
}

// ── Project commands ──────────────────────────────────────

#[tauri::command]
fn init_project(
    path: String,
    options: InitOptions,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<InitResult, String> {
    let p = PathBuf::from(&path);
    let kb = project::init_project(&p, &options).map_err(|e| e.to_string())?;
    let root = kb.root.clone();
    *state.kb.lock().unwrap() = Some(kb);
    start_project_watcher(&app_handle, &state, &root);
    Ok(InitResult { root })
}

#[tauri::command]
fn open_project(
    path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<OpenResult, String> {
    let p = PathBuf::from(&path);
    let (kb, warnings) = project::open_project(&p).map_err(|e| e.to_string())?;
    let result = OpenResult {
        root: kb.root.clone(),
        warnings: warnings.into_iter().map(Into::into).collect(),
    };
    let root = kb.root.clone();
    *state.kb.lock().unwrap() = Some(kb);
    start_project_watcher(&app_handle, &state, &root);
    Ok(result)
}

#[tauri::command]
fn close_project(state: State<'_, AppState>) -> Result<(), String> {
    // Stop the file watcher first.
    *state.watcher.lock().unwrap() = None;
    // Then clear the KB.
    *state.kb.lock().unwrap() = None;
    Ok(())
}

// ── Node CRUD commands ────────────────────────────────────

#[tauri::command]
fn create_node(params: NodeParams, state: State<'_, AppState>) -> Result<NodeDto, String> {
    with_kb(&state, |kb| {
        let node = kb.create_node(params).map_err(|e| e.to_string())?;
        Ok(NodeDto::from(&node))
    })
}

#[tauri::command]
fn get_node(id: String, state: State<'_, AppState>) -> Result<NodeDto, String> {
    with_kb(&state, |kb| {
        let node = kb.get_node(&id).map_err(|e| e.to_string())?;
        Ok(NodeDto::from(node))
    })
}

#[tauri::command]
fn update_node(
    id: String,
    params: NodeParams,
    state: State<'_, AppState>,
) -> Result<NodeDto, String> {
    with_kb(&state, |kb| {
        let node = kb.update_node(&id, params).map_err(|e| e.to_string())?;
        Ok(NodeDto::from(&node))
    })
}

#[tauri::command]
fn delete_node(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_kb(&state, |kb| kb.delete_node(&id).map_err(|e| e.to_string()))
}

// ── Edge CRUD commands ────────────────────────────────────

#[tauri::command]
fn create_edge(
    dependent_id: String,
    dependency_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_kb(&state, |kb| {
        kb.create_edge(&dependent_id, &dependency_id)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn validate_edge_deletion(
    dependent_id: String,
    dependency_id: String,
    state: State<'_, AppState>,
) -> Result<EdgeDeletionCheckDto, String> {
    with_kb(&state, |kb| {
        let check = kb
            .validate_edge_deletion(&dependent_id, &dependency_id)
            .map_err(|e| e.to_string())?;
        Ok(EdgeDeletionCheckDto {
            is_last_dependency: check.is_last_dependency,
            node_title: check.node_title,
        })
    })
}

#[tauri::command]
fn delete_edge(
    dependent_id: String,
    dependency_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_kb(&state, |kb| {
        kb.delete_edge(&dependent_id, &dependency_id)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn remove_dependency_and_convert_to_axiom(
    dependent_id: String,
    dependency_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_kb(&state, |kb| {
        kb.remove_dependency_and_convert_to_axiom(&dependent_id, &dependency_id)
            .map_err(|e| e.to_string())
    })
}

// ── Staleness commands ────────────────────────────────────

#[tauri::command]
fn mark_node_reviewed(id: String, state: State<'_, AppState>) -> Result<NodeDto, String> {
    with_kb(&state, |kb| {
        let node = kb.mark_node_reviewed(&id).map_err(|e| e.to_string())?;
        Ok(NodeDto::from(&node))
    })
}

// ── Validation commands ───────────────────────────────────

#[tauri::command]
fn validate_project(
    state: State<'_, AppState>,
) -> Result<ex_grapha_core::validation::ValidationReport, String> {
    with_kb(&state, |kb| Ok(kb.validate()))
}

// ── App entry ─────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            kb: Arc::new(Mutex::new(None)),
            watcher: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            init_project,
            open_project,
            close_project,
            create_node,
            get_node,
            update_node,
            delete_node,
            create_edge,
            validate_edge_deletion,
            delete_edge,
            remove_dependency_and_convert_to_axiom,
            mark_node_reviewed,
            validate_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
