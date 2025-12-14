use std::path::PathBuf;
use std::sync::Arc;

use crate::DsConfig;

/// Kind of workspace based on how it was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorkspaceKind {
    /// Monorepo with multiple packages (npm/pnpm workspaces).
    Monorepo,
    /// Single package workspace.
    #[default]
    SinglePackage,
}

/// A workspace is an organizational structure discovered from disk.
///
/// It represents a monorepo or single-package project, containing:
/// - The root directory of the workspace
/// - Workspace-wide configuration (from root `dsconfig.json`)
/// - Paths to packages within the workspace
///
/// This is distinct from `Session`, which is the runtime state for
/// daemon/LSP use cases with program caching and file watching.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// The root directory of the workspace.
    pub root: PathBuf,
    /// The kind of workspace (monorepo or single package).
    pub kind: WorkspaceKind,
    /// The workspace-wide dsconfig (from root `dsconfig.json`).
    /// Child packages inherit from this configuration.
    pub config: Option<Arc<DsConfig>>,
    /// Paths to packages within the workspace.
    /// For single-package workspaces, this is just the root.
    /// For monorepos, these are the package directories.
    pub package_paths: Vec<PathBuf>,
}

impl Workspace {
    /// Create a new single-package workspace.
    pub fn single_package(root: PathBuf) -> Self {
        Self {
            root: root.clone(),
            kind: WorkspaceKind::SinglePackage,
            config: None,
            package_paths: vec![root],
        }
    }

    /// Create a new monorepo workspace.
    pub fn monorepo(root: PathBuf, package_paths: Vec<PathBuf>) -> Self {
        Self {
            root,
            kind: WorkspaceKind::Monorepo,
            config: None,
            package_paths,
        }
    }

    /// Set the workspace-wide configuration.
    pub fn with_config(mut self, config: DsConfig) -> Self {
        self.config = Some(Arc::new(config));
        self
    }

    /// Check if this is a monorepo workspace.
    pub fn is_monorepo(&self) -> bool {
        self.kind == WorkspaceKind::Monorepo
    }

    /// Check if a path is within this workspace.
    pub fn contains_path(&self, path: &std::path::Path) -> bool {
        path.starts_with(&self.root)
    }

    /// Find the package path that contains the given path.
    pub fn find_package_for_path(&self, path: &std::path::Path) -> Option<&PathBuf> {
        self.package_paths
            .iter()
            .find(|pkg_path| path.starts_with(pkg_path))
    }
}
