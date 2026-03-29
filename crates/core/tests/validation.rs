use std::fs;

use ex_grapha_core::{
    model::{Dependency, NodeType},
    node::NodeParams,
    project::{init_project, open_project, InitOptions},
    validation::{Severity, ValidationRule},
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
fn deduction(title: &str, deps: Vec<Dependency>, relation: &str) -> NodeParams {
    NodeParams {
        title: title.into(),
        node_type: NodeType::Deduction,
        tags: vec![],
        dependencies: deps,
        relation: Some(relation.into()),
        content: format!("# {title}\n"),
    }
}

/// Build a `Dependency`.
fn dep(id: &str) -> Dependency {
    Dependency {
        node_id: id.into(),
    }
}

/// Write a raw `.md` file to the nodes directory.
fn write_node_file(dir: &std::path::Path, filename: &str, content: &str) {
    fs::write(dir.join("nodes").join(filename), content).unwrap();
}

// ── Happy path ───────────────────────────────────────────

#[test]
fn validate_clean_project() {
    let dir = temp_dir("val-clean");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("Axiom A")).unwrap();
    let b = kb.create_node(axiom("Axiom B")).unwrap();

    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    kb.create_node(deduction(
        "Deduction C",
        vec![dep(&a_id), dep(&b_id)],
        &format!("{a_id} AND {b_id}"),
    ))
    .unwrap();

    let report = kb.validate();
    assert!(report.is_valid());
    assert_eq!(report.error_count, 0);
    assert_eq!(report.warning_count, 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_empty_project() {
    let dir = temp_dir("val-empty");
    let kb = init_project(&dir, &DEFAULTS).unwrap();

    let report = kb.validate();
    assert!(report.is_valid());
    assert_eq!(report.issues.len(), 0);

    let _ = fs::remove_dir_all(&dir);
}

// ── Cycle detection ──────────────────────────────────────

#[test]
fn validate_detects_direct_cycle() {
    let dir = temp_dir("val-cycle-direct");
    init_project(&dir, &DEFAULTS).unwrap();

    // A depends on B, B depends on A → cycle
    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"Node A\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-bbb222\"
relation: \"n-bbb222\"
---
# Node A
",
    );
    write_node_file(
        &dir,
        "n-bbb222.md",
        "\
---
id: \"n-bbb222\"
title: \"Node B\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-aaa111\"
relation: \"n-aaa111\"
---
# Node B
",
    );

    let (kb, _warnings) = open_project(&dir).unwrap();
    let report = kb.validate();

    let cycle_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.rule == ValidationRule::NoCycles)
        .collect();
    assert!(!cycle_issues.is_empty(), "should detect the cycle");
    assert_eq!(cycle_issues[0].severity, Severity::Error);
    assert!(cycle_issues[0].context.is_some());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_detects_transitive_cycle() {
    let dir = temp_dir("val-cycle-transitive");
    init_project(&dir, &DEFAULTS).unwrap();

    // A→B, B→C, C→A
    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"A\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-bbb222\"
relation: \"n-bbb222\"
---
",
    );
    write_node_file(
        &dir,
        "n-bbb222.md",
        "\
---
id: \"n-bbb222\"
title: \"B\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-ccc333\"
relation: \"n-ccc333\"
---
",
    );
    write_node_file(
        &dir,
        "n-ccc333.md",
        "\
---
id: \"n-ccc333\"
title: \"C\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-aaa111\"
relation: \"n-aaa111\"
---
",
    );

    let (kb, _) = open_project(&dir).unwrap();
    let report = kb.validate();

    assert!(report
        .issues
        .iter()
        .any(|i| i.rule == ValidationRule::NoCycles));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_no_false_positive_on_diamond() {
    let dir = temp_dir("val-diamond");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Diamond: A, B both depend on nothing; C depends on A and B; D depends
    // on A and B. No cycle.
    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    kb.create_node(deduction(
        "C",
        vec![dep(&a_id), dep(&b_id)],
        &format!("{a_id} AND {b_id}"),
    ))
    .unwrap();
    kb.create_node(deduction(
        "D",
        vec![dep(&a_id), dep(&b_id)],
        &format!("{a_id} AND {b_id}"),
    ))
    .unwrap();

    let report = kb.validate();
    assert!(
        !report
            .issues
            .iter()
            .any(|i| i.rule == ValidationRule::NoCycles),
        "diamond is not a cycle"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_project_with_cyclic_data_does_not_panic() {
    let dir = temp_dir("val-cycle-safe-load");
    init_project(&dir, &DEFAULTS).unwrap();

    // Write cyclic data directly.
    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"A\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-bbb222\"
relation: \"n-bbb222\"
---
",
    );
    write_node_file(
        &dir,
        "n-bbb222.md",
        "\
---
id: \"n-bbb222\"
title: \"B\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-aaa111\"
relation: \"n-aaa111\"
---
",
    );

    // Must not panic (compute_dependencies is cycle-safe).
    let result = open_project(&dir);
    assert!(result.is_ok());

    let _ = fs::remove_dir_all(&dir);
}

// ── Dangling references ──────────────────────────────────

#[test]
fn validate_detects_dangling_reference() {
    let dir = temp_dir("val-dangling");
    init_project(&dir, &DEFAULTS).unwrap();

    // Deduction referencing a node that doesn't exist.
    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"Orphaned Deduction\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-missing\"
relation: \"n-missing\"
---
",
    );

    let (kb, _) = open_project(&dir).unwrap();
    let report = kb.validate();

    let dangling: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.rule == ValidationRule::NoDanglingReferences)
        .collect();
    assert_eq!(dangling.len(), 1);
    assert_eq!(dangling[0].severity, Severity::Error);
    assert!(dangling[0].message.contains("n-missing"));

    let _ = fs::remove_dir_all(&dir);
}

