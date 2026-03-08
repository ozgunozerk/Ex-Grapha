//! Utility helpers: node ID generation, timestamps, OS username.

use rand::Rng;

use crate::model::NodeId;

/// Return the current OS username (e.g., "alice").
///
/// Used for `created_by` and `status_updated_by` fields in node frontmatter.
pub fn os_username() -> String {
    whoami::username()
}

/// Generate a random node ID in the format `n-XXXXXX` (6 hex chars).
pub fn generate_node_id() -> NodeId {
    let mut rng = rand::thread_rng();
    let hex: u32 = rng.gen_range(0..0x1000000); // 24 bits = 6 hex digits
    format!("n-{hex:06x}")
}

/// Return the current UTC time as an ISO 8601 string (e.g.,
/// "2026-03-08T12:00:00Z").
pub fn now_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
