//! Project scaffolding (init, open) and the `KnowledgeBase` in-memory state.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::ProjectConfig,
    error::Error,
    model::{Node, NodeId},
    node_parser,
};

/// A warning produced while loading a project (e.g., a malformed node file).
#[derive(Debug, Clone)]
pub struct LoadWarning {
    pub path: PathBuf,
    pub message: String,
}

/// In-memory representation of an open knowledge base project.
#[derive(Debug)]
pub struct KnowledgeBase {
    /// Absolute path to the project root directory.
    pub root: PathBuf,
    /// Project-level configuration from `.knowledgebase/config.yaml`.
    pub config: ProjectConfig,
    /// All loaded nodes, keyed by their ID.
    pub nodes: HashMap<NodeId, Node>,
    /// Reverse dependency map: `dependents[A] = {B, C}` means B and C depend on
    /// A. Used for staleness propagation and deletion blocking.
    pub dependents: HashMap<NodeId, HashSet<NodeId>>,
    /// Transitive dependency sets: `dependencies[A]` = all nodes that A
    /// (transitively) depends on. In-memory only, used for O(1) cycle
    /// detection.
    pub(crate) dependencies: HashMap<NodeId, HashSet<NodeId>>,
    /// Node IDs that appeared in more than one `.md` file during
    /// `open_project`. The `HashMap` silently overwrites duplicates, so we
    /// record them here for the validation engine to report.
    pub(crate) duplicate_ids: Vec<(NodeId, PathBuf)>,
}

impl KnowledgeBase {
    /// Write a node's current in-memory state to its `.md` file on disk.
    pub(crate) fn write_node_to_disk(&self, node_id: &str) -> Result<(), Error> {
        let node = self
            .nodes
            .get(node_id)
            .ok_or_else(|| Error::NodeNotFound(node_id.to_string()))?;
        let file_path = self.root.join(format!("nodes/{node_id}.md"));
        node_parser::write_node_file(&file_path, node)
    }

    /// Rebuild `dependents` and `dependencies` from the current node set.
    pub fn rebuild_indexes(&mut self) {
        // Rebuild dependents (reverse dependency map).
        self.dependents.clear();
        for node in self.nodes.values() {
            let dependent_id = &node.frontmatter.id;
            for dep in &node.frontmatter.dependencies {
                self.dependents
                    .entry(dep.node_id.clone())
                    .or_default()
                    .insert(dependent_id.clone());
            }
        }

        // Rebuild dependencies via memoized DFS.
        self.dependencies.clear();
        let ids: Vec<NodeId> = self.nodes.keys().cloned().collect();
        for id in ids {
            self.compute_dependencies(&id);
        }
    }

