/// Return the current OS username (e.g., "alice").
///
/// Used for `created_by` and `status_updated_by` fields in node frontmatter.
pub fn os_username() -> String {
    whoami::username()
}
