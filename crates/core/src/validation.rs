//! Project-wide validation engine.
//!
//! # Why this exists
//!
//! CRUD operations (`create_node`, `create_edge`, …) enforce invariants
//! inline — they reject cycles, dangling refs, and type violations at the
//! point of mutation.  However, on-disk data can be edited externally (git
//! merges, manual edits, concurrent tools), so a separate "validate the
//! whole project" pass is useful.
//!
//! # Performance
//!
//! Every check is **O(V + E)** or better (V = nodes, E = total edges
//! across all nodes):
//!
//! | Check                   | Cost       | Notes                                 |
//! |-------------------------|-----------|---------------------------------------|
//! | Type constraints        | O(V)      | One call per node                     |
//! | Dangling references     | O(E)      | One O(1) HashMap lookup per edge      |
//! | Relation integrity      | O(V)      | One `parse_relation()` per deduction  |
//! | Cycle detection         | O(V)      | Scan `dependencies` for self-ref   |
//! | Tag / annotation refs   | O(V + E)  | HashSet lookups per tag and per edge  |
//! | Duplicate IDs           | O(D)      | D = number of duplicates (tiny)       |
//!
//! A 10 000-node / 50 000-edge graph validates in milliseconds.
//!
//! # How it relates to in-memory indexes
//!
//! `dependencies` is **purely in-memory** — never written to disk.
//! It is rebuilt from scratch every time `open_project()` runs. Users
//! cannot tamper with it; the only data on disk is each node's
//! `dependencies` list in the YAML frontmatter. The validation engine
//! checks that frontmatter-level data is consistent.

use std::collections::HashSet;

use serde::Serialize;

use crate::{
    model::NodeId,
    project::KnowledgeBase,
    relation::{self, RelationErrorKind},
};

// ── Types ──────────────────────────────────────────────────

/// Severity of a validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

/// Which validation rule was violated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRule {
    NoCycles,
    NoDanglingReferences,
    RelationIntegrity,
    RelationParsability,
    TypeConstraints,
    NoDuplicateIds,
    TagReference,
    AnnotationReference,
}

/// A single validation issue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub rule: ValidationRule,
    /// The node this issue pertains to (`None` for graph-level issues).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Human-readable description.
    pub message: String,
    /// Extra context: cycle path, missing IDs, undefined names, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>,
}

/// The result of running the full validation engine.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub error_count: usize,
    pub warning_count: usize,
}

impl ValidationReport {
    /// `true` when there are zero errors (warnings are OK).
    pub fn is_valid(&self) -> bool {
        self.error_count == 0
    }
}

// ── Engine ─────────────────────────────────────────────────

impl KnowledgeBase {
    /// Run **all** validation checks and return a report with every issue
    /// found.  The engine never fails fast — it collects everything.
    pub fn validate(&self) -> ValidationReport {
        let mut issues = Vec::new();

        self.check_duplicate_ids(&mut issues);
        self.check_type_constraints(&mut issues);
        self.check_dangling_references(&mut issues);
        self.check_relation_integrity(&mut issues);
        self.check_cycles(&mut issues);
        self.check_tag_references(&mut issues);
        self.check_annotation_references(&mut issues);

        let error_count = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        let warning_count = issues.len() - error_count;

        ValidationReport {
            issues,
            error_count,
            warning_count,
        }
    }

    // ── Individual checks ──────────────────────────────────

