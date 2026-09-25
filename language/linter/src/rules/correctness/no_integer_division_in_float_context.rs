use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow integer division whose result immediately becomes a float.
    pub NO_INTEGER_DIVISION_IN_FLOAT_CONTEXT {
        id: "no-integer-division-in-float-context",
        summary: "Disallow integer division whose result immediately becomes a float",
        explanation: r#"
Converting the result of integer division to a float preserves the already-truncated integer quotient.
Instead, you SHOULD convert both operands to the intended floating-point type before dividing them.
"#,
        example: {
            reported: r#"
function ratio(left: int32, right: int32): float64 {
    return (left / right) as float64;
}
"#,
            accepted: r#"
function ratio(left: int32, right: int32): float64 {
    return (left as float64) / (right as float64);
}
"#,
        },
        provenance: [Clippy("integer_division")],
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report integer division cast immediately to a floating-point type.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect explicit casts to concrete floating-point types
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::As {
            expression: division,
            target_type,
        } = node
        else {
            continue;
        };
        if !matches!(
            module.primitive_type(target_type.into_any())?,
            Some(dir::PrimitiveType::Float(_))
        ) {
            continue;
        }

        // require compiler-defined division over integral operands
        let Some((dir::BinaryOperator::Divide, operands @ [left, right])) =
            module.builtin_binary(*division)?
        else {
            continue;
        };
        if !operands.iter().all(dir::BuiltinOperand::is_integral) {
            continue;
        }

        // move the conversion onto both division operands
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("integer quotient is converted to a float", span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            left.source.local_id,
            right.source.local_id,
            *target_type,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build floating-point division with conversions on both operands.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    left: dir::LocalNodeId<dir::Expression>,
    right: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::TypeExpression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let left_span = module.source_extent(left.into_any())?;
    let right_span = module.source_extent(right.into_any())?;
    let target_span = module.source_extent(target.into_any())?;
    if module.has_unretained_comment(span, &[left_span, right_span, target_span])? {
        return Ok(None);
    }

    // retain each operand and the exact authored target type
    let left = module.expression_source(left, dir::OperatorPrecedence::Comparison)?;
    let right = module.expression_source(right, dir::OperatorPrecedence::Comparison)?;
    let target = module.source(target_span)?;
    let replacement = format!("({left} as {target}) / ({right} as {target})");
    let patch = Patch::replace(span, replacement);
    let suggestion = lint.suggestion("convert both operands before division", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Move a floating-point cast onto both integer operands.
    #[test]
    fn test_converts_operands_before_division() {
        let session = TestSession::dir(
            &NO_INTEGER_DIVISION_IN_FLOAT_CONTEXT,
            r#"
function ratio(left: int32, right: int32): float64 {
    return ((left + 1) / right) as float64;
}
"#,
        );

        session.assert_suggestions(
            r#"
function ratio(left: int32, right: int32): float64 {
    return ((left + 1) as float64) / (right as float64);
}
"#,
        );
    }

    /// Accept division already performed in floating point.
    #[test]
    fn test_accepts_float_division() {
        let session = TestSession::dir(
            &NO_INTEGER_DIVISION_IN_FLOAT_CONTEXT,
            r#"
function ratio(left: int32, right: int32): float64 {
    return (left as float64) / (right as float64);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an integer quotient retained as an integer.
    #[test]
    fn test_accepts_integer_quotient() {
        let session = TestSession::dir(
            &NO_INTEGER_DIVISION_IN_FLOAT_CONTEXT,
            r#"
function quotient(left: int32, right: int32): int32 {
    return left / right;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
