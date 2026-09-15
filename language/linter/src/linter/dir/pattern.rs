use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the decision for one pattern.
    pub(crate) fn pattern_decision(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<&dir::PatternDecision, ProviderError> {
        let pattern = pattern.into_global_any(self.id);
        self.decisions.pattern_decision(pattern).ok_or_else(|| {
            ProviderError::internal(format!("pattern {pattern:?} has no pattern decision"))
        })
    }

    /// Return the canonical language item selected by one pattern.
    pub(crate) fn pattern_language_item(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<Option<dir::LanguageItem>, ProviderError> {
        let symbol = match self.pattern_decision(pattern)? {
            // fieldless nominal variant
            dir::PatternDecision::Variant(variant) => variant.case.variant,

            // nominal destructuring
            dir::PatternDecision::Destructure(destructure) => match destructure.as_ref() {
                dir::PatternDestructureResolution::Nominal(nominal) => nominal.key.symbol,
                _ => return Ok(None),
            },

            // newtype payload projection
            dir::PatternDecision::Project(project) => match &project.projection {
                dir::OperationResolution::One(dir::Projection::NewtypePayload { key, .. }) => {
                    key.symbol
                }
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        Ok(self.dir.environment.language.item(symbol))
    }

    /// Return the scalar literal tested directly by one pattern.
    pub(crate) fn pattern_literal(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<Option<dir::Literal>, ProviderError> {
        let dir::PatternDecision::Test(test) = self.pattern_decision(pattern)? else {
            return Ok(None);
        };
        let dir::PredicateTest::Unary(test) = &test.predicate.test else {
            return Ok(None);
        };
        let dir::PredicateCondition::Literal(literal) = &test.condition else {
            return Ok(None);
        };
        if !matches!(test.input, dir::PredicateOperand::Direct(_)) {
            return Ok(None);
        }

        Ok(Some(*literal))
    }

    /// Return the sole pattern binding selected directly by one expression.
    pub(crate) fn selected_pattern_binding(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::GlobalSymbolId>, ProviderError> {
        // select the pattern's only declared binding
        let Some(binding) = self.sole_declared_symbol(pattern.into_any()) else {
            return Ok(None);
        };

        // require the expression to select the declared binding directly
        if self.selected_symbol(expression)? != Some(binding) {
            return Ok(None);
        }

        Ok(Some(binding))
    }
}
