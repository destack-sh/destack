/// Options for the emit phase.
#[derive(Debug, Clone)]
pub struct EmitOptions {
    /// Whether to overwrite existing files. Default: true.
    pub overwrite: bool,
    /// Whether to create parent directories if they don't exist. Default: true.
    pub create_dirs: bool,
    /// Dry run: report what would be written without actually writing. Default: false.
    pub dry_run: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            overwrite: true,
            create_dirs: true,
            dry_run: false,
        }
    }
}
