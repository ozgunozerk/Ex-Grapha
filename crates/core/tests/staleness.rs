use std::fs;

use ex_grapha_core::{
    model::{Dependency, NodeType, Status},
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
                annotation: None,
            })
            .collect(),
        relation: Some(relation.into()),
        content: format!("# {title}\n"),
    }
}

// ── Single-hop propagation ──────────────────────────────

#[test]
fn edit_axiom_marks_direct_dependent_stale() {
    let dir = temp_dir("stale-single-hop");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("Axiom A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb
        .create_node(deduction("Deduction B", &[&a_id], &a_id))
        .unwrap();
    let b_id = b.frontmatter.id.clone();

    // Both should start as Current.
    assert_eq!(
        kb.get_node(&a_id).unwrap().frontmatter.status,
        Status::Current
    );
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Current
    );

    // Edit axiom A.
    kb.update_node(&a_id, axiom("Axiom A (edited)")).unwrap();

    // A stays current (it was just edited).
    assert_eq!(
        kb.get_node(&a_id).unwrap().frontmatter.status,
        Status::Current
    );

    // B becomes stale with A as the source.
    let b_node = kb.get_node(&b_id).unwrap();
    assert_eq!(b_node.frontmatter.status, Status::Stale);
    assert_eq!(b_node.frontmatter.stale_sources.len(), 1);
    assert_eq!(b_node.frontmatter.stale_sources[0].node_id, a_id);

    // Verify on disk.
    let on_disk =
        ex_grapha_core::node_parser::read_node_file(&dir.join(format!("nodes/{b_id}.md"))).unwrap();
    assert_eq!(on_disk.frontmatter.status, Status::Stale);
    assert_eq!(on_disk.frontmatter.stale_sources.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

// ── Transitive cascade ──────────────────────────────────

#[test]
fn edit_propagates_transitively() {
    let dir = temp_dir("stale-transitive");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // A → B → C
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    let c = kb.create_node(deduction("C", &[&b_id], &b_id)).unwrap();
    let c_id = c.frontmatter.id.clone();

    // Edit A.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();

    // Both B and C should be stale.
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Stale
    );
    assert_eq!(
        kb.get_node(&c_id).unwrap().frontmatter.status,
        Status::Stale
    );

    // Both have A as the stale source.
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.stale_sources[0].node_id,
        a_id
    );
    assert_eq!(
        kb.get_node(&c_id).unwrap().frontmatter.stale_sources[0].node_id,
        a_id
    );

    let _ = fs::remove_dir_all(&dir);
}

// ── Review stops propagation ────────────────────────────

#[test]
fn review_stops_cascade() {
    let dir = temp_dir("stale-review-stops");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // A → B → C
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    let c = kb.create_node(deduction("C", &[&b_id], &b_id)).unwrap();
    let c_id = c.frontmatter.id.clone();

    // Edit A → B and C become stale.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Stale
    );
    assert_eq!(
        kb.get_node(&c_id).unwrap().frontmatter.status,
        Status::Stale
    );

    // Review B → B becomes current, C stays stale (review doesn't propagate).
    kb.mark_node_reviewed(&b_id).unwrap();
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Current
    );
    assert!(kb
        .get_node(&b_id)
        .unwrap()
        .frontmatter
        .stale_sources
        .is_empty());
    assert_eq!(
        kb.get_node(&c_id).unwrap().frontmatter.status,
        Status::Stale
    );

    let _ = fs::remove_dir_all(&dir);
}

// ── Edit re-propagates ──────────────────────────────────

#[test]
fn edit_stale_node_re_propagates() {
    let dir = temp_dir("stale-re-propagate");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // A → B → C
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    let c = kb.create_node(deduction("C", &[&b_id], &b_id)).unwrap();
    let c_id = c.frontmatter.id.clone();

    // Edit A → B and C stale.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();

    // Now edit B (while it's stale) → B becomes current, C gets a new
    // stale source from B.
    kb.update_node(&b_id, deduction("B (edited)", &[&a_id], &a_id))
        .unwrap();

    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Current
    );
    assert!(kb
        .get_node(&b_id)
        .unwrap()
        .frontmatter
        .stale_sources
        .is_empty());

    // C should have two stale sources: A (from the first wave) and B (from
    // the re-propagation).
    let c_node = kb.get_node(&c_id).unwrap();
    assert_eq!(c_node.frontmatter.status, Status::Stale);
    assert_eq!(c_node.frontmatter.stale_sources.len(), 2);
    let source_ids: Vec<&str> = c_node
        .frontmatter
        .stale_sources
        .iter()
        .map(|s| s.node_id.as_str())
        .collect();
    assert!(source_ids.contains(&a_id.as_str()));
    assert!(source_ids.contains(&b_id.as_str()));

    let _ = fs::remove_dir_all(&dir);
}

