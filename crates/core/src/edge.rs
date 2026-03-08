//! Edge CRUD operations and cycle detection on `KnowledgeBase`.

use std::collections::{HashSet, VecDeque};

use crate::{
    error::Error,
    model::{Dependency, EdgeAnnotation, NodeType},
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
    /// cycle. Uses BFS from `dependent` through the `dependents` adjacency
    /// map: if `dependency` is reachable, the new edge would close a cycle.
    fn would_create_cycle(&self, dependency_id: &str, dependent_id: &str) -> bool {
        if dependency_id == dependent_id {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(dependent_id.to_string());
        visited.insert(dependent_id.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(downstream) = self.dependents.get(&current) {
                for next in downstream {
                    if next == dependency_id {
                        return true;
                    }
                    if visited.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }

        false
    }

    /// Add a dependency edge: `dependent` depends on `dependency`.
    pub fn create_edge(
        &mut self,
        dependent_id: &str,
        dependency_id: &str,
        annotation: Option<EdgeAnnotation>,
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
                path: vec![
                    dependent_id.to_string(),
                    dependency_id.to_string(),
                    dependent_id.to_string(),
                ],
            });
        }

        // Add the dependency.
        let node = self.nodes.get_mut(dependent_id).unwrap();
        node.frontmatter.dependencies.push(Dependency {
            node_id: dependency_id.to_string(),
            annotation,
        });

        let file_path = self.root.join(format!("nodes/{dependent_id}.md"));
        node_parser::write_node_file(&file_path, node)?;

        self.rebuild_adjacency();
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

        self.rebuild_adjacency();
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

        self.rebuild_adjacency();
        Ok(())
    }

    /// Change or remove the annotation on an existing edge.
    pub fn update_edge_annotation(
        &mut self,
        dependent_id: &str,
        dependency_id: &str,
        annotation: Option<EdgeAnnotation>,
    ) -> Result<(), Error> {
        let node = self
            .nodes
            .get_mut(dependent_id)
            .ok_or_else(|| Error::NodeNotFound(dependent_id.to_string()))?;

        let dep = node
            .frontmatter
            .dependencies
            .iter_mut()
            .find(|d| d.node_id == dependency_id)
            .ok_or_else(|| Error::EdgeNotFound {
                from: dependent_id.to_string(),
                to: dependency_id.to_string(),
            })?;

        dep.annotation = annotation;

        let file_path = self.root.join(format!("nodes/{dependent_id}.md"));
        node_parser::write_node_file(&file_path, node)?;

        Ok(())
    }
}
