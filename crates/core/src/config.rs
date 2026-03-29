//! Project-level configuration (`config.yaml`): tags and display toggles.

use serde::{Deserialize, Serialize};

/// A project-level tag definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagDef {
    pub name: String,
}

/// Display toggles for the graph canvas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub relation_nodes: bool,
}

/// Project-level configuration stored in `.knowledgebase/config.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub display: DisplayConfig,
    pub tag_definitions: Vec<TagDef>,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            relation_nodes: true,
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            display: DisplayConfig::default(),
            tag_definitions: vec![
                TagDef {
                    name: "well-established".into(),
                },
                TagDef {
                    name: "tentative".into(),
                },
                TagDef {
                    name: "speculative".into(),
                },
            ],
        }
    }
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
