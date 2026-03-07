use ex_grapha_core::config::ProjectConfig;

#[test]
fn parse_config() {
    let yaml = "\
edge_annotations:
  - label: \"supports\"
    color: \"#22c55e\"
  - label: \"contradicts\"
    color: \"#ef4444\"
display:
  edge_labels: true
  edge_colors: true
  relation_nodes: true
tag_definitions:
  - name: \"well-established\"
  - name: \"tentative\"
";
    let config = ProjectConfig::from_yaml(yaml).unwrap();
    assert_eq!(config.edge_annotations.len(), 2);
    assert_eq!(config.edge_annotations[0].label, "supports");
    assert_eq!(config.edge_annotations[0].color, "#22c55e");
    assert!(config.display.edge_labels);
    assert_eq!(config.tag_definitions.len(), 2);
    assert_eq!(config.tag_definitions[0].name, "well-established");
}

#[test]
fn round_trip_config() {
    let yaml = "\
edge_annotations:
  - label: supports
    color: '#22c55e'
  - label: contradicts
    color: '#ef4444'
display:
  edge_labels: true
  edge_colors: true
  relation_nodes: true
tag_definitions:
  - name: well-established
  - name: tentative
";
    let original = ProjectConfig::from_yaml(yaml).unwrap();
    let serialized = original.to_yaml().unwrap();
    let reparsed = ProjectConfig::from_yaml(&serialized).unwrap();
    assert_eq!(original, reparsed);
}
