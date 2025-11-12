use dyst_source::FileRegistry;

/// Workspace for interacting with the language.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The files in the workspace.
    pub files: FileRegistry,
}