//! Edge CRUD operations and cycle detection on `KnowledgeBase`.

use std::collections::{HashSet, VecDeque};

use crate::{
    error::Error,
    model::{Dependency, NodeType},
    node_parser,
    project::KnowledgeBase,
};

/// Result of validating an edge deletion before executing it.
#[derive(Debug, Clone)]
pub struct EdgeDeletionCheck {
    /// True if this is the last dependency on the dependent node.
    pub is_last_dependency: bool,
    /// Title of the dependent node (for the frontend confirmation prompt).
    pub node_title: String,
}

impl KnowledgeBase {
    // ── Edge CRUD ─────────────────────────────────────────

    /// Check whether adding an edge `dependent → dependency` would create a
    /// cycle. O(1) check using transitive dependency sets: if `dependent` is
    /// already in `dependency`'s transitive deps, adding the edge would close
    /// a cycle.
    fn would_create_cycle(&self, dependency_id: &str, dependent_id: &str) -> bool {
        if dependency_id == dependent_id {
            return true;
        }

        // If dependency transitively depends on dependent, adding
        // dependent → dependency would create a cycle.
        self.dependencies
            .get(dependency_id)
            .is_some_and(|deps: &HashSet<String>| deps.contains(dependent_id))
    }

    /// Add a dependency edge: `dependent` depends on `dependency`.
    pub fn create_edge(
        &mut self,
        dependent_id: &str,
        dependency_id: &str,
    ) -> Result<(), Error> {
        // Both nodes must exist.
        if !self.nodes.contains_key(dependent_id) {
            return Err(Error::NodeNotFound(dependent_id.to_string()));
        }
        if !self.nodes.contains_key(dependency_id) {
            return Err(Error::NodeNotFound(dependency_id.to_string()));
        }

        // Check for duplicate.
        let node = &self.nodes[dependent_id];
        if node
            .frontmatter
            .dependencies
            .iter()
            .any(|d| d.node_id == dependency_id)
        {
            return Err(Error::EdgeAlreadyExists {
                from: dependent_id.to_string(),
                to: dependency_id.to_string(),
            });
        }

        // Cycle detection.
        if self.would_create_cycle(dependency_id, dependent_id) {
            return Err(Error::CycleDetected {
                path: vec![dependent_id.to_string(), dependency_id.to_string()],
            });
        }

        // Add the dependency.
        let node = self.nodes.get_mut(dependent_id).unwrap();
        node.frontmatter.dependencies.push(Dependency {
            node_id: dependency_id.to_string(),
        });

        let file_path = self.root.join(format!("nodes/{dependent_id}.md"));
        node_parser::write_node_file(&file_path, node)?;

        // --- Incremental index update (no full rebuild) ---
        //
        // We just added: dependent → dependency.
        //
        // 1. Record in the reverse dep map: dependency is now depended on by dependent.
        self.dependents
            .entry(dependency_id.to_string())
            .or_default()
            .insert(dependent_id.to_string());

        // 2. Collect the set of transitive deps that the *dependent* just gained
        //    through this new edge. That set is: { dependency } ∪
        //    dependencies[dependency] (i.e. the dependency itself, plus everything *it*
        //    transitively depends on).
        let mut gained = HashSet::new();
        gained.insert(dependency_id.to_string());
        if let Some(sub) = self.dependencies.get(dependency_id).cloned() {
            gained.extend(sub);
        }

        // 3. Propagate `gained` into the dependent's dependencies, and then into every
        //    node that transitively depend on the dependent). We use BFS and stop
        //    propagation along any branch where nothing new was actually added
        //    (fixed-point).
        let mut queue = VecDeque::new();
        queue.push_back(dependent_id.to_string());
        while let Some(current) = queue.pop_front() {
            let entry = self.dependencies.entry(current.clone()).or_default();
            let before_len = entry.len();
            entry.extend(gained.iter().cloned());
            // Only continue downstream if this node's set actually grew.
            if entry.len() > before_len {
                if let Some(downstream) = self.dependents.get(&current) {
                    for next in downstream {
                        queue.push_back(next.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// Check whether deleting an edge would leave the dependent node with
    /// zero dependencies (requiring conversion to axiom).
    pub fn validate_edge_deletion(
        &self,
        dependent_id: &str,
        dependency_id: &str,
    ) -> Result<EdgeDeletionCheck, Error> {
        let node = self
            .nodes
            .get(dependent_id)
            .ok_or_else(|| Error::NodeNotFound(dependent_id.to_string()))?;

        if !node
            .frontmatter
            .dependencies
            .iter()
            .any(|d| d.node_id == dependency_id)
        {
            return Err(Error::EdgeNotFound {
                from: dependent_id.to_string(),
                to: dependency_id.to_string(),
            });
        }

        Ok(EdgeDeletionCheck {
            is_last_dependency: node.frontmatter.dependencies.len() == 1,
            node_title: node.frontmatter.title.clone(),
        })
    }

    /// Remove a dependency edge without converting the node type.
    pub fn delete_edge(&mut self, dependent_id: &str, dependency_id: &str) -> Result<(), Error> {
        let node = self
            .nodes
            .get_mut(dependent_id)
            .ok_or_else(|| Error::NodeNotFound(dependent_id.to_string()))?;

        let before = node.frontmatter.dependencies.len();
        node.frontmatter
            .dependencies
            .retain(|d| d.node_id != dependency_id);

        if node.frontmatter.dependencies.len() == before {
            return Err(Error::EdgeNotFound {
                from: dependent_id.to_string(),
                to: dependency_id.to_string(),
            });
        }

        let file_path = self.root.join(format!("nodes/{dependent_id}.md"));
        node_parser::write_node_file(&file_path, node)?;

        self.rebuild_indexes();
        Ok(())
    }

    /// Remove the last dependency and convert the node to an axiom.
    pub fn remove_dependency_and_convert_to_axiom(
        &mut self,
        dependent_id: &str,
        dependency_id: &str,
    ) -> Result<(), Error> {
        let node = self
            .nodes
            .get_mut(dependent_id)
            .ok_or_else(|| Error::NodeNotFound(dependent_id.to_string()))?;

        let before = node.frontmatter.dependencies.len();
        node.frontmatter
            .dependencies
            .retain(|d| d.node_id != dependency_id);

        if node.frontmatter.dependencies.len() == before {
            return Err(Error::EdgeNotFound {
                from: dependent_id.to_string(),
                to: dependency_id.to_string(),
            });
        }

        // Convert to axiom: clear type, relation, and stale_sources.
        node.frontmatter.node_type = NodeType::Axiom;
        node.frontmatter.relation = None;
        node.frontmatter.stale_sources.clear();

        let file_path = self.root.join(format!("nodes/{dependent_id}.md"));
        node_parser::write_node_file(&file_path, node)?;

        self.rebuild_indexes();
        Ok(())
    }

}
