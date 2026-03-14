//! Staleness propagation engine.
//!
//! When a node is edited its content has changed, so every downstream
//! dependent may be out-of-date. This module provides two operations:
//!
//! - **`propagate_staleness`**: marks all transitive dependents of an edited
//!   node as `Stale`, recording the edited node as a `StaleSource`.
//! - **`mark_node_reviewed`**: clears staleness on a single node (status →
//!   `Current`, `stale_sources` → empty) **without** cascading further.
//!
//! ## Rules
//!
//! - **Axioms are immune**: they have no upstream dependencies, so they are
//!   never marked stale by propagation.  Editing an axiom *does* propagate to
//!   its dependents, but the axiom itself stays current.
//! - **Idempotent**: propagating from the same source twice does not duplicate
//!   `stale_sources` entries.
//! - **Review stops the cascade**: `mark_node_reviewed` does *not* propagate
//!   any change — the node simply becomes current.
//! - **Edit re-propagates**: if a stale node is edited, it becomes current and
//!   a *new* propagation wave rolls downstream.

use std::collections::{HashSet, VecDeque};

use crate::{
    error::Error,
    model::{Node, NodeId, NodeType, StaleSource, Status},
    project::KnowledgeBase,
    util,
};

impl KnowledgeBase {
    /// Mark all downstream dependents of `edited_node_id` as stale.
    ///
    /// Traverses the `dependents` map level by level (BFS). For each
    /// affected node: sets status to `Stale`, appends a `StaleSource`
    /// entry, and writes the update to disk.
    ///
    /// **Axioms are skipped** — they never become stale from propagation.
    /// Nodes that already carry this same `StaleSource` are skipped
    /// (idempotent).
    ///
    /// Returns the list of node IDs that were actually marked stale.
    pub fn propagate_staleness(&mut self, edited_node_id: &str) -> Result<Vec<NodeId>, Error> {
        let now = util::now_iso8601();
        let username = util::os_username();

        // Seed the BFS with direct dependents of the edited node.
        let mut queue: VecDeque<NodeId> = self
            .dependents
            .get(edited_node_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();

        let mut visited = HashSet::new();
        let mut affected = Vec::new();

        while let Some(node_id) = queue.pop_front() {
            if !visited.insert(node_id.clone()) {
                continue;
            }

            let node = match self.nodes.get_mut(&node_id) {
                Some(n) => n,
                None => continue, // dangling reference — skip
            };

            // Axioms never become stale from propagation.
            if node.frontmatter.node_type == NodeType::Axiom {
                continue;
            }

            // Idempotent: skip if already stale from this exact source.
            if node
                .frontmatter
                .stale_sources
                .iter()
                .any(|s| s.node_id == edited_node_id)
            {
                continue;
            }

            // Mark stale.
            node.frontmatter.status = Status::Stale;
            node.frontmatter.stale_sources.push(StaleSource {
                node_id: edited_node_id.to_string(),
                changed_at: now.clone(),
            });
            node.frontmatter.status_updated_at = now.clone();
            node.frontmatter.status_updated_by = username.clone();

            self.write_node_to_disk(&node_id)?;

            affected.push(node_id.clone());

            // Enqueue this node's dependents to cascade further.
            if let Some(downstream) = self.dependents.get(&node_id) {
                for next in downstream {
                    queue.push_back(next.clone());
                }
            }
        }

        Ok(affected)
    }

    /// Mark a node as reviewed: status → `Current`, `stale_sources` → cleared.
    ///
    /// Does **not** propagate further — the cascade stops at this node.
    /// Calling this on an already-current node is a no-op.
    pub fn mark_node_reviewed(&mut self, id: &str) -> Result<Node, Error> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or_else(|| Error::NodeNotFound(id.to_string()))?;

        node.frontmatter.status = Status::Current;
        node.frontmatter.stale_sources.clear();
        node.frontmatter.status_updated_at = util::now_iso8601();
        node.frontmatter.status_updated_by = util::os_username();

        self.write_node_to_disk(id)?;

        Ok(self.nodes[id].clone())
    }
}
