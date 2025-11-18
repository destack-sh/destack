use std::path::{Path, PathBuf};

use crate::resolve::ResolveError;

/// Context for the resolver.
#[derive(Debug, Default, Clone)]
pub struct ResolveContext {
    /// Whether the specifier is fully specified.
    pub is_fully_specified: bool,
    /// Query `?query`, contains `?` (like `?foo` in `foo.js?foo`).
    pub query: Option<String>,
    /// Fragment `#query`, contains `#` (like `#foo` in `foo.js#foo`).
    pub fragment: Option<String>,
    /// Files that was found on file system.
    pub file_dependencies: Option<Vec<PathBuf>>,
    /// Dependencies that was not found on file system.
    pub missing_dependencies: Option<Vec<PathBuf>>,
    /// The current resolving alias for bailing recursion alias.
    pub resolving_alias: Option<String>,
    /// Current depth of the resolver.
    pub depth: u8,
    /// Maximum depth of the resolver.
    pub max_depth: u8 = 64,
}

impl ResolveContext {
    pub fn set_query_fragment(&mut self, query: Option<&str>, fragment: Option<&str>) {
        if let Some(query) = query {
            self.query.replace(query.to_string());
        }
        if let Some(fragment) = fragment {
            self.fragment.replace(fragment.to_string());
        }
    }

    pub fn add_file_dependency(&mut self, dep: &Path) {
        if let Some(deps) = &mut self.file_dependencies {
            deps.push(dep.to_path_buf());
        }
    }

    pub fn add_missing_dependency(&mut self, dep: &Path) {
        if let Some(deps) = &mut self.missing_dependencies {
            deps.push(dep.to_path_buf());
        }
    }

    /// Check the context's depth in order to detect recursion.
    pub fn check_depth(&mut self) -> Result<(), ResolveError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(ResolveError::Recursion);
        }
        Ok(())
    }
}
