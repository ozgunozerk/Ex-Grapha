use std::fs;

use ex_grapha_core::{
    model::{Dependency, NodeType, Status},
    node::NodeParams,
    project::{init_project, InitOptions},
    watcher::NodeChangeKind,
};

/// Default options: no git integration files.
const DEFAULTS: InitOptions = InitOptions {
    include_git_hook: false,
    include_github_workflow: false,
};

/// Create a unique temp directory for each test.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("ex-grapha-tests").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Shorthand for building an axiom NodeParams.
fn axiom(title: &str) -> NodeParams {
    NodeParams {
        title: title.into(),
        node_type: NodeType::Axiom,
        tags: vec![],
        dependencies: vec![],
        relation: None,
        content: format!("# {title}\n"),
    }
}

/// Shorthand for building a deduction NodeParams.
fn deduction(title: &str, dep_ids: &[&str], relation: &str) -> NodeParams {
    NodeParams {
        title: title.into(),
        node_type: NodeType::Deduction,
        tags: vec![],
        dependencies: dep_ids
            .iter()
            .map(|id| Dependency {
                node_id: id.to_string(),
            })
            .collect(),
        relation: Some(relation.into()),
        content: format!("# {title}\n"),
    }
}

/// Write a raw axiom node file to disk (simulating an external editor).
fn write_axiom_file(dir: &std::path::Path, id: &str, title: &str) {
    let content = format!(
        "\
---
id: \"{id}\"
title: \"{title}\"
type: \"axiom\"
tags: []
status: \"current\"
status_updated_at: \"2026-03-14T00:00:00Z\"
status_updated_by: \"external\"
created_at: \"2026-03-14T00:00:00Z\"
created_by: \"external\"
dependencies: []
---

# {title}

Content of {title}.
"
    );
    fs::write(dir.join(format!("nodes/{id}.md")), content).unwrap();
}

/// Write a raw deduction node file to disk (simulating an external editor).
fn write_deduction_file(
    dir: &std::path::Path,
    id: &str,
    title: &str,
    dep_ids: &[&str],
    relation: &str,
) {
    let deps_yaml: String = dep_ids
        .iter()
        .map(|d| format!("  - node_id: \"{d}\""))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "\
---
id: \"{id}\"
title: \"{title}\"
type: \"deduction\"
tags: []
status: \"current\"
status_updated_at: \"2026-03-14T00:00:00Z\"
status_updated_by: \"external\"
created_at: \"2026-03-14T00:00:00Z\"
created_by: \"external\"
dependencies:
{deps_yaml}
relation: \"{relation}\"
---

# {title}

Content of {title}.
"
    );
    fs::write(dir.join(format!("nodes/{id}.md")), content).unwrap();
}

// ── External create ──────────────────────────────────────

#[test]
fn ingest_external_node_create() {
    let dir = temp_dir("watcher-ext-create");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    assert_eq!(kb.nodes.len(), 0);

    // Write a new node file externally.
    write_axiom_file(&dir, "n-ext001", "External Axiom");

    // Ingest it.
    let result = kb
        .ingest_external_node(&dir.join("nodes/n-ext001.md"))
        .unwrap();

    assert_eq!(result.kind, NodeChangeKind::Created);
    assert_eq!(result.node_id, "n-ext001");
    assert!(result.stale_affected.is_empty());
    assert!(result.orphaned_dependents.is_empty());

    // Node should now be in memory.
    assert_eq!(kb.nodes.len(), 1);
    let node = kb.get_node("n-ext001").unwrap();
    assert_eq!(node.frontmatter.title, "External Axiom");

    let _ = fs::remove_dir_all(&dir);
}

// ── External modify (content change) ────────────────────

#[test]
fn ingest_external_node_modify_triggers_staleness() {
    let dir = temp_dir("watcher-ext-modify");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Create an axiom and a deduction via the API.
    let a = kb.create_node(axiom("Axiom A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb
        .create_node(deduction("Deduction B", &[&a_id], &a_id))
        .unwrap();
    let b_id = b.frontmatter.id.clone();

    // Both should be current.
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Current
    );

    // Externally modify A's file (change the title).
    write_axiom_file(&dir, &a_id, "Axiom A (externally edited)");

    let result = kb
        .ingest_external_node(&dir.join(format!("nodes/{a_id}.md")))
        .unwrap();

    assert_eq!(result.kind, NodeChangeKind::Modified);
    assert_eq!(result.node_id, a_id);
    // B should have been marked stale.
    assert!(result.stale_affected.contains(&b_id));

    // Verify in-memory state.
    assert_eq!(
        kb.get_node(&a_id).unwrap().frontmatter.title,
        "Axiom A (externally edited)"
    );
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Stale
    );

    let _ = fs::remove_dir_all(&dir);
}

// ── Self-write detection ─────────────────────────────────

