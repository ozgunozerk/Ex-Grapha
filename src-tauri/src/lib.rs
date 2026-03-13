use std::{path::PathBuf, sync::Mutex};

use ex_grapha_core::{
    model::{EdgeAnnotation, Node},
    node::NodeParams,
    project::{self, InitOptions, KnowledgeBase, LoadWarning},
};
use tauri::State;

/// Shared application state: the currently open knowledge base (if any).
struct AppState {
    kb: Mutex<Option<KnowledgeBase>>,
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
    annotation: Option<String>,
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
                    annotation: d.annotation.as_ref().map(|a| a.label.clone()),
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
    state: State<'_, AppState>,
) -> Result<InitResult, String> {
    let p = PathBuf::from(&path);
    let kb = project::init_project(&p, &options).map_err(|e| e.to_string())?;
    let root = kb.root.clone();
    *state.kb.lock().unwrap() = Some(kb);
    Ok(InitResult { root })
}

#[tauri::command]
fn open_project(path: String, state: State<'_, AppState>) -> Result<OpenResult, String> {
    let p = PathBuf::from(&path);
    let (kb, warnings) = project::open_project(&p).map_err(|e| e.to_string())?;
    let result = OpenResult {
        root: kb.root.clone(),
        warnings: warnings.into_iter().map(Into::into).collect(),
    };
    *state.kb.lock().unwrap() = Some(kb);
    Ok(result)
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
    annotation: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_kb(&state, |kb| {
        let ann = annotation.map(|label| EdgeAnnotation { label });
        kb.create_edge(&dependent_id, &dependency_id, ann)
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

#[tauri::command]
fn update_edge_annotation(
    dependent_id: String,
    dependency_id: String,
    annotation: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_kb(&state, |kb| {
        let ann = annotation.map(|label| EdgeAnnotation { label });
        kb.update_edge_annotation(&dependent_id, &dependency_id, ann)
            .map_err(|e| e.to_string())
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
            kb: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            init_project,
            open_project,
            create_node,
            get_node,
            update_node,
            delete_node,
            create_edge,
            validate_edge_deletion,
            delete_edge,
            remove_dependency_and_convert_to_axiom,
            update_edge_annotation,
            validate_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
