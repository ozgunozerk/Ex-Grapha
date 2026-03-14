//! Core methods for handling external file changes.
//!
//! These methods update the in-memory `KnowledgeBase` when a file is created,
//! modified, or deleted outside of the application (e.g., by an external
//! editor, `git pull`, or manual file editing).
//!
//! This module contains **no OS watcher logic** — it only provides the
//! data-layer operations that respond to detected changes. The actual
//! file-system watcher lives in the Tauri layer (`src-tauri/src/watcher.rs`).

use std::{fs, path::Path};

use crate::{
    config::ProjectConfig, error::Error, model::NodeId, node_parser, project::KnowledgeBase,
};

// ── Result types ──────────────────────────────────────────

/// What kind of external change occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeChangeKind {
    /// A new node was created externally.
    Created,
    /// An existing node was modified externally (content differs).
    Modified,
    /// A node was deleted externally.
    Deleted,
    /// The file was written but content matches in-memory state (self-write).
    Unchanged,
}

/// Result of processing an external file event.
#[derive(Debug, Clone)]
pub struct NodeChangeResult {
    /// The ID of the affected node.
    pub node_id: NodeId,
    /// What kind of change was detected.
    pub kind: NodeChangeKind,
    /// Node IDs that were marked stale (populated only for `Modified`).
    pub stale_affected: Vec<NodeId>,
    /// Node IDs that now have broken dependencies (populated only for
    /// `Deleted`).
    pub orphaned_dependents: Vec<NodeId>,
}

// ── KnowledgeBase methods ─────────────────────────────────

impl KnowledgeBase {
    /// Ingest an externally created or modified node file.
    ///
    /// Parses the file, compares against in-memory state, and updates the
    /// graph accordingly:
    ///
    /// - **New node** → inserted, indexes rebuilt, returns `Created`.
    /// - **Modified node** → replaced, indexes rebuilt, staleness propagated,
    ///   returns `Modified` with the list of nodes that became stale.
    /// - **Unchanged node** → content matches in-memory (self-write
    ///   suppression), returns `Unchanged`.
    pub fn ingest_external_node(&mut self, file_path: &Path) -> Result<NodeChangeResult, Error> {
        let parsed = node_parser::read_node_file(file_path)?;
        let node_id = parsed.frontmatter.id.clone();

        // Check if we already have this node in memory.
        if let Some(existing) = self.nodes.get(&node_id) {
            if *existing == parsed {
                // Content matches — this is a self-write echo; ignore.
                return Ok(NodeChangeResult {
                    node_id,
                    kind: NodeChangeKind::Unchanged,
                    stale_affected: Vec::new(),
                    orphaned_dependents: Vec::new(),
                });
            }

            // Content differs — external modification.
            self.nodes.insert(node_id.clone(), parsed);
            self.rebuild_indexes();
            let stale_affected = self.propagate_staleness(&node_id)?;

            return Ok(NodeChangeResult {
                node_id,
                kind: NodeChangeKind::Modified,
                stale_affected,
                orphaned_dependents: Vec::new(),
            });
        }

        // Node doesn't exist in memory — external creation.
        self.nodes.insert(node_id.clone(), parsed);
        self.rebuild_indexes();

        Ok(NodeChangeResult {
            node_id,
            kind: NodeChangeKind::Created,
            stale_affected: Vec::new(),
            orphaned_dependents: Vec::new(),
        })
    }

    /// Remove an externally-deleted node from the in-memory graph.
    ///
    /// Unlike [`delete_node()`](crate::node), this does **not** block on
    /// existing dependents — the file is already gone. Instead, it reports
    /// which nodes now have broken (dangling) dependencies.
    pub fn remove_external_node(&mut self, node_id: &str) -> Result<NodeChangeResult, Error> {
        if self.nodes.remove(node_id).is_none() {
            return Err(Error::NodeNotFound(node_id.to_string()));
        }

        // Collect nodes that depended on the deleted node (orphaned).
        let orphaned_dependents: Vec<NodeId> = self
            .dependents
            .get(node_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        self.rebuild_indexes();

        Ok(NodeChangeResult {
            node_id: node_id.to_string(),
            kind: NodeChangeKind::Deleted,
            stale_affected: Vec::new(),
            orphaned_dependents,
        })
    }

    /// Reload the project configuration from disk.
    ///
    /// Re-reads `.knowledgebase/config.yaml` and replaces the in-memory
    /// `config` field.
    pub fn reload_config(&mut self) -> Result<(), Error> {
        let config_path = self.root.join(".knowledgebase/config.yaml");
        let config_str = fs::read_to_string(&config_path)?;
        self.config = ProjectConfig::from_yaml(&config_str)?;
        Ok(())
    }
}
