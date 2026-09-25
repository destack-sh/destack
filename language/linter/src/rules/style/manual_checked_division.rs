use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer checked division and remainder operations over manual zero guards.
    pub MANUAL_CHECKED_DIVISION {
        id: "manual-checked-division",
        summary: "Prefer checked division over manual zero guards",
        explanation: r#"
Guarding an unsigned division or remainder operation with a zero comparison
manually implements checked division.
Instead, you SHOULD use the corresponding checked method and handle its `undefined` result.
"#,
        example: {
            reported: r#"
function divide(value: uint32, divisor: uint32): uint32 | undefined {
    if (divisor != 0) {
        return value / divisor;
    }

    return undefined;
}
"#,
            accepted: r#"
function divide(value: uint32, divisor: uint32): uint32 | undefined {
    return value.checkedDivide(divisor);
}
"#,
        },
        provenance: [Clippy("manual_checked_ops")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// The branch in which one divisor is known to be nonzero.
#[derive(Debug, Clone, Copy)]
enum NonzeroBranch {
    /// The first branch.
    Then,
    /// The alternate branch.
    Else,
}

/// Report unsigned divisions guarded by an equivalent zero comparison.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored if statements with ordinary expression conditions
    for (_, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
            condition,
            then_expression,
            else_expression,
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        let Some((divisor, branch)) = guarded_divisor(module, condition)? else {
            continue;
        };
        let branch = match branch {
            NonzeroBranch::Then => Some(*then_expression),
            NonzeroBranch::Else => *else_expression,
        };
        let Some(branch) = branch else {
            continue;
        };
        let operations = guarded_operations(module, branch, divisor)?;
        if operations.is_empty() {
            continue;
        }

        // report the guard and every division operation it protects
        let span = module.source_extent(condition.into_any())?;
        let mut diagnostic = lint
            .diagnostic("manual zero guard precedes checked division", span)
            .primary("zero check");
        for operation in operations {
            let span = module.source_extent(operation.into_any())?;
            diagnostic = diagnostic.label(span, "division operation here");
        }
        output.report(diagnostic.help("use the corresponding checked division method"));
    }

    Ok(output)
}

/// Select the divisor and branch proved nonzero by one exact unsigned comparison.
fn guarded_divisor(
    module: &DirModule<'_>,
    condition: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, NonzeroBranch)>, ProviderError> {
    let Some(comparison) = module.integer_comparison(condition, 0)? else {
        return Ok(None);
    };

    // recognize the exact comparisons that prove a nonzero unsigned value
    let branch = match comparison.operator {
        dir::BinaryOperator::NotEqual
        | dir::BinaryOperator::NotEqualStrict
        | dir::BinaryOperator::GreaterThan => Some(NonzeroBranch::Then),
        dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => Some(NonzeroBranch::Else),
        _ => None,
    };
    let Some(branch) = branch else {
        return Ok(None);
    };
    let divisor = comparison.value;
    let primitive = module.primitive_type(divisor.into_any())?;
    if !primitive.is_some_and(dir::PrimitiveType::is_unsigned_integer)
        || !module.is_duplicable_expression(divisor)?
    {
        return Ok(None);
    }

    Ok(Some((divisor, branch)))
}

/// Return guarded divisions when the first branch use of the divisor is a division.
fn guarded_operations(
    module: &DirModule<'_>,
    branch: dir::LocalNodeId<dir::Expression>,
    divisor: dir::LocalNodeId<dir::Expression>,
) -> Result<Vec<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let view = module.view();
    let callable = module.enclosing_callable_body(branch.into_any());
    let mut uses = Vec::new();

    // collect equivalent divisor uses in source order without entering nested callables
    for (expression, _) in view.iter_nodes::<dir::Expression>() {
        if !view.is_inside(expression.into_any(), branch.into_any())
            || module.enclosing_callable_body(expression.into_any()) != callable
            || !module.is_same_computation(expression, divisor)?
        {
            continue;
        }
        let span = module.source_extent(expression.into_any())?;
        let operation = containing_division(module, expression, divisor)?;
        uses.push((span.start, operation));
    }
    uses.sort_by_key(|(start, _)| *start);

    // require division to be the first use after the guard
    if uses
        .first()
        .is_none_or(|(_, operation)| operation.is_none())
    {
        return Ok(Vec::new());
    }
    let mut operations = Vec::new();
    for (_, operation) in uses {
        if let Some(operation) = operation
            && !operations.contains(&operation)
        {
            operations.push(operation);
        }
    }

    Ok(operations)
}

