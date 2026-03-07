mod common;

use ex_grapha_core::project::{init_project, open_project};
use std::fs;

/// Create a unique temp directory for each test.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir()
        .join("ex-grapha-tests")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ── init_project ───────────────────────────────────────────

#[test]
fn init_creates_directory_structure() {
    let dir = temp_dir("init-structure");
    let kb = init_project(&dir).unwrap();

    assert!(dir.join("nodes").is_dir());
    assert!(dir.join("assets").is_dir());
    assert!(dir.join(".knowledgebase/config.yaml").is_file());
    assert!(dir.join(".knowledgebase/hooks/validate.sh").is_file());
    assert!(dir.join(".github/workflows/validate.yaml").is_file());
    assert!(dir.join("README.md").is_file());
    assert!(dir.join(".gitignore").is_file());

    assert!(kb.nodes.is_empty());
    assert!(kb.dependents.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn init_writes_default_config() {
    let dir = temp_dir("init-config");
    let kb = init_project(&dir).unwrap();

    assert_eq!(kb.config.edge_annotations.len(), 5);
    assert_eq!(kb.config.edge_annotations[0].label, "supports");
    assert!(kb.config.display.edge_labels);
    assert_eq!(kb.config.tag_definitions.len(), 3);

    // Verify the file on disk round-trips
    let yaml = fs::read_to_string(dir.join(".knowledgebase/config.yaml")).unwrap();
    let parsed = ex_grapha_core::config::ProjectConfig::from_yaml(&yaml).unwrap();
    assert_eq!(parsed, kb.config);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn init_rejects_existing_project() {
    let dir = temp_dir("init-exists");
    init_project(&dir).unwrap();

    // Second init should fail
    let err = init_project(&dir).unwrap_err();
    assert!(err.to_string().contains("already contains"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn init_gitignore_contains_positions() {
    let dir = temp_dir("init-gitignore");
    init_project(&dir).unwrap();

    let content = fs::read_to_string(dir.join(".gitignore")).unwrap();
    assert!(content.contains(".positions.json"));

    let _ = fs::remove_dir_all(&dir);
}

// ── open_project ───────────────────────────────────────────

#[test]
fn open_empty_project() {
    let dir = temp_dir("open-empty");
    init_project(&dir).unwrap();

    let (kb, warnings) = open_project(&dir).unwrap();
    assert!(kb.nodes.is_empty());
    assert!(warnings.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_project_loads_nodes() {
    let dir = temp_dir("open-nodes");
    init_project(&dir).unwrap();

    // Write two node files
    fs::write(dir.join("nodes/n-4a7b2c.md"), common::AXIOM_FILE).unwrap();
    fs::write(dir.join("nodes/n-7c1d3e.md"), common::DEDUCTION_FILE).unwrap();

    let (kb, warnings) = open_project(&dir).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(kb.nodes.len(), 2);
    assert!(kb.nodes.contains_key("n-4a7b2c"));
    assert!(kb.nodes.contains_key("n-7c1d3e"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_project_builds_adjacency() {
    let dir = temp_dir("open-adjacency");
    init_project(&dir).unwrap();

    fs::write(dir.join("nodes/n-4a7b2c.md"), common::AXIOM_FILE).unwrap();
    fs::write(dir.join("nodes/n-7c1d3e.md"), common::DEDUCTION_FILE).unwrap();

    let (kb, _) = open_project(&dir).unwrap();

    // n-7c1d3e depends on n-4a7b2c and n-3f8a1d (the latter doesn't exist as a file,
    // but the adjacency map still records the relationship)
    let deps_of_4a7b2c = kb.dependents.get("n-4a7b2c").unwrap();
    assert!(deps_of_4a7b2c.contains("n-7c1d3e"));

    let deps_of_3f8a1d = kb.dependents.get("n-3f8a1d").unwrap();
    assert!(deps_of_3f8a1d.contains("n-7c1d3e"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_project_collects_warnings_for_malformed_files() {
    let dir = temp_dir("open-warnings");
    init_project(&dir).unwrap();

    // Write a valid node
    fs::write(dir.join("nodes/n-4a7b2c.md"), common::AXIOM_FILE).unwrap();
    // Write a malformed node
    fs::write(dir.join("nodes/n-bad000.md"), "not valid frontmatter").unwrap();

    let (kb, warnings) = open_project(&dir).unwrap();

    // The valid node was loaded
    assert_eq!(kb.nodes.len(), 1);
    assert!(kb.nodes.contains_key("n-4a7b2c"));

    // The malformed node produced a warning
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].path.ends_with("n-bad000.md"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_project_ignores_non_md_files() {
    let dir = temp_dir("open-non-md");
    init_project(&dir).unwrap();

    fs::write(dir.join("nodes/n-4a7b2c.md"), common::AXIOM_FILE).unwrap();
    fs::write(dir.join("nodes/notes.txt"), "just a text file").unwrap();

    let (kb, warnings) = open_project(&dir).unwrap();
    assert_eq!(kb.nodes.len(), 1);
    assert!(warnings.is_empty());

    let _ = fs::remove_dir_all(&dir);
}

// ── open_project error cases ───────────────────────────────

#[test]
fn open_missing_nodes_dir() {
    let dir = temp_dir("open-no-nodes");
    fs::create_dir_all(dir.join(".knowledgebase")).unwrap();
    fs::write(dir.join(".knowledgebase/config.yaml"), "edge_annotations: []\ndisplay:\n  edge_labels: true\n  edge_colors: true\n  relation_nodes: true\ntag_definitions: []\n").unwrap();
    // No nodes/ directory

    let err = open_project(&dir).unwrap_err();
    assert!(err.to_string().contains("nodes/"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_missing_config() {
    let dir = temp_dir("open-no-config");
    fs::create_dir_all(dir.join("nodes")).unwrap();
    // No .knowledgebase/config.yaml

    let err = open_project(&dir).unwrap_err();
    assert!(err.to_string().contains("config.yaml"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn open_nonexistent_path() {
    let dir = std::env::temp_dir()
        .join("ex-grapha-tests")
        .join("does-not-exist");
    let _ = fs::remove_dir_all(&dir);

    let err = open_project(&dir).unwrap_err();
    assert!(err.to_string().contains("cannot resolve path"));
}

// ── round-trip: init then open ─────────────────────────────

#[test]
fn init_then_open_round_trips() {
    let dir = temp_dir("init-then-open");
    let kb_init = init_project(&dir).unwrap();

    let (kb_open, warnings) = open_project(&dir).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(kb_init.config, kb_open.config);
    assert!(kb_open.nodes.is_empty());

    let _ = fs::remove_dir_all(&dir);
}
