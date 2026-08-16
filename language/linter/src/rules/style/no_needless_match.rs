use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow matches that reconstruct their scrutinee unchanged.
    pub NO_NEEDLESS_MATCH {
        id: "no-needless-match",
        summary: "Disallow matches that reconstruct their scrutinee unchanged",
        explanation: r#"
A match whose every arm returns the exact value selected by its pattern leaves the scrutinee unchanged.
Instead, you SHOULD use the scrutinee directly.
"#,
        example: {
            reported: r#"
function preserve(value: int32): int32 {
    return match (value) {
        0 => 0
        1 => 1
        value => value
    };
}
"#,
            accepted: r#"
function preserve(value: int32): int32 {
    return value;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report matches whose arms preserve every selected value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect multi arm matches that preserve their result type
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { value, arms } = node else {
            continue;
        };
        if arms.len() < 2
            || module.adjusted_type_id(expression.into_any())?
                != module.adjusted_type_id(value.into_any())?
        {
            continue;
        }

        // require every unguarded expression arm to reconstruct its input
        let mut is_identity = true;
        for arm in arms {
            let dir::MatchArm::Expression {
                pattern,
                guard: None,
                body,
            } = view.get(*arm)
            else {
                is_identity = false;
                break;
            };
            if !arm_preserves_value(module, *pattern, *body)? {
                is_identity = false;
                break;
            }
        }
        if !is_identity {
            continue;
        }

        // replace the complete match when no arm comments are discarded
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("match returns its scrutinee unchanged", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, *value)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return whether one arm body reconstructs the value accepted by its pattern.
fn arm_preserves_value(
    module: &DirModule<'_>,
    pattern: dir::LocalNodeId<dir::Pattern>,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let view = module.view();

    // recognize direct bindings, scalar literals, and fieldless variants
    let is_preserved = match module.pattern_decision(pattern)? {
        dir::PatternDecision::Bind(binding) => match (binding.symbol, binding.pattern) {
            (Some(symbol), None) => module.selected_symbol(body)? == Some(symbol),
            _ => false,
        },
        dir::PatternDecision::Test(test) => {
            let dir::PredicateTest::Unary(test) = &test.predicate.test else {
                return Ok(false);
            };
            let dir::PredicateCondition::Literal(pattern_value) = &test.condition else {
                return Ok(false);
            };
            matches!(test.input, dir::PredicateOperand::Direct(_))
                && module.scalar_constant(body)?.as_ref() == Some(pattern_value)
        }
        dir::PatternDecision::Variant(variant) => {
            module.selected_symbol(body)? == Some(variant.case.variant)
        }
        _ => false,
    };
    if is_preserved {
        return Ok(true);
    }

    // recognize canonical Result variant reconstruction
    let dir::PatternDecision::Destructure(destructure) = module.pattern_decision(pattern)? else {
        return Ok(false);
    };
    let dir::PatternDestructureResolution::Nominal(nominal) = destructure.as_ref() else {
        return Ok(false);
    };
    let Some(variant) = module
        .dir
        .environment
        .language
        .item(nominal.selection.symbol)
    else {
        return Ok(false);
    };
    let constructor = match variant {
        dir::LanguageItem::Ok => dir::LanguageItem::Result.member("ok"),
        dir::LanguageItem::Err => dir::LanguageItem::Result.member("err"),
        _ => return Ok(false),
    };
    if module.language_member(body)? != Some(constructor) {
        return Ok(false);
    }

    // require one bound payload passed unchanged to the constructor
    let dir::Expression::Call { arguments, .. } = view.get(body) else {
        return Ok(false);
    };
    let [argument] = arguments.as_slice() else {
        return Ok(false);
    };
    let dir::Argument::Positional { value } = view.get(*argument) else {
        return Ok(false);
    };
    if nominal.fields.len() != 1 {
        return Ok(false);
    }
    let mut bindings = module.symbols_declared_within(pattern.into_any());
    let Some(binding) = bindings.next() else {
        return Ok(false);
    };
    if bindings.next().is_some() {
        return Ok(false);
    }

    Ok(module.selected_symbol(*value)? == Some(binding))
}

/// Build one direct scrutinee replacement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: destack_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(value.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // preserve authored grouping around the scrutinee
    let value = module.expression_source(value, dir::OperatorPrecedence::Lowest)?;
    let patch = Patch::replace(extent, value.into_owned());
    let suggestion = lint.fix("use the scrutinee directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an identity match over scalar values.
    #[test]
    fn test_replaces_scalar_identity() {
        let session = TestSession::dir(&NO_NEEDLESS_MATCH, NO_NEEDLESS_MATCH.example.reported());

        session.assert_fixes(NO_NEEDLESS_MATCH.example.accepted());
    }

    /// Replace canonical Result reconstruction.
    #[test]
    fn test_replaces_result_reconstruction() {
        let session = TestSession::dir(
            &NO_NEEDLESS_MATCH,
            r#"
function preserve(result: Result<int32, string>): Result<int32, string> {
    return match (result) {
        Ok { value } => Result.ok(value)
        Err { error } => Result.err(error)
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function preserve(result: Result<int32, string>): Result<int32, string> {
    return result;
}
"#,
        );
    }

    /// Accept a match that transforms one payload.
    #[test]
    fn test_accepts_transformed_payload() {
        let session = TestSession::dir(
            &NO_NEEDLESS_MATCH,
            r#"
function increment(result: Result<int32, string>): Result<int32, string> {
    return match (result) {
        Ok { value } => Result.ok(value + 1)
        Err { error } => Result.err(error)
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an identity shaped match with a guard.
    #[test]
    fn test_accepts_guarded_arm() {
        let session = TestSession::dir(
            &NO_NEEDLESS_MATCH,
            r#"
function preserve(value: int32): int32 {
    return match (value) {
        matched if (matched > 0) => matched
        matched => matched
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
