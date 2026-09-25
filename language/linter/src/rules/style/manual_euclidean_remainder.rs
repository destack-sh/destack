use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer remainderEuclidean over its manual double-remainder form.
    pub MANUAL_EUCLIDEAN_REMAINDER {
        id: "manual-euclidean-remainder",
        summary: "Prefer remainderEuclidean over its manual double-remainder form",
        explanation: r#"
Applying remainder twice around an added positive divisor manually normalizes a negative remainder.
Instead, you SHOULD call `.remainderEuclidean()` on the dividend.
"#,
        example: {
            reported: r#"
function wrap(value: int32): int32 {
    return ((value % 4) + 4) % 4;
}
"#,
            accepted: r#"
function wrap(value: int32): int32 {
    return value.remainderEuclidean(4);
}
"#,
        },
        provenance: [Clippy("manual_rem_euclid")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One manual Euclidean remainder.
struct EuclideanRemainder {
    /// The dividend.
    dividend: dir::LocalNodeId<dir::Expression>,
    /// The repeated divisor.
    divisor: dir::LocalNodeId<dir::Expression>,
}

/// Report canonical double-remainder normalization.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin outer remainder operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some(remainder) = euclidean_remainder(module, expression)? else {
            continue;
        };

        // replace the full normalization with the canonical method
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic =
            lint.diagnostic("double remainder manually normalizes the result", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &remainder)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Select one exact `((value % modulus) + modulus) % modulus` expression.
fn euclidean_remainder(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<EuclideanRemainder>, ProviderError> {
    let Some((dir::BinaryOperator::Remainder, operands @ [normalized, outer_divisor])) =
        module.builtin_binary(expression)?
    else {
        return Ok(None);
    };
    if !operands.iter().all(dir::BuiltinOperand::is_integral) {
        return Ok(None);
    }

    // require signed arithmetic and an exact positive divisor
    let is_signed = module
        .primitive_type(expression.into_any())?
        .is_some_and(dir::PrimitiveType::is_signed_integer);
    let is_positive = module
        .integral_constant(outer_divisor.source.local_id)?
        .is_some_and(|value| value > 0);
    if !is_signed || !is_positive {
        return Ok(None);
    }

    // select the normalized addition
    let Some((dir::BinaryOperator::Add, [left, right])) =
        module.builtin_binary(normalized.source.local_id)?
    else {
        return Ok(None);
    };

    // normalize the inner remainder onto the left
    for (remainder, added_divisor) in [(left, right), (right, left)] {
        let Some((dir::BinaryOperator::Remainder, [dividend, inner_divisor])) =
            module.builtin_binary(remainder.source.local_id)?
        else {
            continue;
        };
        if module.is_same_operand(inner_divisor, added_divisor)?
            && module.is_same_operand(inner_divisor, outer_divisor)?
        {
            return Ok(Some(EuclideanRemainder {
                dividend: dividend.source.local_id,
                divisor: outer_divisor.source.local_id,
            }));
        }
    }

    Ok(None)
}

/// Build the canonical Euclidean remainder call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    remainder: &EuclideanRemainder,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let dividend_span = module.source_extent(remainder.dividend.into_any())?;
    let divisor_span = module.source_extent(remainder.divisor.into_any())?;
    if module.has_unretained_comment(span, &[dividend_span, divisor_span])? {
        return Ok(None);
    }

    // retain the dividend and one evaluation of the divisor
    let dividend =
        module.expression_source(remainder.dividend, dir::OperatorPrecedence::Postfix)?;
    let divisor = module.source(divisor_span)?;
    let replacement = format!("{dividend}.remainderEuclidean({divisor})");
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.fix("call `.remainderEuclidean()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a commuted double-remainder normalization.
    #[test]
    fn test_replaces_commuted_normalization() {
        let session = TestSession::dir(
            &MANUAL_EUCLIDEAN_REMAINDER,
            r#"
function wrap(value: int32): int32 {
    return (4 + (value % 4)) % 4;
}
"#,
        );

        session.assert_fixes(
            r#"
function wrap(value: int32): int32 {
    return value.remainderEuclidean(4);
}
"#,
        );
    }

    /// Accept a single remainder operation.
    #[test]
    fn test_accepts_single_remainder() {
        let session = TestSession::dir(
            &MANUAL_EUCLIDEAN_REMAINDER,
            r#"
function remainder(value: int32, modulus: int32): int32 {
    return value % modulus;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept normalization with a signed divisor whose sign is unknown.
    #[test]
    fn test_accepts_unknown_signed_divisor() {
        let session = TestSession::dir(
            &MANUAL_EUCLIDEAN_REMAINDER,
            r#"
function wrap(value: int32, modulus: int32): int32 {
    return ((value % modulus) + modulus) % modulus;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept unsigned normalization because its intermediate addition can wrap.
    #[test]
    fn test_accepts_unsigned_normalization() {
        let session = TestSession::dir(
            &MANUAL_EUCLIDEAN_REMAINDER,
            r#"
function wrap(value: uint8): uint8 {
    return ((value % 200) + 200) % 200;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
