use std::path::PathBuf;
use std::sync::Mutex;

use ex_grapha_core::project::{self, InitOptions, KnowledgeBase, LoadWarning};
use tauri::State;

/// Shared application state: the currently open knowledge base (if any).
struct AppState {
    kb: Mutex<Option<KnowledgeBase>>,
}

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

#[tauri::command]
fn init_project(
    path: String,
    include_git_hook: bool,
    include_github_workflow: bool,
    state: State<'_, AppState>,
) -> Result<InitResult, String> {
    let p = PathBuf::from(&path);
    let options = InitOptions {
        include_git_hook,
        include_github_workflow,
    };
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            kb: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![init_project, open_project])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
