use std::path::{Path, PathBuf};

use crate::{ResolveError, ResolveTrace};

/// Scope guard that decrements resolve depth when dropped.
#[derive(Debug)]
pub(crate) struct ResolveDepthGuard<'a> {
    /// The active resolve frame.
    frame: &'a mut ResolveFrame,
}

impl ResolveDepthGuard<'_> {
    /// Return a mutable reference to the guarded frame.
    pub(crate) fn frame(&mut self) -> &mut ResolveFrame {
        self.frame
    }
}

impl Drop for ResolveDepthGuard<'_> {
    fn drop(&mut self) {
        self.frame.leave_depth();
    }
}

/// The mutable scratch state for one resolve call chain.
#[derive(Debug, Default, Clone)]
pub(crate) struct ResolveFrame {
    /// Whether the current specifier already has a fully specified extension.
    pub is_fully_specified: bool,
    /// The rewrite currently being resolved for loop detection.
    pub active_rewrite: Option<String>,

    /// The found dependencies when tracing is enabled.
    pub found_dependencies: Option<Vec<PathBuf>>,
    /// The missing dependencies when tracing is enabled.
    pub missing_dependencies: Option<Vec<PathBuf>>,

    /// Current recursion depth.
    pub depth: u8,
    /// Maximum allowed recursion depth.
    pub max_depth: u8 = 64,
}

impl ResolveFrame {
    /// Create one frame that records dependency tracing.
    pub(crate) fn with_trace() -> Self {
        Self {
            found_dependencies: Some(Vec::new()),
            missing_dependencies: Some(Vec::new()),
            ..Self::default()
        }
    }

    /// Append any recorded dependency tracing into the given public trace.
    pub(crate) fn append_trace_to(&mut self, trace: &mut ResolveTrace) {
        if let Some(found_dependencies) = &mut self.found_dependencies {
            trace.found_dependencies.append(found_dependencies);
        }

        if let Some(missing_dependencies) = &mut self.missing_dependencies {
            trace.missing_dependencies.append(missing_dependencies);
        }
    }

    /// Track one found dependency when dependency tracing is enabled.
    pub(crate) fn track_found_dependency(&mut self, path: &Path) {
        if let Some(dependencies) = &mut self.found_dependencies {
            dependencies.push(path.to_path_buf());
        }
    }

    /// Track one missing dependency when dependency tracing is enabled.
    pub(crate) fn track_missing_dependency(&mut self, path: &Path) {
        if let Some(dependencies) = &mut self.missing_dependencies {
            dependencies.push(path.to_path_buf());
        }
    }

    /// Enter one resolve depth frame.
    pub(crate) fn enter_depth(&mut self) -> Result<(), ResolveError> {
        if self.depth >= self.max_depth {
            return Err(ResolveError::RecursiveDependency { depth: self.depth });
        }

        self.depth += 1;
        Ok(())
    }

    /// Leave one resolve depth frame.
    pub(crate) fn leave_depth(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    /// Enter resolve depth and return a scope guard.
    pub(crate) fn depth_guard(&mut self) -> Result<ResolveDepthGuard<'_>, ResolveError> {
        self.enter_depth()?;
        Ok(ResolveDepthGuard { frame: self })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Enter and leave depth through guard scope.
    #[test]
    fn test_depth_guard_increments_and_decrements() {
        let mut frame = ResolveFrame::default();
        assert_eq!(frame.depth, 0);

        {
            let mut guard = frame.depth_guard().expect("expected depth guard");
            assert_eq!(guard.frame().depth, 1);
        }

        assert_eq!(frame.depth, 0);
    }

    /// Keep depth unchanged when entering beyond max depth.
    #[test]
    fn test_enter_depth_errors_without_increment_at_max_depth() {
        let mut frame = ResolveFrame {
            depth: 2,
            max_depth: 2,
            ..ResolveFrame::default()
        };

        let result = frame.enter_depth();
        assert_eq!(result, Err(ResolveError::RecursiveDependency { depth: 2 }));
        assert_eq!(frame.depth, 2);
    }

    /// Leave depth should not underflow.
    #[test]
    fn test_leave_depth_clamps_at_zero() {
        let mut frame = ResolveFrame::default();
        frame.leave_depth();
        assert_eq!(frame.depth, 0);
    }
}
