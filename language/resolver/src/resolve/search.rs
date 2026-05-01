use crate::{ResolverError, ResolverOptions, ResolverResult};

/// The maximum recursive resolve depth for one resolver search.
const MAX_RESOLVE_DEPTH: u8 = 64;

/// The candidate probing mode for one resolver search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateMode {
    /// Probe exactly the requested path.
    Exact,
    /// Probe implicit file, extension, and directory candidates.
    Implicit,
}

/// The recursive controls for one resolver search.
#[derive(Debug, Clone)]
pub(crate) struct ResolverSearch {
    /// The current recursion depth.
    depth: u8,
    /// The candidate probing mode.
    candidate_mode: CandidateMode,
    /// The active specifier rewrites in this branch.
    rewrites: Vec<String>,
}

impl Default for ResolverSearch {
    fn default() -> Self {
        Self {
            depth: 0,
            candidate_mode: CandidateMode::Implicit,
            rewrites: Vec::new(),
        }
    }
}

impl ResolverSearch {
    /// Create the root resolver search from one option set.
    pub(crate) fn root(options: &ResolverOptions) -> Self {
        let candidate_mode = if options.is_fully_specified {
            CandidateMode::Exact
        } else {
            CandidateMode::Implicit
        };

        Self {
            candidate_mode,
            ..Self::default()
        }
    }

    /// Descend into one recursive resolver search.
    pub(crate) fn descend(&self) -> ResolverResult<Self> {
        if self.depth >= MAX_RESOLVE_DEPTH {
            return Err(ResolverError::RecursiveDependency { depth: self.depth });
        }

        Ok(Self {
            depth: self.depth + 1,
            candidate_mode: self.candidate_mode,
            rewrites: self.rewrites.clone(),
        })
    }

    /// Return true when implicit file and directory candidates are disabled.
    pub(crate) fn is_fully_specified(&self) -> bool {
        self.candidate_mode == CandidateMode::Exact
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
    fn enter_rewrite(&mut self, specifier: impl Into<String>) {
        self.rewrites.push(specifier.into());
        self.candidate_mode = CandidateMode::Implicit;
    }

    /// Enter a browser field rewrite search.
    pub(crate) fn enter_browser_rewrite(&mut self, specifier: impl Into<String>) {
        self.enter_rewrite(specifier);
    }

    /// Enter an alias rewrite search.
    pub(crate) fn enter_alias_rewrite(&mut self, specifier: impl Into<String>) {
        self.enter_rewrite(specifier);
    }

    /// Enter a package root search.
    pub(crate) fn enter_package_root(&mut self) {
        self.candidate_mode = CandidateMode::Implicit;
    }

    /// Enter an extension alias search.
    pub(crate) fn enter_extension_alias(&mut self) {
        self.candidate_mode = CandidateMode::Exact;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Descending into recursion increments depth and preserves flags.
    #[test]
    fn test_descend_increments_depth() {
        let options = ResolverOptions {
            is_fully_specified: true,
            ..ResolverOptions::default()
        };
        let search = ResolverSearch::root(&options);
        let mut search = search.descend().expect("expected child search");
        search.enter_browser_rewrite("alias");

        assert_eq!(search.depth, 1);
        assert!(search.has_rewrite("alias"));
        assert!(!search.is_fully_specified());
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
