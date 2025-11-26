use std::path::{Path, PathBuf};

use crate::resolve::ResolveError;

/// Mutable state passed through the resolution call chain.
/// - Track recursion depth to detect circular dependencies
/// - Collect parsed query/fragment from specifiers (e.g., `?foo#bar`)
/// - Maintain resolution flags that affect behavior at different stages
#[derive(Debug, Default, Clone)]
pub struct ResolutionContext {
    // resolution state
    /// Whether the current specifier has a fully-specified extension.
    /// When true, the resolver won't try adding extensions.
    pub is_fully_specified: bool,
    /// Alias currently being resolved, used to detect and bail on recursive aliases.
    pub resolving_alias: Option<String>,
    /// Query string from specifier (e.g., `?foo` from `module.js?foo`).
    pub query: Option<String>,
    /// Fragment from specifier (e.g., `#bar` from `module.js#bar`).
    pub fragment: Option<String>,

    // dependency tracking (optional, for build tools)
    /// Files found during resolution (only tracked if initialized to Some).
    pub found_dependencies: Option<Vec<PathBuf>>,
    /// Files not found during resolution (only tracked if initialized to Some).
    pub missing_dependencies: Option<Vec<PathBuf>>,

    // recursion
    /// Current recursion depth.
    pub depth: u8,
    /// Maximum allowed recursion depth.
    pub max_depth: u8 = 64,
}

impl ResolutionContext {
    /// Track a found dependency (if dependency tracking is enabled).
    pub fn track_found_dependency(&mut self, path: &Path) {
        if let Some(dependencies) = &mut self.found_dependencies {
            dependencies.push(path.to_path_buf());
        }
    }

    /// Track a missing dependency (if dependency tracking is enabled).
    pub fn track_missing_dependency(&mut self, path: &Path) {
        if let Some(dependencies) = &mut self.missing_dependencies {
            dependencies.push(path.to_path_buf());
        }
    }

    /// Increment depth and check for excessive recursion.
    pub fn check_depth(&mut self) -> Result<(), ResolveError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            Err(ResolveError::RecursiveDependency { depth: self.depth })
        } else {
            Ok(())
        }
    }
}
