use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow float loop counters accumulated by repeated addition.
    pub NO_FLOAT_LOOP_COUNTER {
        id: "no-float-loop-counter",
        summary: "Disallow float loop counters accumulated by repeated addition",
        explanation: r#"
Repeated floating-point addition accumulates rounding error and may skip the intended loop bound.
Instead, you SHOULD count with an integer and derive each floating-point value from that counter.
"#,
        example: {
            reported: r#"
for (let value: float64 = 0.0; value < 1.0; value += 0.1) {
    // sample the interval
}
"#,
            accepted: r#"
for (let index: int32 = 0; index < 10; index++) {
    const value = (index as float64) / 10.0;
}
"#,
        },
        provenance: [Clippy("while_float")],
        category: Correctness,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report three-part loops that update a floating-point counter additively.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect complete counter loops
    for (_, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::For {
            condition: Some(condition),
            increment: Some(increment),
            ..
        } = node
        else {
            continue;
        };
        let Some(counter) = additive_counter(module, *increment)? else {
            continue;
        };
        if !matches!(
            module.primitive_type(counter.into_any())?,
            Some(dir::PrimitiveType::Float(_))
        ) || !condition_uses(module, *condition, counter)?
        {
            continue;
        }

        // report the accumulating update
        let span = module.source_extent(increment.into_any())?;
        output.report(lint.diagnostic(
            "floating-point loop counter accumulates rounding error",
            span,
        ));
    }

    Ok(output)
}

/// Select the place updated by one builtin additive counter operation.
fn additive_counter(
    module: &DirModule<'_>,
    increment: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let view = module.view();
    let counter = match view.get(increment) {
        dir::Expression::Unary {
            operator:
                dir::UnaryOperator::PostIncrement
                | dir::UnaryOperator::PreIncrement
                | dir::UnaryOperator::PostDecrement
                | dir::UnaryOperator::PreDecrement,
            right,
        } => *right,
        dir::Expression::Assign { .. } => {
            let Some(assignment) = module.place_assignment(increment) else {
                return Ok(None);
            };
            if !matches!(
                assignment.operator,
                dir::AssignOperator::AddAssign | dir::AssignOperator::SubtractAssign
            ) {
                return Ok(None);
            }

            assignment.target
        }
        _ => return Ok(None),
    };
    if module.builtin_operands(increment.into_any())?.is_none()
        || module.access_resolution(counter).is_none()
    {
        return Ok(None);
    }

    Ok(Some(counter))
}

/// Return whether one comparison uses the selected counter place.
fn condition_uses(
    module: &DirModule<'_>,
    condition: dir::LocalNodeId<dir::Expression>,
    counter: dir::LocalNodeId<dir::Expression>,
) -> Result<bool, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(condition)? else {
        return Ok(false);
    };
    if !matches!(
        operator,
        dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual
    ) {
        return Ok(false);
    }
    let access = module.access_resolution(counter);
    let is_used = module.access_resolution(left.source.local_id) == access
        || module.access_resolution(right.source.local_id) == access;

    Ok(is_used)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an additive floating-point loop counter.
    #[test]
    fn test_reports_float_counter() {
        let session = TestSession::dir(
            &NO_FLOAT_LOOP_COUNTER,
            r#"
for (let value: float64 = 0.0; value < 1.0; value += 0.1) {
    // sample the interval
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-float-loop-counter]: floating-point loop counter accumulates rounding error
 ──▶ main.tspp:1:45
  │
1 │ for (let value: float64 = 0.0; value < 1.0; value += 0.1) {
  │                                             ^^^^^^^^^^^^
2 │     // sample the interval
3 │ }
  │
"#,
        );
    }

    /// Accept an integer counter that derives a floating-point value.
    #[test]
    fn test_accepts_integer_counter() {
        let session = TestSession::dir(
            &NO_FLOAT_LOOP_COUNTER,
            r#"
for (let index: int32 = 0; index < 10; index++) {
    const value = (index as float64) / 10.0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