    /// Recursively compute transitive deps for `node_id`, caching results.
    ///
    /// **Cycle-safe**: an empty sentinel is inserted before recursing. If we
    /// encounter the same node again during descent, the sentinel is returned
    /// and the recursion terminates. For cyclic graphs this means a node in a
    /// cycle will end up containing itself in its transitive deps — the
    /// validation engine uses this property to detect cycles.
    fn compute_dependencies(&mut self, node_id: &str) -> HashSet<NodeId> {
        if let Some(cached) = self.dependencies.get(node_id) {
            return cached.clone();
        }

        // Sentinel: break infinite recursion on cycles.
        self.dependencies
            .insert(node_id.to_string(), HashSet::new());

        let direct_deps: Vec<NodeId> = self
            .nodes
            .get(node_id)
            .map(|n| {
                n.frontmatter
                    .dependencies
                    .iter()
                    .map(|d| d.node_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        let mut result = HashSet::new();
        for dep_id in &direct_deps {
            result.insert(dep_id.clone());
            let sub = self.compute_dependencies(dep_id);
            result.extend(sub);
        }

        self.dependencies
            .insert(node_id.to_string(), result.clone());
        result
    }
}

// ── Init ───────────────────────────────────────────────────

/// Options for scaffolding a new knowledge base project.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct InitOptions {
    /// Create `.knowledgebase/hooks/validate.sh` (generic git pre-commit hook).
    pub include_git_hook: bool,
    /// Create `.github/workflows/validate.yaml` (GitHub Actions CI workflow).
    pub include_github_workflow: bool,
}

/// Scaffold a new knowledge base project at `path`.
///
/// Creates the directory layout with sensible defaults and returns
/// a `KnowledgeBase` ready for use.
pub fn init_project(path: &Path, options: &InitOptions) -> Result<KnowledgeBase, Error> {
    if path.join(".knowledgebase").exists() {
        return Err(Error::InvalidProject(
            "directory already contains a .knowledgebase folder".into(),
        ));
    }

    // Create core directory structure (always present)
    fs::create_dir_all(path.join("nodes"))?;
    fs::create_dir_all(path.join("assets"))?;
    fs::create_dir_all(path.join(".knowledgebase"))?;

    // Write default config
    let config = ProjectConfig::default();
    let config_yaml = config.to_yaml()?;
    fs::write(path.join(".knowledgebase/config.yaml"), &config_yaml)?;

    // Write README
    fs::write(
        path.join("README.md"),
        "# Knowledge Base\n\nA dependency-aware knowledge graph managed by Ex Grapha.\n",
    )?;

    // Write .gitignore
    fs::write(path.join(".gitignore"), KB_GITIGNORE)?;

    // Optional: git pre-commit hook
    if options.include_git_hook {
        fs::create_dir_all(path.join(".knowledgebase/hooks"))?;
        fs::write(
            path.join(".knowledgebase/hooks/validate.sh"),
            VALIDATE_SH_TEMPLATE,
        )?;
    }

    // Optional: GitHub Actions workflow
    if options.include_github_workflow {
        fs::create_dir_all(path.join(".github/workflows"))?;
        fs::write(
            path.join(".github/workflows/validate.yaml"),
            VALIDATE_YAML_TEMPLATE,
        )?;
    }

    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    Ok(KnowledgeBase {
        root,
        config,
        nodes: HashMap::new(),
        dependents: HashMap::new(),
        dependencies: HashMap::new(),
        duplicate_ids: Vec::new(),
    })
}

// ── Open ───────────────────────────────────────────────────

/// Open an existing knowledge base project at `path`.
///
/// Validates the directory structure, loads all nodes, and builds
/// the in-memory graph. Returns the `KnowledgeBase` along with any
/// warnings for node files that could not be parsed.
pub fn open_project(path: &Path) -> Result<(KnowledgeBase, Vec<LoadWarning>), Error> {
    let root = path
        .canonicalize()
        .map_err(|e| Error::InvalidProject(format!("cannot resolve path: {e}")))?;

    // Validate required structure
    let nodes_dir = root.join("nodes");
    if !nodes_dir.is_dir() {
        return Err(Error::InvalidProject("missing `nodes/` directory".into()));
    }

    let config_path = root.join(".knowledgebase/config.yaml");
    if !config_path.is_file() {
        return Err(Error::InvalidProject(
            "missing `.knowledgebase/config.yaml`".into(),
        ));
    }

    // Load config
    let config_str = fs::read_to_string(&config_path)?;
    let config = ProjectConfig::from_yaml(&config_str)?;

    // Load all node files
    let mut nodes = HashMap::new();
    let mut warnings = Vec::new();
    let mut duplicate_ids = Vec::new();

    let entries: Vec<_> = fs::read_dir(&nodes_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    for entry in entries {
        let file_path = entry.path();
        match node_parser::read_node_file(&file_path) {
            Ok(node) => {
                let id = node.frontmatter.id.clone();
                if nodes.insert(id.clone(), node).is_some() {
                    warnings.push(LoadWarning {
                        path: file_path.clone(),
                        message: format!("duplicate node ID `{id}`; keeping the last file loaded"),
                    });
                    duplicate_ids.push((id, file_path.clone()));
                }
            }
            Err(e) => {
                warnings.push(LoadWarning {
                    path: file_path,
                    message: e.to_string(),
                });
            }
        }
    }

    let mut kb = KnowledgeBase {
        root,
        config,
        nodes,
        dependents: HashMap::new(),
        dependencies: HashMap::new(),
        duplicate_ids,
    };
    kb.rebuild_indexes();

    Ok((kb, warnings))
}

// ── Templates ──────────────────────────────────────────────

const VALIDATE_SH_TEMPLATE: &str = r#"#!/bin/sh
# Pre-commit hook: validate knowledge base integrity.
# Requires the ex-grapha CLI (will be available after Issue #5).
# ex-grapha validate .
echo "Knowledge base validation hook (not yet implemented)"
"#;

const VALIDATE_YAML_TEMPLATE: &str = r#"name: Validate Knowledge Base

on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate
        run: echo "Validation not yet implemented"
"#;

const KB_GITIGNORE: &str = "\
# Node positions (local display preference, not shared)
.positions.json
";
