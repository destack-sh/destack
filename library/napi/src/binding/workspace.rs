use std::path::PathBuf;

use napi_derive::napi;

/// Options for creating a Workspace.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WorkspaceOptions {
    /// The current working directory.
    pub cwd: String,
}

impl Default for WorkspaceOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        }
    }
}

/// A workspace containing multiple program roots.
#[napi]
#[derive(Debug)]
pub struct Workspace {
    inner: destack_workspace::Workspace,
}

#[napi]
impl Workspace {
    /// Create a new Workspace.
    #[napi(constructor)]
    pub fn new(options: WorkspaceOptions) -> Self {
        let cwd = PathBuf::from(options.cwd);
        Self {
            inner: destack_workspace::Workspace::single_package(cwd),
        }
    }
}

/// Get the default workspace options.
#[napi(js_name = "defaultWorkspaceOptions")]
pub fn default_workspace_options() -> WorkspaceOptions {
    WorkspaceOptions::default()
}