    /// Duplicate node IDs detected at load time.
    fn check_duplicate_ids(&self, issues: &mut Vec<ValidationIssue>) {
        for (id, path) in &self.duplicate_ids {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                rule: ValidationRule::NoDuplicateIds,
                node_id: Some(id.clone()),
                message: format!("duplicate node ID `{id}` found in `{}`", path.display()),
                context: Some(vec![path.display().to_string()]),
            });
        }
    }

    /// Axiom/deduction type constraints.
    fn check_type_constraints(&self, issues: &mut Vec<ValidationIssue>) {
        for node in self.nodes.values() {
            if let Err(e) = node.frontmatter.validate_type_constraints() {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    rule: ValidationRule::TypeConstraints,
                    node_id: Some(node.frontmatter.id.clone()),
                    message: e.to_string(),
                    context: None,
                });
            }
        }
    }

    /// Every dependency must point to an existing node.
    fn check_dangling_references(&self, issues: &mut Vec<ValidationIssue>) {
        for node in self.nodes.values() {
            for dep in &node.frontmatter.dependencies {
                if !self.nodes.contains_key(&dep.node_id) {
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        rule: ValidationRule::NoDanglingReferences,
                        node_id: Some(node.frontmatter.id.clone()),
                        message: format!(
                            "node `{}` depends on `{}` which does not exist",
                            node.frontmatter.id, dep.node_id
                        ),
                        context: Some(vec![dep.node_id.clone()]),
                    });
                }
            }
        }
    }

    /// relation expression syntax + operand matching.
    ///
    /// Skips nodes without a relation (those are caught by
    /// `check_type_constraints` if they should have one).
    fn check_relation_integrity(&self, issues: &mut Vec<ValidationIssue>) {
        for node in self.nodes.values() {
            let relation_str = match &node.frontmatter.relation {
                Some(r) => r,
                None => continue,
            };

            let dep_ids: Vec<String> = node
                .frontmatter
                .dependencies
                .iter()
                .map(|d| d.node_id.clone())
                .collect();

            if let Err(errors) = relation::parse_relation(relation_str, &dep_ids) {
                for err in errors {
                    let rule = match err.kind {
                        RelationErrorKind::UnknownOperand | RelationErrorKind::MissingOperand => {
                            ValidationRule::RelationIntegrity
                        }
                        _ => ValidationRule::RelationParsability,
                    };
                    issues.push(ValidationIssue {
                        severity: Severity::Error,
                        rule,
                        node_id: Some(node.frontmatter.id.clone()),
                        message: format!("node `{}`: {}", node.frontmatter.id, err.message),
                        context: None,
                    });
                }
            }
        }
    }

    /// The dependency graph must be a DAG (no cycles).
    ///
    /// Leverages the already-computed `dependencies`.  With the
    /// cycle-safe `compute_dependencies`, a node in a cycle will
    /// contain itself in its own transitive deps set.  We detect that
    /// here and extract a concrete cycle path via targeted DFS.
    fn check_cycles(&self, issues: &mut Vec<ValidationIssue>) {
        // Find any node that is in its own transitive deps.
        let flagged = self
            .dependencies
            .iter()
            .find(|(id, deps)| deps.contains(*id));

        let start_id = match flagged {
            Some((id, _)) => id.clone(),
            None => return,
        };

        // Extract a concrete cycle path starting from the flagged node.
        let cycle_path = self.extract_cycle_path(&start_id);

        issues.push(ValidationIssue {
            severity: Severity::Error,
            rule: ValidationRule::NoCycles,
            node_id: None,
            message: format!(
                "dependency graph contains a cycle: {}",
                cycle_path.join(" -> ")
            ),
            context: Some(cycle_path),
        });
    }

    /// Starting from `start_id` (known to be in a cycle), follow
    /// dependency edges until we revisit a node, then trim to the cycle.
    fn extract_cycle_path(&self, start_id: &str) -> Vec<String> {
        let mut path = vec![start_id.to_string()];
        let mut visited: HashSet<&str> = HashSet::new();
        visited.insert(start_id);
        let mut current = start_id;

        loop {
            // Pick the first dependency that is also in a cycle (i.e.,
            // appears in its own transitive deps).
            let next = self.nodes.get(current).and_then(|node| {
                node.frontmatter
                    .dependencies
                    .iter()
                    .map(|d| d.node_id.as_str())
                    .find(|dep_id| {
                        self.dependencies
                            .get(*dep_id)
                            .is_some_and(|deps| deps.contains(*dep_id))
                            || *dep_id == start_id
                    })
            });

            match next {
                Some(next_id) => {
                    if visited.contains(next_id) {
                        // Found the cycle closure — trim to just the cycle.
                        if let Some(pos) = path.iter().position(|p| p == next_id) {
                            let mut cycle: Vec<String> = path[pos..].to_vec();
                            cycle.push(next_id.to_string());
                            return cycle;
                        }
                        path.push(next_id.to_string());
                        return path;
                    }
                    visited.insert(next_id);
                    path.push(next_id.to_string());
                    current = next_id;
                }
                None => return path,
            }
        }
    }

    /// Warn when a node uses a tag not defined in config.
    fn check_tag_references(&self, issues: &mut Vec<ValidationIssue>) {
        let defined: HashSet<&str> = self
            .config
            .tag_definitions
            .iter()
            .map(|t| t.name.as_str())
            .collect();

        for node in self.nodes.values() {
            for tag in &node.frontmatter.tags {
                if !defined.contains(tag.as_str()) {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        rule: ValidationRule::TagReference,
                        node_id: Some(node.frontmatter.id.clone()),
                        message: format!(
                            "node `{}` uses tag `{tag}` not defined in config",
                            node.frontmatter.id
                        ),
                        context: Some(vec![tag.clone()]),
                    });
                }
            }
        }
    }

    /// Warn when an edge uses an annotation label
    /// not defined in config.
    fn check_annotation_references(&self, issues: &mut Vec<ValidationIssue>) {
        let defined: HashSet<&str> = self
            .config
            .edge_annotations
            .iter()
            .map(|a| a.label.as_str())
            .collect();

        for node in self.nodes.values() {
            for dep in &node.frontmatter.dependencies {
                if let Some(ann) = &dep.annotation {
                    if !defined.contains(ann.label.as_str()) {
                        issues.push(ValidationIssue {
                            severity: Severity::Warning,
                            rule: ValidationRule::AnnotationReference,
                            node_id: Some(node.frontmatter.id.clone()),
                            message: format!(
                                "node `{}` uses annotation `{}` on dependency `{}` not defined in config",
                                node.frontmatter.id, ann.label, dep.node_id
                            ),
                            context: Some(vec![ann.label.clone()]),
                        });
                    }
                }
            }
        }
    }
}
