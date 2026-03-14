use std::fs;

use ex_grapha_core::{
    error::Error,
    model::{Dependency, EdgeAnnotation, NodeType},
    node::NodeParams,
    project::{init_project, InitOptions},
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
        content: String::new(),
    }
}

/// Create a deduction node depending on the given axiom IDs.
fn deduction(title: &str, dep_ids: &[&str], relation: &str) -> NodeParams {
    NodeParams {
        title: title.into(),
        node_type: NodeType::Deduction,
        tags: vec![],
        dependencies: dep_ids
            .iter()
            .map(|id| Dependency {
                node_id: id.to_string(),
                annotation: None,
            })
            .collect(),
        relation: Some(relation.into()),
        content: String::new(),
    }
}

// ── Create edge ──────────────────────────────────────────

#[test]
fn create_edge_basic() {
    let dir = temp_dir("edge-create-basic");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    // B depends on A
    kb.create_edge(&b_id, &a_id, None).unwrap();

    let b_node = kb.get_node(&b_id).unwrap();
    assert_eq!(b_node.frontmatter.dependencies.len(), 1);
    assert_eq!(b_node.frontmatter.dependencies[0].node_id, a_id);
    assert!(b_node.frontmatter.dependencies[0].annotation.is_none());

    // Reverse dep map updated
    let deps = kb.dependents.get(&a_id).unwrap();
    assert!(deps.contains(&b_id));

    // Persisted on disk
    let on_disk =
        ex_grapha_core::node_parser::read_node_file(&dir.join(format!("nodes/{b_id}.md"))).unwrap();
    assert_eq!(on_disk.frontmatter.dependencies.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_edge_with_annotation() {
    let dir = temp_dir("edge-create-annotated");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    kb.create_edge(
        &b_id,
        &a_id,
        Some(EdgeAnnotation {
            label: "requires".into(),
        }),
    )
    .unwrap();

    let b_node = kb.get_node(&b_id).unwrap();
    assert_eq!(
        b_node.frontmatter.dependencies[0]
            .annotation
            .as_ref()
            .unwrap()
            .label,
        "requires"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_edge_node_not_found() {
    let dir = temp_dir("edge-create-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    // Dependent doesn't exist
    let err = kb.create_edge("n-nonexistent", &a_id, None).unwrap_err();
    assert!(matches!(err, Error::NodeNotFound(_)));

    // Dependency doesn't exist
    let err = kb.create_edge(&a_id, "n-nonexistent", None).unwrap_err();
    assert!(matches!(err, Error::NodeNotFound(_)));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_edge_already_exists() {
    let dir = temp_dir("edge-create-dup");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    kb.create_edge(&b_id, &a_id, None).unwrap();
    let err = kb.create_edge(&b_id, &a_id, None).unwrap_err();
    assert!(matches!(err, Error::EdgeAlreadyExists { .. }));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_edge_self_loop() {
    let dir = temp_dir("edge-create-self-loop");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let err = kb.create_edge(&a_id, &a_id, None).unwrap_err();
    assert!(matches!(err, Error::CycleDetected { .. }));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_edge_direct_cycle() {
    let dir = temp_dir("edge-create-direct-cycle");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    // B depends on A
    kb.create_edge(&b_id, &a_id, None).unwrap();

    // A depends on B would create a cycle
    let err = kb.create_edge(&a_id, &b_id, None).unwrap_err();
    assert!(matches!(err, Error::CycleDetected { .. }));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_edge_transitive_cycle() {
    let dir = temp_dir("edge-create-transitive-cycle");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let c = kb.create_node(axiom("C")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();
    let c_id = c.frontmatter.id.clone();

    // B depends on A, C depends on B
    kb.create_edge(&b_id, &a_id, None).unwrap();
    kb.create_edge(&c_id, &b_id, None).unwrap();

    // A depends on C would create A->C->B->A cycle
    let err = kb.create_edge(&a_id, &c_id, None).unwrap_err();
    assert!(matches!(err, Error::CycleDetected { .. }));

    let _ = fs::remove_dir_all(&dir);
}

// ── Validate edge deletion ───────────────────────────────

#[test]
fn validate_edge_deletion_not_last() {
    let dir = temp_dir("edge-validate-not-last");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let c = kb.create_node(axiom("C")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();
    let c_id = c.frontmatter.id.clone();

    // C depends on both A and B
    let c_node = kb
        .create_node(deduction(
            "C deduction",
            &[&a_id, &b_id],
            &format!("({a_id} AND {b_id})"),
        ))
        .unwrap();
    let c_ded_id = c_node.frontmatter.id.clone();

    // Deleting one of two deps is not the last
    let check = kb.validate_edge_deletion(&c_ded_id, &a_id).unwrap();
    assert!(!check.is_last_dependency);
    assert_eq!(check.node_title, "C deduction");

    // Clean up the extra axiom C that was unused
    kb.delete_node(&c_id).unwrap();
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_edge_deletion_last() {
    let dir = temp_dir("edge-validate-last");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    // B depends only on A
    let b = kb
        .create_node(deduction("B deduction", &[&a_id], &a_id))
        .unwrap();
    let b_id = b.frontmatter.id.clone();

    let check = kb.validate_edge_deletion(&b_id, &a_id).unwrap();
    assert!(check.is_last_dependency);
    assert_eq!(check.node_title, "B deduction");

    let _ = fs::remove_dir_all(&dir);
}

// ── Delete edge ──────────────────────────────────────────

#[test]
fn delete_edge_basic() {
    let dir = temp_dir("edge-delete-basic");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    // C depends on A and B
    let c = kb
        .create_node(deduction(
            "C",
            &[&a_id, &b_id],
            &format!("({a_id} AND {b_id})"),
        ))
        .unwrap();
    let c_id = c.frontmatter.id.clone();

    // Delete one edge
    kb.delete_edge(&c_id, &a_id).unwrap();

    let c_node = kb.get_node(&c_id).unwrap();
    assert_eq!(c_node.frontmatter.dependencies.len(), 1);
    assert_eq!(c_node.frontmatter.dependencies[0].node_id, b_id);

    // Reverse dep map updated
    let a_deps = kb.dependents.get(&a_id);
    assert!(a_deps.is_none() || !a_deps.unwrap().contains(&c_id));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn delete_edge_not_found() {
    let dir = temp_dir("edge-delete-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    let err = kb.delete_edge(&a_id, &b_id).unwrap_err();
    assert!(matches!(err, Error::EdgeNotFound { .. }));

    let _ = fs::remove_dir_all(&dir);
}

// ── Remove dependency and convert to axiom ───────────────

#[test]
fn remove_dependency_and_convert_to_axiom_basic() {
    let dir = temp_dir("edge-remove-convert");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    // B depends only on A
    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.node_type,
        NodeType::Deduction
    );

    kb.remove_dependency_and_convert_to_axiom(&b_id, &a_id)
        .unwrap();

    let b_node = kb.get_node(&b_id).unwrap();
    assert_eq!(b_node.frontmatter.node_type, NodeType::Axiom);
    assert!(b_node.frontmatter.dependencies.is_empty());
    assert!(b_node.frontmatter.relation.is_none());

    // Verify on disk
    let on_disk =
        ex_grapha_core::node_parser::read_node_file(&dir.join(format!("nodes/{b_id}.md"))).unwrap();
    assert_eq!(on_disk.frontmatter.node_type, NodeType::Axiom);

    let _ = fs::remove_dir_all(&dir);
}

// ── Update edge annotation ───────────────────────────────

#[test]
fn update_edge_annotation_basic() {
    let dir = temp_dir("edge-update-ann");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    kb.create_edge(&b_id, &a_id, None).unwrap();

    // Set annotation
    kb.update_edge_annotation(
        &b_id,
        &a_id,
        Some(EdgeAnnotation {
            label: "supports".into(),
        }),
    )
    .unwrap();

    let b_node = kb.get_node(&b_id).unwrap();
    assert_eq!(
        b_node.frontmatter.dependencies[0]
            .annotation
            .as_ref()
            .unwrap()
            .label,
        "supports"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn update_edge_annotation_remove() {
    let dir = temp_dir("edge-update-ann-remove");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    kb.create_edge(
        &b_id,
        &a_id,
        Some(EdgeAnnotation {
            label: "requires".into(),
        }),
    )
    .unwrap();

    // Remove annotation
    kb.update_edge_annotation(&b_id, &a_id, None).unwrap();

    let b_node = kb.get_node(&b_id).unwrap();
    assert!(b_node.frontmatter.dependencies[0].annotation.is_none());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn update_edge_annotation_not_found() {
    let dir = temp_dir("edge-update-ann-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    let err = kb
        .update_edge_annotation(
            &a_id,
            &b_id,
            Some(EdgeAnnotation {
                label: "test".into(),
            }),
        )
        .unwrap_err();
    assert!(matches!(err, Error::EdgeNotFound { .. }));

    let _ = fs::remove_dir_all(&dir);
}