/// Return the direct unsigned division or remainder that consumes one divisor occurrence.
fn containing_division(
    module: &DirModule<'_>,
    occurrence: dir::LocalNodeId<dir::Expression>,
    divisor: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let view = module.view();
    let Some(divisor_type) = module.primitive_type(divisor.into_any())? else {
        return Ok(None);
    };

    // recognize builtin division and remainder by the guarded value
    if let Some(parent) = view
        .get_parent_for(occurrence)
        .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
    {
        if let Some((operator, [left, right])) = module.builtin_binary(parent)?
            && matches!(
                operator,
                dir::BinaryOperator::Divide | dir::BinaryOperator::Remainder
            )
            && right.source.local_id == occurrence
            && module.primitive_type(left.source.local_id.into_any())? == Some(divisor_type)
        {
            return Ok(Some(parent));
        }

        // recognize compound division and remainder by the guarded value
        if let Some(assignment) = module.place_assignment(parent)
            && let Ok(operator) = dir::BinaryOperator::try_from(assignment.operator)
            && matches!(
                operator,
                dir::BinaryOperator::Divide | dir::BinaryOperator::Remainder
            )
            && assignment.value == occurrence
            && module.primitive_type(assignment.target.into_any())? == Some(divisor_type)
        {
            return Ok(Some(parent));
        }
    }

    // recognize canonical Euclidean division and remainder calls
    let Some(call_expression) = module.argument_call(occurrence) else {
        return Ok(None);
    };
    let Some(call) = module.member_call(call_expression) else {
        return Ok(None);
    };
    let [argument] = call.arguments else {
        return Ok(None);
    };
    if call.is_optional()
        || !call.generic_arguments.is_empty()
        || view.get(*argument).value() != Some(occurrence)
        || module.primitive_type(call.receiver.into_any())? != Some(divisor_type)
    {
        return Ok(None);
    }
    let member = module.language_member(call_expression)?;
    if matches!(
        member,
        Some(member)
            if member == dir::LanguageItem::Integer.member("divideEuclidean")
                || member == dir::LanguageItem::Integer.member("remainderEuclidean")
    ) {
        return Ok(Some(call_expression));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report division in a branch guarded by a nonzero comparison.
    #[test]
    fn test_reports_nonzero_guard() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            MANUAL_CHECKED_DIVISION.example.reported.source(),
        );

        session.assert_diagnostics(
            r#"warning[manual-checked-division]: manual zero guard precedes checked division
 ──▶ main.tspp:2:9
  │
1 │ function divide(value: uint32, divisor: uint32): uint32 | undefined {
2 │     if (divisor != 0) {
  │         ^^^^^^^^^^^^ zero check
3 │         return value / divisor;
  │                --------------- division operation here
4 │     }
5 │
  │

 = help: use the corresponding checked division method
"#,
        );
    }

    /// Report division in the alternate branch of a zero comparison.
    #[test]
    fn test_reports_zero_branch() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
function divide(value: uint32, divisor: uint32): uint32 {
    if (divisor == 0) {
        return value;
    } else {
        return value / divisor;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[manual-checked-division]: manual zero guard precedes checked division
 ──▶ main.tspp:2:9
  │
1 │ function divide(value: uint32, divisor: uint32): uint32 {
2 │     if (divisor == 0) {
  │         ^^^^^^^^^^^^ zero check
3 │         return value;
4 │     } else {
5 │         return value / divisor;
  │                --------------- division operation here
6 │     }
7 │ }
  │

 = help: use the corresponding checked division method
"#,
        );
    }

    /// Report every division protected by one guard.
    #[test]
    fn test_reports_multiple_divisions() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
declare function consume(value: uint32): void;

function divide(first: uint32, second: uint32, divisor: uint32): void {
    if (divisor > 0) {
        consume(first / divisor);
        consume(second / divisor);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[manual-checked-division]: manual zero guard precedes checked division
 ──▶ main.tspp:4:9
  │
2 │
3 │ function divide(first: uint32, second: uint32, divisor: uint32): void {
4 │     if (divisor > 0) {
  │         ^^^^^^^^^^^ zero check
5 │         consume(first / divisor);
  │                 --------------- division operation here
6 │         consume(second / divisor);
  │                 ---------------- division operation here
7 │     }
8 │ }
  │

 = help: use the corresponding checked division method
"#,
        );
    }

    /// Report compound division protected by one guard.
    #[test]
    fn test_reports_compound_division() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
function divide(value: uint32, divisor: uint32): uint32 {
    let result = value;
    if (0 < divisor) {
        result /= divisor;
    }
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[manual-checked-division]: manual zero guard precedes checked division
 ──▶ main.tspp:3:9
  │
1 │ function divide(value: uint32, divisor: uint32): uint32 {
2 │     let result = value;
3 │     if (0 < divisor) {
  │         ^^^^^^^^^^^ zero check
4 │         result /= divisor;
  │         ----------------- division operation here
5 │     }
6 │     return result;
  │

 = help: use the corresponding checked division method
"#,
        );
    }

    /// Report remainder protected by one nonzero comparison.
    #[test]
    fn test_reports_remainder() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
function remainder(value: uint32, divisor: uint32): uint32 | undefined {
    if (divisor !== 0) {
        return value % divisor;
    }

    return undefined;
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[manual-checked-division]: manual zero guard precedes checked division
 ──▶ main.tspp:2:9
  │
1 │ function remainder(value: uint32, divisor: uint32): uint32 | undefined {
2 │     if (divisor !== 0) {
  │         ^^^^^^^^^^^^^ zero check
3 │         return value % divisor;
  │                --------------- division operation here
4 │     }
5 │
  │

 = help: use the corresponding checked division method
"#,
        );
    }

    /// Report canonical Euclidean operations protected by one nonzero comparison.
    #[test]
    fn test_reports_euclidean_operations() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
declare function consume(value: uint32): void;

function divide(value: uint32, divisor: uint32): void {
    if (divisor > 0) {
        consume(value.divideEuclidean(divisor));
        consume(value.remainderEuclidean(divisor));
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"warning[manual-checked-division]: manual zero guard precedes checked division
 ──▶ main.tspp:4:9
  │
2 │
3 │ function divide(value: uint32, divisor: uint32): void {
4 │     if (divisor > 0) {
  │         ^^^^^^^^^^^ zero check
5 │         consume(value.divideEuclidean(divisor));
  │                 ------------------------------ division operation here
6 │         consume(value.remainderEuclidean(divisor));
  │                 --------------------------------- division operation here
7 │     }
8 │ }
  │

 = help: use the corresponding checked division method
"#,
        );
    }

    /// Accept signed division because another overflow case remains.
    #[test]
    fn test_accepts_signed_division() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
function divide(value: int32, divisor: int32): int32 | undefined {
    if (divisor != 0) {
        return value / divisor;
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a divisor used before the guarded division.
    #[test]
    fn test_accepts_earlier_divisor_use() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
declare function consume(value: uint32): void;

function divide(value: uint32, divisor: uint32): void {
    if (divisor != 0) {
        consume(divisor);
        consume(value / divisor);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept division by another value.
    #[test]
    fn test_accepts_other_divisor() {
        let session = TestSession::dir(
            &MANUAL_CHECKED_DIVISION,
            r#"
function divide(value: uint32, checked: uint32, divisor: uint32): uint32 | undefined {
    if (checked != 0) {
        return value / divisor;
    }
    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
