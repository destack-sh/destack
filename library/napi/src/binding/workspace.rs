use std::path::PathBuf;

use napi_derive::napi;

use super::transpiler::TranspileOptions;

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
            inner: destack_workspace::Workspace::new(cwd),
        }
    }

    /// Add a root to the workspace.
    /// Creates a new Program for the given root path.
    #[napi]
    pub fn add_root(&self, root: String) -> napi::Result<()> {
        let root_path = PathBuf::from(root);
        self.inner.add_root(root_path);
        Ok(())
    }

    /// Get the current working directory.
    #[napi(getter)]
    pub fn cwd(&self) -> String {
        self.inner.cwd.to_string_lossy().to_string()
    }

    /// Get the number of programs in the workspace.
    #[napi(getter)]
    pub fn program_count(&self) -> u32 {
        self.inner.programs.len() as u32
    }

    /// Transpile a single file.
    /// Returns the transpiled TypeScript/JavaScript code.
    #[napi]
    pub fn transpile_file(
        &self,
        path: String,
        content: String,
        options: Option<TranspileOptions>,
    ) -> napi::Result<String> {
        // delegate to the stateless transpile function
        let result = super::transpiler::transpile_file(path, content, options)?;
        Ok(result.code)
    }
}

/// Get the default workspace options.
#[napi(js_name = "defaultWorkspaceOptions")]
pub fn default_workspace_options() -> WorkspaceOptions {
    WorkspaceOptions::default()
}
