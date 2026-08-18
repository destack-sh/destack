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

    /// Return the canonical language item selected by one checked pattern.
    pub(crate) fn pattern_language_item(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<Option<dir::LanguageItem>, ProviderError> {
        let symbol = match self.pattern_decision(pattern)? {
            // fieldless nominal variant
            dir::PatternDecision::Variant(variant) => variant.case.variant,

            // nominal destructuring
            dir::PatternDecision::Destructure(destructure) => match destructure.as_ref() {
                dir::PatternDestructureResolution::Nominal(nominal) => nominal.selection.symbol,
                _ => return Ok(None),
            },

            // newtype payload projection
            dir::PatternDecision::Project(project) => match &project.projection {
                dir::OperationResolution::One(dir::Projection::NewtypePayload {
                    selection,
                    ..
                }) => selection.symbol,
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        Ok(self.dir.environment.language.item(symbol))
    }
}
