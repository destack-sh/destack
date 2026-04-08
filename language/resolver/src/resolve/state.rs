use crate::{ResolveError, ResolveOptions};

/// The maximum recursive resolve depth for one branch.
const MAX_RESOLVE_DEPTH: u8 = 64;

/// The branch local execution state for one resolve flow.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResolveState {
    /// The current recursion depth.
    depth: u8,
    /// The rewrite currently being resolved for loop detection.
    active_rewrite: Option<String>,
    /// Whether the current branch is fully specified.
    is_fully_specified: bool,
}

impl ResolveState {
    /// Build the initial resolve state from one option set.
    pub(crate) fn new(options: &ResolveOptions) -> Self {
        Self {
            is_fully_specified: options.is_fully_specified,
            ..Self::default()
        }
    }

    /// Return one child state for a recursive resolve call.
    pub(crate) fn enter(&self) -> Result<Self, ResolveError> {
        if self.depth >= MAX_RESOLVE_DEPTH {
            return Err(ResolveError::RecursiveDependency { depth: self.depth });
        }

        Ok(Self {
            depth: self.depth + 1,
            active_rewrite: self.active_rewrite.clone(),
            is_fully_specified: self.is_fully_specified,
        })
    }

    /// Return the current recursion depth.
    pub(crate) fn depth(&self) -> u8 {
        self.depth
    }

    /// Return the currently active rewrite, when present.
    pub(crate) fn active_rewrite(&self) -> Option<&str> {
        self.active_rewrite.as_deref()
    }

    /// Return whether the current branch is fully specified.
    pub(crate) fn is_fully_specified(&self) -> bool {
        self.is_fully_specified
    }

    /// Return one state with a new active rewrite.
    pub(crate) fn with_active_rewrite(&self, active_rewrite: Option<String>) -> Self {
        Self {
            depth: self.depth,
            active_rewrite,
            is_fully_specified: self.is_fully_specified,
        }
    }

    /// Return one state with a new fully specified flag.
    pub(crate) fn with_fully_specified(&self, is_fully_specified: bool) -> Self {
        Self {
            depth: self.depth,
            active_rewrite: self.active_rewrite.clone(),
            is_fully_specified,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Entering recursion increments depth and preserves flags.
    #[test]
    fn test_enter_increments_depth() {
        let options = ResolveOptions::default().with_is_fully_specified(true);
        let state = ResolveState::new(&options).with_active_rewrite(Some("alias".to_string()));

        let state = state.enter().expect("expected child state");

        assert_eq!(state.depth(), 1);
        assert_eq!(state.active_rewrite(), Some("alias"));
        assert!(state.is_fully_specified());
    }

    /// Entering beyond the max depth returns an error.
    #[test]
    fn test_enter_rejects_excessive_depth() {
        let mut state = ResolveState::default();

        for _ in 0..MAX_RESOLVE_DEPTH {
            state = state.enter().expect("expected deeper state");
        }

        assert!(matches!(
            state.enter(),
            Err(ResolveError::RecursiveDependency {
                depth: MAX_RESOLVE_DEPTH
            })
        ));
    }
}
