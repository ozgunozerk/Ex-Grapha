mod common;

use ex_grapha_core::{frontmatter::parse_node, model::*};

#[test]
fn axiom_validates_ok() {
    let node = parse_node(common::AXIOM_FILE).unwrap();
    assert!(node.frontmatter.validate_type_constraints().is_ok());
}

#[test]
fn deduction_validates_ok() {
    let node = parse_node(common::DEDUCTION_FILE).unwrap();
    assert!(node.frontmatter.validate_type_constraints().is_ok());
}

#[test]
fn axiom_with_deps_fails_validation() {
    let mut node = parse_node(common::AXIOM_FILE).unwrap();
    node.frontmatter.dependencies.push(Dependency {
        node_id: "n-aaaaaa".into(),
        annotation: None,
    });
    let err = node.frontmatter.validate_type_constraints().unwrap_err();
    assert!(err.to_string().contains("empty dependencies"));
}

#[test]
fn axiom_with_relation_fails_validation() {
    let mut node = parse_node(common::AXIOM_FILE).unwrap();
    node.frontmatter.relation = Some("n-aaaaaa".into());
    let err = node.frontmatter.validate_type_constraints().unwrap_err();
    assert!(err.to_string().contains("relation expression"));
}

#[test]
fn axiom_with_stale_sources_fails_validation() {
    let mut node = parse_node(common::AXIOM_FILE).unwrap();
    node.frontmatter.stale_sources.push(StaleSource {
        node_id: "n-aaaaaa".into(),
        changed_at: "2026-01-01T00:00:00Z".into(),
    });
    let err = node.frontmatter.validate_type_constraints().unwrap_err();
    assert!(err.to_string().contains("stale_sources"));
}

#[test]
fn deduction_without_deps_fails_validation() {
    let mut node = parse_node(common::DEDUCTION_FILE).unwrap();
    node.frontmatter.dependencies.clear();
    let err = node.frontmatter.validate_type_constraints().unwrap_err();
    assert!(err.to_string().contains("at least one dependency"));
}

#[test]
fn deduction_without_relation_fails_validation() {
    let mut node = parse_node(common::DEDUCTION_FILE).unwrap();
    node.frontmatter.relation = None;
    let err = node.frontmatter.validate_type_constraints().unwrap_err();
    assert!(err.to_string().contains("relation expression"));
}
