mod pattern;
mod predicate;
mod rewrite;

pub use pattern::*;
pub use predicate::*;
pub use rewrite::*;

#[cfg(test)]
mod tests {
    use tspp_source::DiagnosticRegistry;

    use super::{PatternError, PredicateError, RewriteError};

    /// Register every pattern diagnostic under one unique stable id.
    #[test]
    fn test_register_diagnostic_ids() {
        let definitions = PatternError::ALL
            .iter()
            .chain(PredicateError::ALL)
            .chain(RewriteError::ALL)
            .collect::<Vec<_>>();
        let ids = definitions
            .iter()
            .map(|definition| definition.id)
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            [
                "expected-pattern-root",
                "expected-pattern-context-node",
                "internal-pattern-error",
                "invalid-pattern-metavariable",
                "invalid-repeated-metavariable",
                "incompatible-pattern-metavariable",
                "ambiguous-repeated-metavariables",
                "expected-predicate-root",
                "internal-predicate-error",
                "repeated-predicate-metavariable",
                "unbound-predicate-metavariable",
                "invalid-predicate-metavariable",
                "unsupported-predicate-expression",
                "invalid-predicate-operands",
                "expected-replacement-root",
                "expected-replacement-context-node",
                "invalid-replacement-metavariable",
                "invalid-repeated-replacement-metavariable",
                "ambiguous-repeated-replacement-metavariables",
                "anonymous-replacement-metavariable",
                "unbound-replacement-metavariable",
                "incompatible-replacement-metavariable",
                "internal-rewrite-error",
                "overlapping-rewrites",
            ]
        );

        // construction rejects invalid, duplicate, and colliding ids
        let registry = DiagnosticRegistry::new(
            definitions
                .iter()
                .map(|definition| (definition.id, definition.is_controllable)),
        );
        for definition in definitions {
            assert_eq!(
                registry.is_controllable(definition.id),
                Some(definition.is_controllable)
            );
        }
    }
}
