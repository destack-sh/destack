use std::path::{Path, PathBuf};

use crate::resolve::ResolveError;

/// Scope guard that decrements resolve depth when dropped.
#[derive(Debug)]
pub struct ResolveDepthGuard<'a> {
    /// The active resolve context.
    context: &'a mut ResolveContext,
}

impl ResolveDepthGuard<'_> {
    /// Return a mutable reference to the guarded context.
    pub fn context(&mut self) -> &mut ResolveContext {
        self.context
    }
}

impl Drop for ResolveDepthGuard<'_> {
    fn drop(&mut self) {
        self.context.leave_depth();
    }
}

/// Mutable state passed through the resolution call chain.
/// - Track recursion depth to detect circular dependencies
/// - Collect parsed query/fragment from specifiers (e.g., `?foo#bar`)
/// - Maintain resolution flags that affect behavior at different stages
#[derive(Debug, Default, Clone)]
pub struct ResolveContext {
    /// Whether the current specifier already has a fully-specified extension.
    pub is_fully_specified: bool,
    /// Alias currently being resolved, used to detect and bail on recursive aliases.
    pub alias: Option<String>,
    /// Query string from specifier (e.g., `?foo` from `module.js?foo`).
    pub query: Option<String>,
    /// Fragment from specifier (e.g., `#bar` from `module.js#bar`).
    pub fragment: Option<String>,

    /// Files found during resolution (only tracked if initialized to Some).
    pub found_dependencies: Option<Vec<PathBuf>>,
    /// Files not found during resolution (only tracked if initialized to Some).
    pub missing_dependencies: Option<Vec<PathBuf>>,

    /// Current recursion depth.
    pub depth: u8,
    /// Maximum allowed recursion depth.
    pub max_depth: u8 = 64,
}

impl ResolveContext {
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

    /// Enter one resolve depth frame.
    pub fn enter_depth(&mut self) -> Result<(), ResolveError> {
        if self.depth >= self.max_depth {
            return Err(ResolveError::RecursiveDependency { depth: self.depth });
        }

        self.depth += 1;
        Ok(())
    }

    /// Leave one resolve depth frame.
    pub fn leave_depth(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    /// Enter resolve depth and return a scope guard.
    pub fn depth_guard(&mut self) -> Result<ResolveDepthGuard<'_>, ResolveError> {
        self.enter_depth()?;
        Ok(ResolveDepthGuard { context: self })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enter and leave depth through guard scope.
    #[test]
    fn test_depth_guard_increments_and_decrements() {
        let mut context = ResolveContext::default();
        assert_eq!(context.depth, 0);

        {
            let mut guard = context.depth_guard().expect("expected depth guard");
            assert_eq!(guard.context().depth, 1);
        }

        assert_eq!(context.depth, 0);
    }

    /// Keep depth unchanged when entering beyond max depth.
    #[test]
    fn test_enter_depth_errors_without_increment_at_max_depth() {
        let mut context = ResolveContext {
            depth: 2,
            max_depth: 2,
            ..ResolveContext::default()
        };

        let result = context.enter_depth();
        assert_eq!(result, Err(ResolveError::RecursiveDependency { depth: 2 }));
        assert_eq!(context.depth, 2);
    }

    /// Leave depth should not underflow.
    #[test]
    fn test_leave_depth_clamps_at_zero() {
        let mut context = ResolveContext::default();
        context.leave_depth();
        assert_eq!(context.depth, 0);
    }
}
