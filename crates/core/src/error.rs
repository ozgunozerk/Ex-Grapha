/// Errors produced by the core library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid node file: missing or malformed frontmatter delimiters")]
    MissingFrontmatter,

    #[error("failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("type constraint violation: {0}")]
    TypeConstraint(String),

    #[error("invalid project: {0}")]
    InvalidProject(String),
}
