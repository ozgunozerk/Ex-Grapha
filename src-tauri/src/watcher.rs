//! OS-level file watcher for detecting external changes to the knowledge base.
//!
//! Uses the `notify` crate (via `notify-debouncer-mini`) to watch `nodes/`
//! and `.knowledgebase/config.yaml`. When changes are detected, updates the
//! in-memory `KnowledgeBase` via core methods and emits Tauri events so the
//! frontend can react.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use ex_grapha_core::{project::KnowledgeBase, watcher::NodeChangeKind};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Emitter};

// ── Event DTOs ────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCreatedEvent {
    pub node_id: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeModifiedEvent {
    pub node_id: String,
    pub stale_affected: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDeletedEvent {
    pub node_id: String,
    pub orphaned_dependents: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherErrorEvent {
    pub message: String,
}

// ── Event name constants ──────────────────────────────────

const EVENT_NODE_CREATED: &str = "node:created-external";
const EVENT_NODE_MODIFIED: &str = "node:modified-external";
const EVENT_NODE_DELETED: &str = "node:deleted-external";
const EVENT_CONFIG_RELOADED: &str = "config:reloaded";
const EVENT_WATCHER_ERROR: &str = "watcher:error";

// ── WatcherHandle ─────────────────────────────────────────

/// Handle to the running file watcher. Dropping this stops the watcher.
pub struct WatcherHandle {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
}

// ── Helper ────────────────────────────────────────────────

/// Extract a node ID from a file path like `nodes/n-4a7b2c.md` → `n-4a7b2c`.
fn node_id_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Check if a path is a `.md` file inside the `nodes/` directory.
fn is_node_file(path: &Path, project_root: &Path) -> bool {
    let nodes_dir = project_root.join("nodes");
    path.starts_with(&nodes_dir)
        && path.extension().is_some_and(|ext| ext == "md")
        && path.parent() == Some(nodes_dir.as_path())
}

/// Check if a path is the config file.
fn is_config_file(path: &Path, project_root: &Path) -> bool {
    path == project_root.join(".knowledgebase/config.yaml")
}

/// Start a debounced file watcher for the given project.
///
/// Watches `nodes/` for node file changes and `.knowledgebase/config.yaml`
/// for config changes. Debounces events by 500ms to handle rapid
/// save-then-format sequences.
///
/// Returns a `WatcherHandle`; dropping it stops the watcher.
pub fn start_watcher(
    app_handle: AppHandle,
    kb_state: Arc<Mutex<Option<KnowledgeBase>>>,
    project_root: PathBuf,
) -> Result<WatcherHandle, String> {
    let root = project_root.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        move |result: DebounceEventResult| {
            handle_debounced_events(&app_handle, &kb_state, &root, result);
        },
    )
    .map_err(|e| format!("failed to create file watcher: {e}"))?;

    // Watch the nodes directory.
    let nodes_dir = project_root.join("nodes");
    debouncer
        .watcher()
        .watch(&nodes_dir, notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("failed to watch nodes directory: {e}"))?;

    // Watch the .knowledgebase directory (for config.yaml changes).
    let kb_dir = project_root.join(".knowledgebase");
    debouncer
        .watcher()
        .watch(&kb_dir, notify::RecursiveMode::NonRecursive)
        .map_err(|e| format!("failed to watch .knowledgebase directory: {e}"))?;

    Ok(WatcherHandle {
        _debouncer: debouncer,
    })
}

/// Process a batch of debounced events.
fn handle_debounced_events(
    app: &AppHandle,
    kb_state: &Arc<Mutex<Option<KnowledgeBase>>>,
    project_root: &Path,
    result: DebounceEventResult,
) {
    let events = match result {
        Ok(events) => events,
        Err(e) => {
            let _ = app.emit(
                EVENT_WATCHER_ERROR,
                WatcherErrorEvent {
                    message: format!("file watcher error: {e:?}"),
                },
            );
            return;
        }
    };

    // Deduplicate paths (multiple events may fire for the same file).
    let mut seen = std::collections::HashSet::new();
    let unique_paths: Vec<PathBuf> = events
        .into_iter()
        .filter(|e| seen.insert(e.path.clone()))
        .map(|e| e.path)
        .collect();

    for path in unique_paths {
        if is_node_file(&path, project_root) {
            handle_node_event(app, kb_state, &path);
        } else if is_config_file(&path, project_root) {
            handle_config_event(app, kb_state);
        }
        // Other files (non-.md, non-config) are silently ignored.
    }
}

/// Handle a node file event (create, modify, or delete).
fn handle_node_event(app: &AppHandle, kb_state: &Arc<Mutex<Option<KnowledgeBase>>>, path: &Path) {
    let mut guard = kb_state.lock().unwrap();
    let kb = match guard.as_mut() {
        Some(kb) => kb,
        None => return, // No project open — ignore.
    };

    if path.exists() {
        // File exists on disk → create or modify.
        match kb.ingest_external_node(path) {
            Ok(result) => match result.kind {
                NodeChangeKind::Created => {
                    let _ = app.emit(
                        EVENT_NODE_CREATED,
                        NodeCreatedEvent {
                            node_id: result.node_id,
                        },
                    );
                }
                NodeChangeKind::Modified => {
                    let _ = app.emit(
                        EVENT_NODE_MODIFIED,
                        NodeModifiedEvent {
                            node_id: result.node_id,
                            stale_affected: result.stale_affected,
                        },
                    );
                }
                NodeChangeKind::Unchanged => {
                    // Self-write suppression — no event needed.
                }
                NodeChangeKind::Deleted => {
                    unreachable!("should not happen in this branch, the file exists")
                }
            },
            Err(e) => {
                let _ = app.emit(
                    EVENT_WATCHER_ERROR,
                    WatcherErrorEvent {
                        message: format!("failed to parse node file {}: {e}", path.display()),
                    },
                );
            }
        }
    } else {
        // File doesn't exist on disk → deletion.
        if let Some(node_id) = node_id_from_path(path) {
            match kb.remove_external_node(&node_id) {
                Ok(result) => {
                    let _ = app.emit(
                        EVENT_NODE_DELETED,
                        NodeDeletedEvent {
                            node_id: result.node_id,
                            orphaned_dependents: result.orphaned_dependents,
                        },
                    );
                }
                Err(_) => {
                    // Node wasn't in memory — probably already handled or
                    // a file we never knew about. Silently ignore.
                }
            }
        }
    }
}

/// Handle a config file event.
fn handle_config_event(app: &AppHandle, kb_state: &Arc<Mutex<Option<KnowledgeBase>>>) {
    let mut guard = kb_state.lock().unwrap();
    let kb = match guard.as_mut() {
        Some(kb) => kb,
        None => return,
    };

    match kb.reload_config() {
        Ok(()) => {
            let _ = app.emit(EVENT_CONFIG_RELOADED, ());
        }
        Err(e) => {
            let _ = app.emit(
                EVENT_WATCHER_ERROR,
                WatcherErrorEvent {
                    message: format!("failed to reload config: {e}"),
                },
            );
        }
    }
}
