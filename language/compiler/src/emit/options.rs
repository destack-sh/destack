/// Options for the emit phase.
#[derive(Debug, Clone, Default)]
pub struct EmitOptions {
    /// Whether to overwrite existing files.
    pub overwrite: bool,
    /// Whether to create parent directories if they don't exist.
    pub create_dirs: bool,
    /// Dry run: report what would be written without actually writing.
    pub dry_run: bool,
}