// ── Relation integrity ───────────────────────────────────

#[test]
fn validate_detects_unknown_operand_in_relation() {
    let dir = temp_dir("val-rel-unknown");
    init_project(&dir, &DEFAULTS).unwrap();

    // Deduction with relation referencing n-bbb222 but only depends on
    // n-aaa111.
    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"Axiom\"
type: axiom
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---
",
    );
    write_node_file(
        &dir,
        "n-ccc333.md",
        "\
---
id: \"n-ccc333\"
title: \"Bad relation\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-aaa111\"
relation: \"n-aaa111 AND n-bbb222\"
---
",
    );

    let (kb, _) = open_project(&dir).unwrap();
    let report = kb.validate();

    assert!(report
        .issues
        .iter()
        .any(|i| i.rule == ValidationRule::RelationIntegrity && i.message.contains("n-bbb222")));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_detects_missing_operand_in_relation() {
    let dir = temp_dir("val-rel-missing");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    let a = kb.create_node(axiom("A")).unwrap();
    let b = kb.create_node(axiom("B")).unwrap();
    let a_id = a.frontmatter.id.clone();
    let b_id = b.frontmatter.id.clone();

    // Deduction depends on both A and B, but relation only mentions A.
    kb.create_node(deduction("C", vec![dep(&a_id), dep(&b_id)], &a_id))
        .unwrap();

    let report = kb.validate();
    assert!(report
        .issues
        .iter()
        .any(|i| i.rule == ValidationRule::RelationIntegrity && i.message.contains(&b_id)));

    let _ = fs::remove_dir_all(&dir);
}

// ── Relation parsability ─────────────────────────────────

#[test]
fn validate_detects_unparseable_relation() {
    let dir = temp_dir("val-rel-syntax");
    init_project(&dir, &DEFAULTS).unwrap();

    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"Axiom\"
type: axiom
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---
",
    );
    write_node_file(
        &dir,
        "n-bbb222.md",
        "\
---
id: \"n-bbb222\"
title: \"Bad syntax\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-aaa111\"
relation: \"(n-aaa111\"
---
",
    );

    let (kb, _) = open_project(&dir).unwrap();
    let report = kb.validate();

    assert!(report
        .issues
        .iter()
        .any(|i| i.rule == ValidationRule::RelationParsability));

    let _ = fs::remove_dir_all(&dir);
}

// ── Type constraints ─────────────────────────────────────

