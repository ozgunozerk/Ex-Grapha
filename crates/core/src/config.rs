use serde::{Deserialize, Serialize};

/// A project-level edge annotation definition (label + display color).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationDef {
    pub label: String,
    pub color: String,
}

/// A project-level tag definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagDef {
    pub name: String,
}

/// Display toggles for the graph canvas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub edge_labels: bool,
    pub edge_colors: bool,
    pub relation_nodes: bool,
}

/// Project-level configuration stored in `.knowledgebase/config.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub edge_annotations: Vec<AnnotationDef>,
    pub display: DisplayConfig,
    pub tag_definitions: Vec<TagDef>,
}

impl ProjectConfig {
    /// Parse a `config.yaml` string into a `ProjectConfig`.
    pub fn from_yaml(yaml: &str) -> Result<Self, crate::error::Error> {
        Ok(serde_yaml::from_str(yaml)?)
    }

    /// Serialize to a YAML string.
    pub fn to_yaml(&self) -> Result<String, crate::error::Error> {
        Ok(serde_yaml::to_string(self)?)
    }
}
