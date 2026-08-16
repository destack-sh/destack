use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the checked decision for one pattern.
    pub(crate) fn pattern_decision(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<&dir::PatternDecision, ProviderError> {
        let pattern = pattern.into_global_any(self.id);
        self.decisions.pattern_decision(pattern).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked pattern {pattern:?} has no pattern decision"
            ))
        })
    }
}
