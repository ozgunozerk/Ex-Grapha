mod common;

use ex_grapha_core::error::Error;
use ex_grapha_core::frontmatter::*;
use ex_grapha_core::model::*;

// ── Parsing ────────────────────────────────────────────────

#[test]
fn parse_axiom_node() {
    let node = parse_node(common::AXIOM_FILE).unwrap();
    let fm = &node.frontmatter;

    assert_eq!(fm.id, "n-4a7b2c");
    assert_eq!(fm.title, "Conservation of Energy");
    assert_eq!(fm.node_type, NodeType::Axiom);
    assert_eq!(fm.tags, vec!["physics", "well-established"]);
    assert_eq!(fm.status, Status::Current);
    assert_eq!(fm.status_updated_at, "2026-03-04T14:30:00Z");
    assert_eq!(fm.status_updated_by, "github-username");
    assert!(fm.stale_sources.is_empty());
    assert_eq!(fm.created_at, "2026-02-15T10:00:00Z");
    assert_eq!(fm.created_by, "github-username");
    assert!(fm.dependencies.is_empty());
    assert!(fm.relation.is_none());

    assert!(node.content.contains("# Conservation of Energy"));
    assert!(node.content.contains("Energy cannot be created or destroyed"));
}

#[test]
fn parse_deduction_node() {
    let node = parse_node(common::DEDUCTION_FILE).unwrap();
    let fm = &node.frontmatter;

    assert_eq!(fm.id, "n-7c1d3e");
    assert_eq!(fm.node_type, NodeType::Deduction);
    assert_eq!(fm.status, Status::Stale);

    assert_eq!(fm.stale_sources.len(), 1);
    assert_eq!(fm.stale_sources[0].node_id, "n-4a7b2c");
    assert_eq!(fm.stale_sources[0].changed_at, "2026-03-04T14:30:00Z");

    assert_eq!(fm.dependencies.len(), 2);
    assert_eq!(fm.dependencies[0].node_id, "n-4a7b2c");
    assert_eq!(
        fm.dependencies[0].annotation.as_ref().unwrap().label,
        "requires"
    );
    assert_eq!(fm.dependencies[1].node_id, "n-3f8a1d");
    assert_eq!(
        fm.dependencies[1].annotation.as_ref().unwrap().label,
        "supports"
    );

    assert_eq!(fm.relation.as_deref(), Some("(n-4a7b2c AND n-3f8a1d)"));
    assert!(node.content.contains("# Orbital Mechanics"));
}

// ── Round-trip ─────────────────────────────────────────────

#[test]
fn round_trip_axiom() {
    let original = parse_node(common::AXIOM_FILE).unwrap();
    let serialized = serialize_node(&original).unwrap();
    let reparsed = parse_node(&serialized).unwrap();

    assert_eq!(original.frontmatter, reparsed.frontmatter);
    assert_eq!(original.content, reparsed.content);
}

#[test]
fn round_trip_deduction() {
    let original = parse_node(common::DEDUCTION_FILE).unwrap();
    let serialized = serialize_node(&original).unwrap();
    let reparsed = parse_node(&serialized).unwrap();

    assert_eq!(original.frontmatter, reparsed.frontmatter);
    assert_eq!(original.content, reparsed.content);
}

// ── Error cases ────────────────────────────────────────────

#[test]
fn missing_opening_delimiter() {
    let input = "id: foo\ntitle: bar\n";
    let err = parse_node(input).unwrap_err();
    assert!(matches!(err, Error::MissingFrontmatter));
}

#[test]
fn missing_closing_delimiter() {
    let input = "---\nid: foo\ntitle: bar\n";
    let err = parse_node(input).unwrap_err();
    assert!(matches!(err, Error::MissingFrontmatter));
}

#[test]
fn malformed_yaml() {
    let input = "---\n[invalid yaml: {{{\n---\n";
    let err = parse_node(input).unwrap_err();
    assert!(matches!(err, Error::Yaml(_)));
}

#[test]
fn missing_required_field() {
    let input = "\
---
id: \"n-000000\"
title: \"Test\"
type: \"axiom\"
tags: []
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---
Content here.
";
    let err = parse_node(input).unwrap_err();
    assert!(matches!(err, Error::Yaml(_)));
}

// ── Edge cases ─────────────────────────────────────────────

#[test]
fn empty_content() {
    let input = "\
---
id: \"n-000000\"
title: \"Empty\"
type: \"axiom\"
tags: []
status: \"current\"
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---
";
    let node = parse_node(input).unwrap();
    assert_eq!(node.content, "");
}

#[test]
fn content_with_triple_dashes() {
    let input = "\
---
id: \"n-000000\"
title: \"Dashes\"
type: \"axiom\"
tags: []
status: \"current\"
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies: []
---

# Heading

---

Some text after a horizontal rule.
";
    let node = parse_node(input).unwrap();
    assert!(node.content.contains("---"));
    assert!(node.content.contains("horizontal rule"));
}

#[test]
fn dependency_without_annotation() {
    let input = "\
---
id: \"n-aaaaaa\"
title: \"No Annotation\"
type: \"deduction\"
tags: []
status: \"current\"
status_updated_at: \"2026-01-01T00:00:00Z\"
status_updated_by: \"test\"
created_at: \"2026-01-01T00:00:00Z\"
created_by: \"test\"
dependencies:
  - node_id: \"n-bbbbbb\"
relation: \"n-bbbbbb\"
---
Body.
";
    let node = parse_node(input).unwrap();
    assert_eq!(node.frontmatter.dependencies.len(), 1);
    assert!(node.frontmatter.dependencies[0].annotation.is_none());
}

#[test]
fn bom_is_stripped() {
    let input = format!(
        "\u{feff}---\n\
         id: \"n-000000\"\n\
         title: \"BOM\"\n\
         type: \"axiom\"\n\
         tags: []\n\
         status: \"current\"\n\
         status_updated_at: \"2026-01-01T00:00:00Z\"\n\
         status_updated_by: \"test\"\n\
         created_at: \"2026-01-01T00:00:00Z\"\n\
         created_by: \"test\"\n\
         dependencies: []\n\
         ---\n\
         Content.\n"
    );
    let node = parse_node(&input).unwrap();
    assert_eq!(node.frontmatter.id, "n-000000");
}

// ── File I/O ───────────────────────────────────────────────

#[test]
fn read_write_node_file() {
    let dir = std::env::temp_dir().join("ex-grapha-test-rw");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("n-test01.md");

    let original = parse_node(common::AXIOM_FILE).unwrap();
    write_node_file(&path, &original).unwrap();
    let loaded = read_node_file(&path).unwrap();

    assert_eq!(original.frontmatter, loaded.frontmatter);
    assert_eq!(original.content, loaded.content);

    let _ = std::fs::remove_dir_all(&dir);
}
