//! Node CRUD operations on `KnowledgeBase`.

use std::fs;

use crate::{
    error::Error,
    model::{Dependency, Node, NodeFrontmatter, NodeType, Status},
    node_parser,
    project::KnowledgeBase,
    util,
};

/// Parameters for creating or updating a node.
pub struct NodeParams {
    pub title: String,
    pub node_type: NodeType,
    pub tags: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub relation: Option<String>,
    pub content: String,
}

impl KnowledgeBase {
    // ── Node CRUD ─────────────────────────────────────────

    /// Create a new node, write it to disk, and add it to the in-memory
    /// graph.
    pub fn create_node(&mut self, params: NodeParams) -> Result<Node, Error> {
        let now = util::now_iso8601();
        let username = util::os_username();

        let frontmatter = NodeFrontmatter {
            id: util::generate_node_id(),
            title: params.title,
            node_type: params.node_type,
            tags: params.tags,
            status: Status::Current,
            status_updated_at: now.clone(),
            status_updated_by: username.clone(),
            stale_sources: Vec::new(),
            created_at: now,
            created_by: username,
            dependencies: params.dependencies,
            relation: params.relation,
        };
        frontmatter.validate_type_constraints()?;

        let node = Node {
            frontmatter,
            content: params.content,
        };

        let file_path = self.root.join(format!("nodes/{}.md", node.frontmatter.id));
        node_parser::write_node_file(&file_path, &node)?;

        self.nodes.insert(node.frontmatter.id.clone(), node.clone());
        self.rebuild_adjacency();

        Ok(node)
    }

    /// Return a reference to a node by ID.
    pub fn get_node(&self, id: &str) -> Result<&Node, Error> {
        self.nodes
            .get(id)
            .ok_or_else(|| Error::NodeNotFound(id.to_string()))
    }

    /// Update an existing node's frontmatter and content, write to disk.
    pub fn update_node(&mut self, id: &str, params: NodeParams) -> Result<Node, Error> {
        let existing = self
            .nodes
            .get(id)
            .ok_or_else(|| Error::NodeNotFound(id.to_string()))?;

        let now = util::now_iso8601();

        let frontmatter = NodeFrontmatter {
            id: id.to_string(),
            title: params.title,
            node_type: params.node_type,
            tags: params.tags,
            status: existing.frontmatter.status.clone(),
            status_updated_at: now,
            status_updated_by: util::os_username(),
            stale_sources: existing.frontmatter.stale_sources.clone(),
            created_at: existing.frontmatter.created_at.clone(),
            created_by: existing.frontmatter.created_by.clone(),
            dependencies: params.dependencies,
            relation: params.relation,
        };
        frontmatter.validate_type_constraints()?;

        let node = Node {
            frontmatter,
            content: params.content,
        };

        let file_path = self.root.join(format!("nodes/{id}.md"));
        node_parser::write_node_file(&file_path, &node)?;

        self.nodes.insert(id.to_string(), node.clone());
        self.rebuild_adjacency();

        Ok(node)
    }

    /// Delete a node. Fails if other nodes depend on it.
    pub fn delete_node(&mut self, id: &str) -> Result<(), Error> {
        if !self.nodes.contains_key(id) {
            return Err(Error::NodeNotFound(id.to_string()));
        }

        // Check if any nodes depend on this one.
        if let Some(deps) = self.dependents.get(id) {
            if !deps.is_empty() {
                let mut dependents: Vec<String> = deps.iter().cloned().collect();
                dependents.sort();
                return Err(Error::DeletionBlocked {
                    node_id: id.to_string(),
                    dependents,
                });
            }
        }

        self.nodes.remove(id);

        let file_path = self.root.join(format!("nodes/{id}.md"));
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }

        self.rebuild_adjacency();

        Ok(())
    }
}
