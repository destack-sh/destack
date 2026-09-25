use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer midpoint operations that cannot overflow intermediate arithmetic.
    pub MANUAL_MIDPOINT {
        id: "manual-midpoint",
        summary: "Prefer midpoint operations that cannot overflow intermediate arithmetic",
        explanation: r#"
Dividing the sum of two numeric values computes the sum before reducing it and may overflow.
Instead, you SHOULD call `.midpoint()` on one operand.

Floating-point midpoint rounding can differ from the expanded arithmetic.
"#,
        example: {
            reported: r#"
function middle(left: uint32, right: uint32): uint32 {
    return (left + right) / 2;
}
"#,
            accepted: r#"
function middle(left: uint32, right: uint32): uint32 {
    return left.midpoint(right);
}
"#,
        },
        provenance: [Clippy("manual_midpoint")],
        category: Correctness,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// One manually computed midpoint.
struct Midpoint {
    /// The first endpoint.
    left: dir::LocalNodeId<dir::Expression>,
    /// The second endpoint.
    right: dir::LocalNodeId<dir::Expression>,
}

impl Midpoint {
    /// Select the summed endpoints from one midpoint scaling operation.
    fn select(
        module: &DirModule<'_>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<Self>, ProviderError> {
        let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
            return Ok(None);
        };

        // select the addition scaled by two, one half, or one right shift
        let sum = match operator {
            dir::BinaryOperator::Divide
                if module.is_numeric_constant(right.source.local_id, 2.0)? =>
            {
                left.source.local_id
            }
            dir::BinaryOperator::Multiply
                if module.is_numeric_constant(left.source.local_id, 0.5)? =>
            {
                right.source.local_id
            }
            dir::BinaryOperator::Multiply
                if module.is_numeric_constant(right.source.local_id, 0.5)? =>
            {
                left.source.local_id
            }
            dir::BinaryOperator::ShiftRight
                if module.is_numeric_constant(right.source.local_id, 1.0)? =>
            {
                left.source.local_id
            }
            _ => return Ok(None),
        };
        let Some((dir::BinaryOperator::Add, operands @ [first, second])) =
            module.builtin_binary(sum)?
        else {
            return Ok(None);
        };
        let result_type = module.node_type_id(expression.into_any())?;
        let is_supported = matches!(
            module.primitive_type(sum.into_any())?,
            Some(dir::PrimitiveType::Integer(_) | dir::PrimitiveType::Float(_))
        );
        if !operands.iter().all(|operand| {
            operand
                .scalar_families
                .as_ref()
                .is_some_and(|families| families.is_numeric())
        }) || !is_supported
            || first.ty != second.ty
            || first.ty != result_type
        {
            return Ok(None);
        }

        // require logical right-shift behavior
        if operator == dir::BinaryOperator::ShiftRight
            && !module
                .primitive_type(sum.into_any())?
                .is_some_and(dir::PrimitiveType::is_unsigned_integer)
        {
            return Ok(None);
        }

        Ok(Some(Self {
            left: first.source.local_id,
            right: second.source.local_id,
        }))
    }
}

/// Report midpoint arithmetic that first computes an overflowing sum.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin scaling operations
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some(midpoint) = Midpoint::select(module, expression)? else {
            continue;
        };

        // replace the arithmetic with the overflow-safe method
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("midpoint arithmetic may overflow its sum", span);
        if let Some(suggestion) = suggestion(module, lint, expression, &midpoint)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build the canonical midpoint call.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    midpoint: &Midpoint,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let span = module.source_extent(expression.into_any())?;
    let left_span = module.source_extent(midpoint.left.into_any())?;
    let right_span = module.source_extent(midpoint.right.into_any())?;
    if module.has_unretained_comment(span, &[left_span, right_span])? {
        return Ok(None);
    }

    // retain both endpoints in one midpoint call
    let left = module.expression_source(midpoint.left, dir::OperatorPrecedence::Postfix)?;
    let right = module.source(right_span)?;
    let patch = Patch::replace(span, format!("{left}.midpoint({right})"));
    let suggestion = lint.suggestion("call `.midpoint()`", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace floating-point multiplication and an unsigned shift.
    #[test]
    fn test_replaces_midpoint_scaling() {
        let session = TestSession::dir(
            &MANUAL_MIDPOINT,
            r#"
function floating(left: float64, right: float64): float64 {
    return 0.5 * (left + right);
}
function shifted(left: uint32, right: uint32): uint32 {
    return (left + right) >> 1;
}
"#,
        );

        session.assert_suggestions(
            r#"
function floating(left: float64, right: float64): float64 {
    return left.midpoint(right);
}
function shifted(left: uint32, right: uint32): uint32 {
    return left.midpoint(right);
}
"#,
        );
    }

    /// Accept unrelated scaling arithmetic.
    #[test]
    fn test_accepts_unrelated_scaling() {
        let session = TestSession::dir(
            &MANUAL_MIDPOINT,
            r#"
function scale(left: float64, right: float64): float64 {
    return 0.25 * (left + right);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
