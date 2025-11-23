use std::path::{Path, PathBuf};

use crate::resolve::ResolveError;

/// Context for the resolver.
#[derive(Debug, Default, Clone)]
pub struct ResolutionContext {
    /// Whether the specifier is fully specified.
    pub is_fully_specified: bool,
    /// Query `?query`, contains `?` (like `?foo` in `foo.js?foo`).
    pub query: Option<String>,
    /// Fragment `#query`, contains `#` (like `#foo` in `foo.js#foo`).
    pub fragment: Option<String>,

    /// Files that we have found on file system in this resolution (only tracked if not none).
    pub found_dependencies: Option<Vec<PathBuf>>,
    /// Files that we have not found on file system in this resolution (only tracked if not none).
    pub missing_dependencies: Option<Vec<PathBuf>>,
    
    /// The current resolving alias for bailing recursion alias.
    pub resolving_alias: Option<String>,

    /// Current depth of the resolution.
    pub depth: u8,
    /// Maximum depth of the resolution.
    pub max_depth: u8 = 64,
}

impl ResolutionContext {
    /// Adds a file to the found dependencies (if we're tracking them).
    pub fn add_found_dependency_maybe(&mut self, dep: &Path) {
        if let Some(deps) = &mut self.found_dependencies {
            deps.push(dep.to_path_buf());
        }
    }

    /// Adds a file to the missing dependencies (if we're tracking them).
    pub fn add_missing_dependency_maybe(&mut self, dep: &Path) {
        if let Some(deps) = &mut self.missing_dependencies {
            deps.push(dep.to_path_buf());
        }
    }

    /// Check the context's depth in order to detect recursion.
    pub fn check_depth(&mut self) -> Result<(), ResolveError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(ResolveError::RecursiveDependency { depth: self.depth });
        }
        Ok(())
    }
}
