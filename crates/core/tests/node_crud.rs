use std::fs;

use ex_grapha_core::{
    error::Error,
    model::{Dependency, EdgeAnnotation, NodeType},
    project::{init_project, InitOptions, NodeParams},
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
fn axiom(title: &str, content: &str) -> NodeParams {
    NodeParams {
        title: title.into(),
        node_type: NodeType::Axiom,
        tags: vec![],
        dependencies: vec![],
        relation: None,
        content: content.into(),
    }
}

// ── Create ────────────────────────────────────────────────

#[test]
fn create_axiom_node() {
    let dir = temp_dir("crud-create-axiom");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let node = kb
        .create_node(NodeParams {
            title: "My Axiom".into(),
            node_type: NodeType::Axiom,
            tags: vec!["physics".into()],
            dependencies: vec![],
            relation: None,
            content: "# My Axiom\n\nSome content.\n".into(),
        })
        .unwrap();

    // ID format
    assert!(node.frontmatter.id.starts_with("n-"));
    assert_eq!(node.frontmatter.id.len(), 8); // "n-" + 6 hex

    // Fields
    assert_eq!(node.frontmatter.title, "My Axiom");
    assert_eq!(node.frontmatter.node_type, NodeType::Axiom);
    assert_eq!(node.frontmatter.tags, vec!["physics"]);
    assert!(node.frontmatter.dependencies.is_empty());
    assert!(node.frontmatter.relation.is_none());

    // In-memory
    assert_eq!(kb.nodes.len(), 1);
    assert!(kb.nodes.contains_key(&node.frontmatter.id));

    // On disk
    let file_path = dir.join(format!("nodes/{}.md", node.frontmatter.id));
    assert!(file_path.is_file());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_deduction_node() {
    let dir = temp_dir("crud-create-deduction");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let axiom_node = kb
        .create_node(axiom("Base Axiom", "Axiom content."))
        .unwrap();
    let axiom_id = axiom_node.frontmatter.id.clone();

    let deduction = kb
        .create_node(NodeParams {
            title: "My Deduction".into(),
            node_type: NodeType::Deduction,
            tags: vec![],
            dependencies: vec![Dependency {
                node_id: axiom_id.clone(),
                annotation: Some(EdgeAnnotation {
                    label: "requires".into(),
                }),
            }],
            relation: Some(axiom_id.clone()),
            content: "Deduction content.".into(),
        })
        .unwrap();

    assert_eq!(deduction.frontmatter.node_type, NodeType::Deduction);
    assert_eq!(deduction.frontmatter.dependencies.len(), 1);
    assert!(deduction.frontmatter.relation.is_some());
    assert_eq!(kb.nodes.len(), 2);

    // Adjacency: axiom has a dependent
    let deps = kb.dependents.get(&axiom_id).unwrap();
    assert!(deps.contains(&deduction.frontmatter.id));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_deduction_without_deps_fails() {
    let dir = temp_dir("crud-create-ded-no-deps");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let err = kb
        .create_node(NodeParams {
            title: "Bad Deduction".into(),
            node_type: NodeType::Deduction,
            tags: vec![],
            dependencies: vec![],
            relation: Some("n-000000".into()),
            content: "Content.".into(),
        })
        .unwrap_err();

    assert!(matches!(err, Error::TypeConstraint(_)));
    assert!(kb.nodes.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_axiom_with_deps_fails() {
    let dir = temp_dir("crud-create-axiom-deps");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let err = kb
        .create_node(NodeParams {
            title: "Bad Axiom".into(),
            node_type: NodeType::Axiom,
            tags: vec![],
            dependencies: vec![Dependency {
                node_id: "n-000000".into(),
                annotation: None,
            }],
            relation: None,
            content: "Content.".into(),
        })
        .unwrap_err();

    assert!(matches!(err, Error::TypeConstraint(_)));

    let _ = fs::remove_dir_all(&dir);
}

// ── Read ──────────────────────────────────────────────────

#[test]
fn get_node_found() {
    let dir = temp_dir("crud-get-found");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let created = kb.create_node(axiom("Test", "Body.")).unwrap();

    let fetched = kb.get_node(&created.frontmatter.id).unwrap();
    assert_eq!(fetched.frontmatter.title, "Test");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn get_node_not_found() {
    let dir = temp_dir("crud-get-missing");
    let kb = init_project(&dir, &DEFAULTS).unwrap();

    let err = kb.get_node("n-nonexistent").unwrap_err();
    assert!(matches!(err, Error::NodeNotFound(_)));

    let _ = fs::remove_dir_all(&dir);
}

// ── Update ────────────────────────────────────────────────

#[test]
fn update_node_title_and_content() {
    let dir = temp_dir("crud-update-basic");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let created = kb
        .create_node(axiom("Original", "Original content."))
        .unwrap();
    let id = created.frontmatter.id.clone();

    let updated = kb
        .update_node(
            &id,
            NodeParams {
                title: "Updated Title".into(),
                node_type: NodeType::Axiom,
                tags: vec!["new-tag".into()],
                dependencies: vec![],
                relation: None,
                content: "Updated content.".into(),
            },
        )
        .unwrap();

    assert_eq!(updated.frontmatter.title, "Updated Title");
    assert_eq!(updated.frontmatter.tags, vec!["new-tag"]);
    assert_eq!(updated.content, "Updated content.");

    // Preserved fields
    assert_eq!(
        updated.frontmatter.created_at,
        created.frontmatter.created_at
    );
    assert_eq!(
        updated.frontmatter.created_by,
        created.frontmatter.created_by
    );

    // Verify on disk
    let on_disk =
        ex_grapha_core::frontmatter::read_node_file(&dir.join(format!("nodes/{id}.md"))).unwrap();
    assert_eq!(on_disk.frontmatter.title, "Updated Title");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn update_node_not_found() {
    let dir = temp_dir("crud-update-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let err = kb
        .update_node("n-nonexistent", axiom("Title", "Content."))
        .unwrap_err();

    assert!(matches!(err, Error::NodeNotFound(_)));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn update_type_conversion_axiom_to_deduction() {
    let dir = temp_dir("crud-type-a2d");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let axiom1 = kb.create_node(axiom("Axiom 1", "Base.")).unwrap();
    let a1_id = axiom1.frontmatter.id.clone();

    let axiom2 = kb
        .create_node(axiom("Axiom 2", "Will become deduction."))
        .unwrap();
    let a2_id = axiom2.frontmatter.id.clone();

    // Convert axiom2 to deduction depending on axiom1
    let updated = kb
        .update_node(
            &a2_id,
            NodeParams {
                title: "Now Deduction".into(),
                node_type: NodeType::Deduction,
                tags: vec![],
                dependencies: vec![Dependency {
                    node_id: a1_id.clone(),
                    annotation: None,
                }],
                relation: Some(a1_id.clone()),
                content: "Derived from axiom 1.".into(),
            },
        )
        .unwrap();

    assert_eq!(updated.frontmatter.node_type, NodeType::Deduction);
    assert_eq!(updated.frontmatter.dependencies.len(), 1);

    // Adjacency updated
    let deps = kb.dependents.get(&a1_id).unwrap();
    assert!(deps.contains(&a2_id));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn update_type_conversion_deduction_to_axiom() {
    let dir = temp_dir("crud-type-d2a");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let axiom_node = kb.create_node(axiom("Base", "Axiom.")).unwrap();
    let a_id = axiom_node.frontmatter.id.clone();

    let deduction = kb
        .create_node(NodeParams {
            title: "Derived".into(),
            node_type: NodeType::Deduction,
            tags: vec![],
            dependencies: vec![Dependency {
                node_id: a_id.clone(),
                annotation: None,
            }],
            relation: Some(a_id.clone()),
            content: "Deduction.".into(),
        })
        .unwrap();
    let d_id = deduction.frontmatter.id.clone();

    // Convert to axiom by clearing deps and relation
    let updated = kb
        .update_node(&d_id, axiom("Now Axiom", "Standalone."))
        .unwrap();

    assert_eq!(updated.frontmatter.node_type, NodeType::Axiom);
    assert!(updated.frontmatter.dependencies.is_empty());
    assert!(updated.frontmatter.relation.is_none());

    // Adjacency cleaned up
    let deps = kb.dependents.get(&a_id);
    assert!(deps.is_none() || deps.unwrap().is_empty());

    let _ = fs::remove_dir_all(&dir);
}

// ── Delete ────────────────────────────────────────────────

#[test]
fn delete_node_success() {
    let dir = temp_dir("crud-delete-ok");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let node = kb.create_node(axiom("Doomed", "Bye.")).unwrap();
    let id = node.frontmatter.id.clone();
    let file_path = dir.join(format!("nodes/{id}.md"));
    assert!(file_path.is_file());

    kb.delete_node(&id).unwrap();

    assert!(!kb.nodes.contains_key(&id));
    assert!(!file_path.exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn delete_node_blocked_by_dependents() {
    let dir = temp_dir("crud-delete-blocked");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let axiom_node = kb.create_node(axiom("Important", "Base.")).unwrap();
    let a_id = axiom_node.frontmatter.id.clone();

    let _deduction = kb
        .create_node(NodeParams {
            title: "Depends on it".into(),
            node_type: NodeType::Deduction,
            tags: vec![],
            dependencies: vec![Dependency {
                node_id: a_id.clone(),
                annotation: None,
            }],
            relation: Some(a_id.clone()),
            content: "Derived.".into(),
        })
        .unwrap();

    let err = kb.delete_node(&a_id).unwrap_err();
    match err {
        Error::DeletionBlocked {
            node_id,
            dependents,
        } => {
            assert_eq!(node_id, a_id);
            assert_eq!(dependents.len(), 1);
        }
        _ => panic!("expected DeletionBlocked, got: {err}"),
    }

    // Node still exists
    assert!(kb.nodes.contains_key(&a_id));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn delete_node_not_found() {
    let dir = temp_dir("crud-delete-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let err = kb.delete_node("n-nonexistent").unwrap_err();
    assert!(matches!(err, Error::NodeNotFound(_)));

    let _ = fs::remove_dir_all(&dir);
}

// ── ID generation ─────────────────────────────────────────

#[test]
fn create_node_generates_unique_ids() {
    let dir = temp_dir("crud-unique-ids");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let mut ids = std::collections::HashSet::new();
    for i in 0..20 {
        let node = kb.create_node(axiom(&format!("Node {i}"), "")).unwrap();
        ids.insert(node.frontmatter.id);
    }

    assert_eq!(ids.len(), 20);
    assert_eq!(kb.nodes.len(), 20);

    let _ = fs::remove_dir_all(&dir);
}