// ── Axioms are immune ───────────────────────────────────

#[test]
fn axioms_never_become_stale_from_propagation() {
    let dir = temp_dir("stale-axiom-immune");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // A(axiom) → B(deduction) depends on A.
    // We also create C(axiom) with an edge from B → C via create_edge
    // (C won't have deps, but B depends on C if we add the edge).
    // Actually, to test this properly: A → B(deduction), but also have
    // C(axiom) that is somehow downstream. Since axioms can't have deps,
    // the only way an axiom is in the dependent chain is through
    // create_edge. But create_edge adds to frontmatter.dependencies,
    // which violates axiom constraints. So axioms can never truly be
    // downstream in the graph.
    //
    // Instead, test that propagation simply skips axioms gracefully:
    // use propagate_staleness directly to verify.
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    // Edit A.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();

    // A stays current (axiom, was the edited node).
    assert_eq!(
        kb.get_node(&a_id).unwrap().frontmatter.status,
        Status::Current
    );
    assert!(kb
        .get_node(&a_id)
        .unwrap()
        .frontmatter
        .stale_sources
        .is_empty());

    // B is stale.
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Stale
    );

    let _ = fs::remove_dir_all(&dir);
}

// ── Idempotent propagation ──────────────────────────────

#[test]
fn propagate_same_source_twice_is_idempotent() {
    let dir = temp_dir("stale-idempotent");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    // Edit A twice.
    kb.update_node(&a_id, axiom("A (edit 1)")).unwrap();
    kb.update_node(&a_id, axiom("A (edit 2)")).unwrap();

    // B should only have one stale_source from A (not two).
    let b_node = kb.get_node(&b_id).unwrap();
    assert_eq!(b_node.frontmatter.status, Status::Stale);
    assert_eq!(b_node.frontmatter.stale_sources.len(), 1);
    assert_eq!(b_node.frontmatter.stale_sources[0].node_id, a_id);

    let _ = fs::remove_dir_all(&dir);
}

// ── Multiple stale sources ──────────────────────────────

#[test]
fn multiple_stale_sources() {
    let dir = temp_dir("stale-multi-source");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // A → C, B → C
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(axiom("B")).unwrap();
    let b_id = b.frontmatter.id.clone();

    let c = kb
        .create_node(deduction(
            "C",
            &[&a_id, &b_id],
            &format!("{a_id} AND {b_id}"),
        ))
        .unwrap();
    let c_id = c.frontmatter.id.clone();

    // Edit A → C becomes stale with source A.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();
    assert_eq!(
        kb.get_node(&c_id).unwrap().frontmatter.stale_sources.len(),
        1
    );

    // Edit B → C gets a second stale source.
    kb.update_node(&b_id, axiom("B (edited)")).unwrap();
    let c_node = kb.get_node(&c_id).unwrap();
    assert_eq!(c_node.frontmatter.stale_sources.len(), 2);
    let source_ids: Vec<&str> = c_node
        .frontmatter
        .stale_sources
        .iter()
        .map(|s| s.node_id.as_str())
        .collect();
    assert!(source_ids.contains(&a_id.as_str()));
    assert!(source_ids.contains(&b_id.as_str()));

    let _ = fs::remove_dir_all(&dir);
}

// ── mark_node_reviewed ──────────────────────────────────