#[test]
fn validate_detects_axiom_with_deps() {
    let dir = temp_dir("val-type-axiom-deps");
    init_project(&dir, &DEFAULTS).unwrap();

    // Axiom that incorrectly has a dependency.
    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"Bad Axiom\"
type: axiom
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-bbb222\"
---
",
    );

    let (kb, _) = open_project(&dir).unwrap();
    let report = kb.validate();

    assert!(report.issues.iter().any(|i| {
        i.rule == ValidationRule::TypeConstraints && i.message.contains("empty dependencies")
    }));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_detects_deduction_without_relation() {
    let dir = temp_dir("val-type-ded-no-rel");
    init_project(&dir, &DEFAULTS).unwrap();

    write_node_file(
        &dir,
        "n-aaa111.md",
        "\
---
id: \"n-aaa111\"
title: \"Axiom\"
type: axiom
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---
",
    );
    write_node_file(
        &dir,
        "n-bbb222.md",
        "\
---
id: \"n-bbb222\"
title: \"Bad Deduction\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-aaa111\"
---
",
    );

    let (kb, _) = open_project(&dir).unwrap();
    let report = kb.validate();

    assert!(report.issues.iter().any(|i| {
        i.rule == ValidationRule::TypeConstraints && i.message.contains("relation expression")
    }));

    let _ = fs::remove_dir_all(&dir);
}

// ── Duplicate IDs ────────────────────────────────────────

#[test]
fn validate_detects_duplicate_ids() {
    let dir = temp_dir("val-dup-ids");
    init_project(&dir, &DEFAULTS).unwrap();

    // Two files with the same ID.
    write_node_file(
        &dir,
        "node-a.md",
        "\
---
id: \"n-aaa111\"
title: \"First\"
type: axiom
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---
",
    );
    write_node_file(
        &dir,
        "node-b.md",
        "\
---
id: \"n-aaa111\"
title: \"Duplicate\"
type: axiom
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---
",
    );

    let (kb, warnings) = open_project(&dir).unwrap();

    // LoadWarning should be emitted.
    assert!(warnings.iter().any(|w| w.message.contains("duplicate")));

    // Validation should report it too.
    let report = kb.validate();
    assert!(report
        .issues
        .iter()
        .any(|i| i.rule == ValidationRule::NoDuplicateIds));

    let _ = fs::remove_dir_all(&dir);
}

// ── Tag warnings ────────────────────────────────────────

#[test]
fn validate_warns_on_undefined_tag() {
    let dir = temp_dir("val-undef-tag");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    kb.create_node(NodeParams {
        title: "Tagged".into(),
        node_type: NodeType::Axiom,
        tags: vec!["nonexistent-tag".into()],
        dependencies: vec![],
        relation: None,
        content: "# Tagged\n".into(),
    })
    .unwrap();

    let report = kb.validate();

    let tag_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.rule == ValidationRule::TagReference)
        .collect();
    assert_eq!(tag_issues.len(), 1);
    assert_eq!(tag_issues[0].severity, Severity::Warning);
    assert!(tag_issues[0].message.contains("nonexistent-tag"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_defined_tags_no_warning() {
    let dir = temp_dir("val-defined-tag");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // Default config includes "well-established", "tentative", "speculative".
    kb.create_node(NodeParams {
        title: "Tagged".into(),
        node_type: NodeType::Axiom,
        tags: vec!["well-established".into(), "tentative".into()],
        dependencies: vec![],
        relation: None,
        content: "# Tagged\n".into(),
    })
    .unwrap();

    let report = kb.validate();
    assert!(
        !report
            .issues
            .iter()
            .any(|i| i.rule == ValidationRule::TagReference),
        "defined tags should not trigger warnings"
    );

    let _ = fs::remove_dir_all(&dir);
}

// ── Multiple issues collected ────────────────────────────

#[test]
fn validate_collects_all_issues() {
    let dir = temp_dir("val-multi");
    let mut kb = init_project(&dir, &DEFAULTS).unwrap();

    // 1. Create an axiom with an undefined tag → warning
    kb.create_node(NodeParams {
        title: "Tagged Axiom".into(),
        node_type: NodeType::Axiom,
        tags: vec!["bad-tag".into()],
        dependencies: vec![],
        relation: None,
        content: "# Tagged\n".into(),
    })
    .unwrap();

    // 2. Write a node with a dangling reference → error
    write_node_file(
        &dir,
        "n-zzz000.md",
        "\
---
id: \"n-zzz000\"
title: \"Dangling\"
type: deduction
status: current
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-ghost\"
relation: \"n-ghost\"
---
",
    );

    // Re-open to pick up the hand-written file.
    let (kb, _) = open_project(&dir).unwrap();
    let report = kb.validate();

    // Should have at least: dangling ref (error) + undefined tag (warning).
    assert!(report.error_count >= 1);
    assert!(report.warning_count >= 1);
    assert!(!report.is_valid());
    assert!(report.issues.len() >= 2);

    let _ = fs::remove_dir_all(&dir);
}
