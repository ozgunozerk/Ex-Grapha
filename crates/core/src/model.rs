use serde::{Deserialize, Serialize};

/// Node identifier, e.g., `"n-4a7b2c"`.
pub type NodeId = String;

/// Whether a node is foundational (`axiom`) or derived (`deduction`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Axiom,
    Deduction,
}

/// Staleness lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Current,
    Stale,
}

/// An optional label on an edge (color is resolved from project config at display time).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeAnnotation {
    pub label: String,
}

/// A dependency entry in a node's frontmatter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub node_id: NodeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<EdgeAnnotation>,
}

/// Records which dependency triggered staleness and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleSource {
    pub node_id: NodeId,
    pub changed_at: String,
}

/// YAML frontmatter of a node file.
///
/// Field order matches the canonical file format so that `serde_yaml`
/// serialization produces a predictable layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeFrontmatter {
    pub id: NodeId,
    pub title: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    #[serde(default)]
    pub tags: Vec<String>,
    pub status: Status,
    pub status_updated_at: String,
    pub status_updated_by: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale_sources: Vec<StaleSource>,
    pub created_at: String,
    pub created_by: String,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
}

/// A complete node: parsed frontmatter + raw markdown body.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub frontmatter: NodeFrontmatter,
    pub content: String,
}

impl NodeFrontmatter {
    /// Check axiom/deduction type constraints.
    ///
    /// - Axiom: empty deps, no relation, no stale_sources.
    /// - Deduction: non-empty deps, relation present.
    pub fn validate_type_constraints(&self) -> Result<(), crate::error::Error> {
        match self.node_type {
            NodeType::Axiom => {
                if !self.dependencies.is_empty() {
                    return Err(crate::error::Error::TypeConstraint(
                        "axiom nodes must have empty dependencies".into(),
                    ));
                }
                if self.relation.is_some() {
                    return Err(crate::error::Error::TypeConstraint(
                        "axiom nodes must not have a relation expression".into(),
                    ));
                }
                if !self.stale_sources.is_empty() {
                    return Err(crate::error::Error::TypeConstraint(
                        "axiom nodes must not have stale_sources".into(),
                    ));
                }
            }
            NodeType::Deduction => {
                if self.dependencies.is_empty() {
                    return Err(crate::error::Error::TypeConstraint(
                        "deduction nodes must have at least one dependency".into(),
                    ));
                }
                if self.relation.is_none() {
                    return Err(crate::error::Error::TypeConstraint(
                        "deduction nodes must have a relation expression".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}