#[test]
fn ingest_external_node_self_write_unchanged() {
    let dir = temp_dir("watcher-self-write");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Create a node via the API (which writes to disk).
    let a = kb.create_node(axiom("Axiom A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    // Now ingest the same file — should be Unchanged (self-write).
    let result = kb
        .ingest_external_node(&dir.join(format!("nodes/{a_id}.md")))
        .unwrap();

    assert_eq!(result.kind, NodeChangeKind::Unchanged);
    assert_eq!(result.node_id, a_id);

    let _ = fs::remove_dir_all(&dir);
}

// ── External delete ──────────────────────────────────────

#[test]
fn remove_external_node_basic() {
    let dir = temp_dir("watcher-ext-delete");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("Axiom A")).unwrap();
    let a_id = a.frontmatter.id.clone();
    assert_eq!(kb.nodes.len(), 1);

    // Simulate external deletion (remove the file, then call remove_external_node).
    fs::remove_file(dir.join(format!("nodes/{a_id}.md"))).unwrap();

    let result = kb.remove_external_node(&a_id).unwrap();

    assert_eq!(result.kind, NodeChangeKind::Deleted);
    assert_eq!(result.node_id, a_id);
    assert!(result.orphaned_dependents.is_empty());

    // Node should be gone from memory.
    assert_eq!(kb.nodes.len(), 0);
    assert!(kb.get_node(&a_id).is_err());

    let _ = fs::remove_dir_all(&dir);
}

// ── Delete with dependents (orphaned) ────────────────────

#[test]
fn remove_external_node_reports_orphaned_dependents() {
    let dir = temp_dir("watcher-ext-delete-orphan");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // A → B (B depends on A).
    let a = kb.create_node(axiom("Axiom A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb
        .create_node(deduction("Deduction B", &[&a_id], &a_id))
        .unwrap();
    let b_id = b.frontmatter.id.clone();

    // Externally delete A (without going through delete_node).
    fs::remove_file(dir.join(format!("nodes/{a_id}.md"))).unwrap();

    let result = kb.remove_external_node(&a_id).unwrap();

    assert_eq!(result.kind, NodeChangeKind::Deleted);
    assert!(result.orphaned_dependents.contains(&b_id));

    // A is gone, B still exists but now has a dangling dependency.
    assert!(kb.get_node(&a_id).is_err());
    assert!(kb.get_node(&b_id).is_ok());

    let _ = fs::remove_dir_all(&dir);
}

// ── Delete nonexistent node errors ───────────────────────

#[test]
fn remove_external_node_nonexistent_errors() {
    let dir = temp_dir("watcher-ext-delete-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let err = kb.remove_external_node("n-nonexistent").unwrap_err();
    assert!(err.to_string().contains("not found"));

    let _ = fs::remove_dir_all(&dir);
}

// ── Config reload ────────────────────────────────────────

#[test]
fn reload_config_picks_up_changes() {
    let dir = temp_dir("watcher-config-reload");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Externally modify config.yaml.
    let new_config = "\
display:
  relation_nodes: false
tag_definitions:
  - name: custom-tag
";
    fs::write(dir.join(".knowledgebase/config.yaml"), new_config).unwrap();

    // Reload.
    kb.reload_config().unwrap();

    assert!(!kb.config.display.relation_nodes);
    assert_eq!(kb.config.tag_definitions.len(), 1);
    assert_eq!(kb.config.tag_definitions[0].name, "custom-tag");

    let _ = fs::remove_dir_all(&dir);
}

// ── Invalid file returns error ───────────────────────────

#[test]
fn ingest_invalid_file_returns_error() {
    let dir = temp_dir("watcher-invalid-file");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Write a malformed node file.
    fs::write(
        dir.join("nodes/n-bad001.md"),
        "this is not valid frontmatter",
    )
    .unwrap();

    let result = kb.ingest_external_node(&dir.join("nodes/n-bad001.md"));
    assert!(result.is_err());

    // KB should be unchanged.
    assert_eq!(kb.nodes.len(), 0);

    let _ = fs::remove_dir_all(&dir);
}

// ── Indexes rebuilt on create ────────────────────────────

#[test]
fn indexes_rebuilt_after_external_create() {
    let dir = temp_dir("watcher-indexes-create");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Create an axiom via the API.
    let a = kb.create_node(axiom("Axiom A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    // Externally write a deduction that depends on A.
    write_deduction_file(&dir, "n-ext002", "Ext Deduction", &[&a_id], &a_id);

    let result = kb
        .ingest_external_node(&dir.join("nodes/n-ext002.md"))
        .unwrap();
    assert_eq!(result.kind, NodeChangeKind::Created);

    // The dependents index should reflect the new dependency.
    let dependents_of_a = kb.dependents.get(&a_id);
    assert!(dependents_of_a.is_some());
    assert!(dependents_of_a.unwrap().contains("n-ext002"));

    let _ = fs::remove_dir_all(&dir);
}

// ── Indexes rebuilt on delete ────────────────────────────

#[test]
fn indexes_rebuilt_after_external_delete() {
    let dir = temp_dir("watcher-indexes-delete");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Create A → B.
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    // Verify B appears in dependents of A.
    assert!(kb.dependents.get(&a_id).unwrap().contains(&b_id));

    // Externally delete B.
    fs::remove_file(dir.join(format!("nodes/{b_id}.md"))).unwrap();
    kb.remove_external_node(&b_id).unwrap();

    // After rebuild, A should have no dependents.
    let dependents_of_a = kb.dependents.get(&a_id);
    assert!(
        dependents_of_a.is_none() || dependents_of_a.unwrap().is_empty(),
        "A should have no dependents after B was deleted"
    );

    let _ = fs::remove_dir_all(&dir);
}