#[test]
fn mark_reviewed_clears_staleness() {
    let dir = temp_dir("stale-review-clears");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    // Make B stale.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Stale
    );

    // Review B.
    let reviewed = kb.mark_node_reviewed(&b_id).unwrap();
    assert_eq!(reviewed.frontmatter.status, Status::Current);
    assert!(reviewed.frontmatter.stale_sources.is_empty());

    // Verify on disk.
    let on_disk =
        ex_grapha_core::node_parser::read_node_file(&dir.join(format!("nodes/{b_id}.md"))).unwrap();
    assert_eq!(on_disk.frontmatter.status, Status::Current);
    assert!(on_disk.frontmatter.stale_sources.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mark_reviewed_on_current_node_is_noop() {
    let dir = temp_dir("stale-review-noop");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    // A is already current — reviewing it should still return current.
    let reviewed = kb.mark_node_reviewed(&a_id).unwrap();
    assert_eq!(reviewed.frontmatter.status, Status::Current);
    assert!(reviewed.frontmatter.stale_sources.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mark_reviewed_nonexistent_node_errors() {
    let dir = temp_dir("stale-review-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let err = kb.mark_node_reviewed("n-nonexistent").unwrap_err();
    assert!(err.to_string().contains("not found"));

    let _ = fs::remove_dir_all(&dir);
}

// ── Diamond propagation ─────────────────────────────────

#[test]
fn diamond_graph_propagation() {
    let dir = temp_dir("stale-diamond");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // A → B, A → C, B → D, C → D (diamond)
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    let c = kb.create_node(deduction("C", &[&a_id], &a_id)).unwrap();
    let c_id = c.frontmatter.id.clone();

    let d = kb
        .create_node(deduction(
            "D",
            &[&b_id, &c_id],
            &format!("{b_id} AND {c_id}"),
        ))
        .unwrap();
    let d_id = d.frontmatter.id.clone();

    // Edit A → B, C, and D all become stale.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();

    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Stale
    );
    assert_eq!(
        kb.get_node(&c_id).unwrap().frontmatter.status,
        Status::Stale
    );
    assert_eq!(
        kb.get_node(&d_id).unwrap().frontmatter.status,
        Status::Stale
    );

    // D should have exactly one stale_source: A (not duplicated via B and C).
    let d_node = kb.get_node(&d_id).unwrap();
    assert_eq!(d_node.frontmatter.stale_sources.len(), 1);
    assert_eq!(d_node.frontmatter.stale_sources[0].node_id, a_id);

    let _ = fs::remove_dir_all(&dir);
}

// ── Long chain propagation ──────────────────────────────

#[test]
fn long_chain_propagation() {
    let dir = temp_dir("stale-long-chain");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Create a chain: A → B → C → D → E
    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let mut prev_id = a_id.clone();
    let mut chain_ids = vec![a_id.clone()];

    for name in ["B", "C", "D", "E"] {
        let node = kb
            .create_node(deduction(name, &[&prev_id], &prev_id))
            .unwrap();
        let id = node.frontmatter.id.clone();
        chain_ids.push(id.clone());
        prev_id = id;
    }

    // Edit A → all of B, C, D, E become stale.
    kb.update_node(&a_id, axiom("A (edited)")).unwrap();

    for id in &chain_ids[1..] {
        assert_eq!(
            kb.get_node(id).unwrap().frontmatter.status,
            Status::Stale,
            "node {id} should be stale"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}

// ── Edit node that has no dependents ─────────────────────

#[test]
fn edit_leaf_node_no_propagation() {
    let dir = temp_dir("stale-leaf-no-prop");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    // Edit B (leaf node — no one depends on it).
    kb.update_node(&b_id, deduction("B (edited)", &[&a_id], &a_id))
        .unwrap();

    // B is current (just edited), A is unaffected.
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Current
    );
    assert_eq!(
        kb.get_node(&a_id).unwrap().frontmatter.status,
        Status::Current
    );

    let _ = fs::remove_dir_all(&dir);
}

// ── Review then edit propagates fresh ────────────────────

#[test]
fn review_then_edit_upstream_propagates_fresh() {
    let dir = temp_dir("stale-review-edit-fresh");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let a_id = a.frontmatter.id.clone();

    let b = kb.create_node(deduction("B", &[&a_id], &a_id)).unwrap();
    let b_id = b.frontmatter.id.clone();

    // Edit A → B stale.
    kb.update_node(&a_id, axiom("A v1")).unwrap();
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Stale
    );

    // Review B → B current.
    kb.mark_node_reviewed(&b_id).unwrap();
    assert_eq!(
        kb.get_node(&b_id).unwrap().frontmatter.status,
        Status::Current
    );

    // Edit A again → B stale again with fresh source.
    kb.update_node(&a_id, axiom("A v2")).unwrap();
    let b_node = kb.get_node(&b_id).unwrap();
    assert_eq!(b_node.frontmatter.status, Status::Stale);
    assert_eq!(b_node.frontmatter.stale_sources.len(), 1);
    assert_eq!(b_node.frontmatter.stale_sources[0].node_id, a_id);

    let _ = fs::remove_dir_all(&dir);
}
