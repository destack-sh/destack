use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer equality over matching one complete scalar constant.
    pub PREFER_EQUALITY_OVER_PATTERN {
        id: "prefer-equality-over-pattern",
        summary: "Prefer equality over matching one complete scalar constant",
        explanation: r#"
A two-arm match that maps one scalar constant and a wildcard to opposite booleans only performs an equality test.
Instead, you SHOULD compare the scrutinee with the constant directly.
"#,
        example: {
            reported: r#"
function isZero(value: int32): boolean {
    return match (value) {
        0 => true
        _ => false
    };
}
"#,
            accepted: r#"
function isZero(value: int32): boolean {
    return value === 0;
}
"#,
        },
        provenance: [],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report boolean matches over one scalar constant and one wildcard.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect exhaustive two arm matches
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Match { value, arms } = node else {
            continue;
        };
        let [first, second] = arms.as_slice() else {
            continue;
        };
        let Some(first) = BooleanArm::from_match_arm(module, *first)? else {
            continue;
        };
        let Some(second) = BooleanArm::from_match_arm(module, *second)? else {
            continue;
        };

        // require one constant followed by a wildcard with the opposite result
        let (constant, is_equal) = match (first.pattern, second.pattern) {
            (BooleanPattern::Constant(constant), BooleanPattern::Wildcard)
                if first.result != second.result =>
            {
                (constant, first.result)
            }
            _ => continue,
        };

        // replace the complete match when every omitted comment is preserved
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("match only compares one scalar constant", extent);
        if let Some(suggestion) = suggestion(module, lint, extent, *value, constant, is_equal)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// One boolean arm pattern relevant to direct equality.
#[derive(Debug, Clone, Copy)]
enum BooleanPattern {
    /// One complete scalar constant.
    Constant(dir::LocalNodeId<dir::Expression>),
    /// The exhaustive wildcard.
    Wildcard,
}

/// One unguarded match arm returning a boolean literal.
#[derive(Debug, Clone, Copy)]
struct BooleanArm {
    /// The selected pattern.
    pattern: BooleanPattern,
    /// The returned boolean.
    result: bool,
}

impl BooleanArm {
    /// Return one unguarded boolean arm with a constant or wildcard pattern.
    fn from_match_arm(
        module: &DirModule<'_>,
        arm: dir::LocalNodeId<dir::MatchArm>,
    ) -> Result<Option<Self>, ProviderError> {
        let view = module.view();
        let Some((pattern, body)) = module.match_arm_value(arm) else {
            return Ok(None);
        };
        let Some(result) = view.get(body).as_boolean() else {
            return Ok(None);
        };

        // select one exact scalar test or exhaustive wildcard
        let pattern = match (view.get(pattern), module.pattern_decision(pattern)?) {
            (dir::Pattern::Wildcard, dir::PatternDecision::Ignore) => BooleanPattern::Wildcard,
            (dir::Pattern::Expression { value }, dir::PatternDecision::Test(_))
                if module.scalar_constant(*value)?.is_some() =>
            {
                BooleanPattern::Constant(*value)
            }
            _ => return Ok(None),
        };

        Ok(Some(Self { pattern, result }))
    }
}

/// Build the direct equality comparison.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    value: dir::LocalNodeId<dir::Expression>,
    constant: dir::LocalNodeId<dir::Expression>,
    is_equal: bool,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let value_span = module.source_extent(value.into_any())?;
    let constant_span = module.source_extent(constant.into_any())?;
    if module.has_unretained_comment(extent, &[value_span, constant_span])? {
        return Ok(None);
    }

    // retain both operands with comparison-safe grouping
    let value = module.expression_source(value, dir::OperatorPrecedence::Equality)?;
    let constant = module.expression_source(constant, dir::OperatorPrecedence::Equality)?;
    let operator = if is_equal { "===" } else { "!==" };
    let patch = Patch::replace(extent, format!("{value} {operator} {constant}"));
    let suggestion = lint.fix("compare the scalar constant directly", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a negative scalar constant match with inequality.
    #[test]
    fn test_replaces_negative_match() {
        let session = TestSession::dir(
            &PREFER_EQUALITY_OVER_PATTERN,
            r#"
function isNonZero(value: int32): boolean {
    return match (value) {
        0 => false
        _ => true
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function isNonZero(value: int32): boolean {
    return value !== 0;
}
"#,
        );
    }

    /// Replace a negative literal pattern with equality.
    #[test]
    fn test_replaces_negative_literal() {
        let session = TestSession::dir(
            &PREFER_EQUALITY_OVER_PATTERN,
            r#"
function isNegativeOne(value: int32): boolean {
    return match (value) {
        -1 => true
        _ => false
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function isNegativeOne(value: int32): boolean {
    return value === -1;
}
"#,
        );
    }

    /// Replace boolean values written in match arm blocks.
    #[test]
    fn test_replaces_block_values() {
        let session = TestSession::dir(
            &PREFER_EQUALITY_OVER_PATTERN,
            r#"
function isZero(value: int32): boolean {
    return match (value) {
        0 => {
            true
        }
        _ => {
            false
        }
    };
}
"#,
        );

        session.assert_fixes(
            r#"
function isZero(value: int32): boolean {
    return value === 0;
}
"#,
        );
    }

    /// Accept literal matches whose arms produce nonboolean values.
    #[test]
    fn test_accepts_nonboolean_match() {
        let session = TestSession::dir(
            &PREFER_EQUALITY_OVER_PATTERN,
            r#"
function describe(value: int32): string {
    return match (value) {
        0 => "zero"
        _ => "other"
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept matches that distinguish more than one literal.
    #[test]
    fn test_accepts_multiple_literal_patterns() {
        let session = TestSession::dir(
            &PREFER_EQUALITY_OVER_PATTERN,
            r#"
function classify(value: int32): boolean {
    return match (value) {
        0 => true
        1 => false
        _ => false
    };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
