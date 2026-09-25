use std::collections::BTreeSet;

use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer slice patterns over indexing after a length guard.
    pub PREFER_SLICE_PATTERN {
        id: "prefer-slice-pattern",
        summary: "Prefer slice patterns over indexing after a length guard",
        explanation: r#"
A length guard followed by fixed indexing separates a sequence's required shape from its extracted values.
Instead, you SHOULD use a slice pattern to check and bind the elements together.
"#,
        example: {
            reported: r#"
function firstPair(values: int32[]): int32 | undefined {
    if (values.length >= 2) {
        return values[0] + values[1];
    }

    return undefined;
}
"#,
            accepted: r#"
function firstPair(values: int32[]): int32 | undefined {
    if (const [first, second, ...] = values) {
        return first + second;
    }

    return undefined;
}
"#,
        },
        provenance: [Clippy("index_refutable_slice")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report length guards followed by fixed indexing of the guarded sequence.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect ordinary if conditions with one expression operand
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
            condition,
            then_expression,
            ..
        } = node
        else {
            continue;
        };
        let Some(condition) = condition.as_expression() else {
            continue;
        };
        let Some((sequence, minimum)) = minimum_length(condition, module)? else {
            continue;
        };
        if minimum == 0 || !indexes_prefix(*then_expression, sequence, minimum, module)? {
            continue;
        }

        let span = module.main_span(expression.into_any())?;
        output.report(lint.diagnostic("length guard precedes fixed sequence indexing", span));
    }

    Ok(output)
}

/// Return the sequence and minimum length established by one comparison.
fn minimum_length(
    expression: dir::LocalNodeId<dir::Expression>,
    module: &DirModule<'_>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, usize)>, ProviderError> {
    let Some((operator, [left, right])) = module.builtin_binary(expression)? else {
        return Ok(None);
    };
    let Some(swapped) = operator.swapped() else {
        return Ok(None);
    };

    // normalize the length access to the left side
    for (length, bound, operator) in [
        (left.source.local_id, right.source.local_id, operator),
        (right.source.local_id, left.source.local_id, swapped),
    ] {
        let Some(sequence) = module.length_receiver(length)? else {
            continue;
        };
        let Some(bound) = module.integral_constant(bound)? else {
            continue;
        };
        let minimum = match operator {
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict if bound >= 0 => {
                bound as usize
            }
            dir::BinaryOperator::GreaterThanOrEqual if bound >= 0 => bound as usize,
            dir::BinaryOperator::GreaterThan if bound >= -1 => (bound + 1) as usize,
            _ => continue,
        };

        return Ok(Some((sequence, minimum)));
    }

    Ok(None)
}

/// Return whether a body indexes every element established by one prefix guard.
fn indexes_prefix(
    body: dir::LocalNodeId<dir::Expression>,
    sequence: dir::LocalNodeId<dir::Expression>,
    minimum: usize,
    module: &DirModule<'_>,
) -> Result<bool, ProviderError> {
    let view = module.view();
    let mut indexes = BTreeSet::new();

    // collect fixed indexes of the guarded sequence outside nested callables
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        if !view.is_inside(expression.into_any(), body.into_any())
            || module.enclosing_callable_body(expression.into_any())
                != module.enclosing_callable_body(body.into_any())
        {
            continue;
        }
        let dir::Expression::Index {
            left,
            index: Some(index),
            is_optional: false,
            ..
        } = node
        else {
            continue;
        };
        if !module.is_same_computation(*left, sequence)? {
            continue;
        }
        let Some(index) = module.integral_constant(*index)? else {
            continue;
        };
        if index >= 0 && index < minimum as i64 {
            indexes.insert(index as usize);
        }
    }

    Ok(indexes.len() == minimum && indexes.into_iter().eq(0..minimum))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report every guarded prefix element read by fixed index.
    #[test]
    fn test_reports_guarded_prefix_indexing() {
        let session = TestSession::dir(
            &PREFER_SLICE_PATTERN,
            r#"
function firstPair(values: int32[]): int32 | undefined {
    if (values.length >= 2) {
        return values[0] + values[1];
    }

    return undefined;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-slice-pattern]: length guard precedes fixed sequence indexing
 ──▶ main.tspp:2:5
  │
1 │ function firstPair(values: int32[]): int32 | undefined {
2 │     if (values.length >= 2) {
  │     ^^
3 │         return values[0] + values[1];
4 │     }
  │
"#,
        );
    }

    /// Accept sparse indexing that does not bind the guarded prefix.
    #[test]
    fn test_accepts_sparse_indexing() {
        let session = TestSession::dir(
            &PREFER_SLICE_PATTERN,
            r#"
function second(values: int32[]): int32 | undefined {
    if (values.length >= 2) {
        return values[1];
    }

    return undefined;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report fixed indexing after an exact length guard in either operand order.
    #[test]
    fn test_reports_exact_length_indexing() {
        let session = TestSession::dir(
            &PREFER_SLICE_PATTERN,
            r#"
function firstPair(values: int32[]): int32 | undefined {
    if (2 === values.length) {
        return values[0] + values[1];
    }

    return undefined;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-slice-pattern]: length guard precedes fixed sequence indexing
 ──▶ main.tspp:2:5
  │
1 │ function firstPair(values: int32[]): int32 | undefined {
2 │     if (2 === values.length) {
  │     ^^
3 │         return values[0] + values[1];
4 │     }
  │
"#,
        );
    }
}
