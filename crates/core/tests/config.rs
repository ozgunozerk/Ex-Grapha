use ex_grapha_core::config::ProjectConfig;

#[test]
fn parse_config() {
    let yaml = "\
display:
  relation_nodes: true
tag_definitions:
  - name: \"well-established\"
  - name: \"tentative\"
";
    let config = ProjectConfig::from_yaml(yaml).unwrap();
    assert!(config.display.relation_nodes);
    assert_eq!(config.tag_definitions.len(), 2);
    assert_eq!(config.tag_definitions[0].name, "well-established");
}

#[test]
fn round_trip_config() {
    let yaml = "\
display:
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
