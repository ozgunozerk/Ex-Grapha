//! Parse and serialize node `.md` files (YAML frontmatter + markdown body).

use std::path::Path;

use crate::{
    error::Error,
    model::{Node, NodeFrontmatter},
};

/// Parse a node `.md` file (YAML frontmatter + markdown body) into a `Node`.
pub fn parse_node(input: &str) -> Result<Node, Error> {
    // Strip UTF-8 BOM if present
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);

    if !input.starts_with("---\n") && !input.starts_with("---\r\n") {
        return Err(Error::MissingFrontmatter);
    }

    // Skip past the opening "---\n"
    let after_opening = if let Some(rest) = input.strip_prefix("---\r\n") {
        rest
    } else if let Some(rest) = input.strip_prefix("---\n") {
        rest
    } else {
        unreachable!() // already checked above
    };

    // Find the closing "---" that sits on its own line.
    let closing_pos = find_closing_delimiter(after_opening)?;

    let yaml_str = &after_opening[..closing_pos];

    // Content starts after the closing "---" and its line ending.
    let after_dashes = &after_opening[closing_pos + 3..];
    let content = after_dashes
        .strip_prefix("\r\n")
        .or_else(|| after_dashes.strip_prefix('\n'))
        .unwrap_or(after_dashes);

    let frontmatter: NodeFrontmatter = serde_yaml::from_str(yaml_str)?;

    Ok(Node {
        frontmatter,
        content: content.to_string(),
    })
}

/// Serialize a `Node` back to the frontmatter + markdown file format.
pub fn serialize_node(node: &Node) -> Result<String, Error> {
    let yaml = serde_yaml::to_string(&node.frontmatter)?;
    // serde_yaml output ends with '\n', so the layout becomes:
    //   ---\n{yaml}---\n{content}
    Ok(format!("---\n{}---\n{}", yaml, node.content))
}

/// Read and parse a node file from disk.
pub fn read_node_file(path: &Path) -> Result<Node, Error> {
    let input = std::fs::read_to_string(path)?;
    parse_node(&input)
}

/// Serialize a node and write it to disk.
pub fn write_node_file(path: &Path, node: &Node) -> Result<(), Error> {
    let output = serialize_node(node)?;
    std::fs::write(path, output)?;
    Ok(())
}

/// Find the byte offset of the closing `---` delimiter within `haystack`.
///
/// The delimiter must appear at the start of a line (i.e., preceded by `\n`
/// or at position 0) and be followed by `\n`, `\r\n`, or end-of-string.
fn find_closing_delimiter(haystack: &str) -> Result<usize, Error> {
    let bytes = haystack.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        // The delimiter must appear at position 0 or right after a newline.
        let candidate = if pos == 0 {
            bytes.starts_with(b"---")
        } else if bytes[pos - 1] == b'\n' {
            bytes[pos..].starts_with(b"---")
        } else {
            false
        };

        if candidate {
            let end = pos + 3;
            // Must be followed by newline, CRLF, or EOF.
            if end == bytes.len()
                || bytes[end] == b'\n'
                || (bytes[end] == b'\r' && bytes.get(end + 1) == Some(&b'\n'))
            {
                return Ok(pos);
            }
        }

        // Advance to the next line.
        match memchr_newline(bytes, pos) {
            Some(nl) => pos = nl + 1,
            None => break,
        }
    }

    Err(Error::MissingFrontmatter)
}

/// Find the next `\n` byte starting from `from`.
fn memchr_newline(bytes: &[u8], from: usize) -> Option<usize> {
    bytes[from..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|i| from + i)
}
