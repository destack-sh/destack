use std::sync::Arc;

use tspp_artifact::SourceDependency;

/// Source observations invalidated by one repository edit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Delta {
    /// Exact preceding source observations that may have changed.
    invalidated: Arc<[SourceDependency]>,
    /// Possible package or module discovery change.
    discovery: Discovery,
}

impl Delta {
    /// Build one edit delta.
    pub(crate) fn new(
        invalidated: impl IntoIterator<Item = SourceDependency>,
        discovery: Discovery,
    ) -> Self {
        let mut invalidated = invalidated.into_iter().collect::<Vec<_>>();
        invalidated.sort_unstable();
        invalidated.dedup();

        Self {
            invalidated: invalidated.into(),
            discovery,
        }
    }

    /// Add invalidated source observations.
    pub(crate) fn extend(&mut self, invalidated: impl IntoIterator<Item = SourceDependency>) {
        let mut combined = self.invalidated.to_vec();
        combined.extend(invalidated);
        combined.sort_unstable();
        combined.dedup();
        self.invalidated = combined.into();
    }

    /// Return the invalidated source observations.
    pub(crate) fn invalidated(&self) -> &[SourceDependency] {
        &self.invalidated
    }

    /// Return the possible discovery change.
    pub(crate) fn discovery(&self) -> Discovery {
        self.discovery
    }
}

/// Possible package or module discovery change from one edit batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Discovery {
    /// No discovery input changed.
    #[default]
    None,
    /// File paths were added, removed, or moved.
    Paths,
    /// One package configuration changed.
    Config,
}

impl Discovery {
    /// Combine two discovery changes.
    pub(crate) fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Config, _) | (_, Self::Config) => Self::Config,
            (Self::Paths, _) | (_, Self::Paths) => Self::Paths,
            (Self::None, Self::None) => Self::None,
        }
    }
}
