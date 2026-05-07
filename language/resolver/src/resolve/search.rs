use crate::{ResolverError, ResolverResult};

/// The maximum recursive resolve depth for one resolver search.
const MAX_RESOLVE_DEPTH: u8 = 64;

/// The recursive controls for one resolver search.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResolverSearch {
    /// The current recursion depth.
    depth: u8,
    /// The active specifier rewrites in this branch.
    rewrites: Vec<String>,
}

impl ResolverSearch {
    /// Create the root resolver search.
    pub(crate) fn root() -> Self {
        Self::default()
    }

    /// Descend into one recursive resolver search.
    pub(crate) fn descend(&self) -> ResolverResult<Self> {
        if self.depth >= MAX_RESOLVE_DEPTH {
            return Err(ResolverError::RecursiveDependency { depth: self.depth });
        }

        Ok(Self {
            depth: self.depth + 1,
            rewrites: self.rewrites.clone(),
        })
    }

    /// Return true when this rewrite is already active in the search branch.
    pub(crate) fn has_rewrite(&self, specifier: &str) -> bool {
        self.rewrites
            .iter()
            .any(|active_rewrite| active_rewrite == specifier)
    }

    /// Return a recursive dependency error at the current depth.
    pub(crate) fn recursive_dependency(&self) -> ResolverError {
        ResolverError::RecursiveDependency { depth: self.depth }
    }

    /// Enter a specifier rewrite search.
    pub(crate) fn enter_rewrite(&mut self, specifier: impl Into<String>) {
        self.rewrites.push(specifier.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Descending into recursion increments depth and preserves rewrites.
    #[test]
    fn test_descend_increments_depth() {
        let search = ResolverSearch::root();
        let mut search = search.descend().expect("expected child search");
        search.enter_rewrite("alias");

        assert_eq!(search.depth, 1);
        assert!(search.has_rewrite("alias"));
    }

    /// Descending beyond the max depth returns an error.
    #[test]
    fn test_descend_rejects_excessive_depth() {
        let mut search = ResolverSearch::default();

        for _ in 0..MAX_RESOLVE_DEPTH {
            search = search.descend().expect("expected deeper search");
        }

        assert!(matches!(
            search.descend(),
            Err(ResolverError::RecursiveDependency {
                depth: MAX_RESOLVE_DEPTH
            })
        ));
    }
}
