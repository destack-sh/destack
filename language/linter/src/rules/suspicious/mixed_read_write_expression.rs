use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow reading and mutating the same place in one combined expression.
    pub MIXED_READ_WRITE_EXPRESSION {
        id: "mixed-read-write-expression",
        summary: "Disallow reading and mutating the same place in one combined expression",
        explanation: r#"
Reading and mutating the same storage across subexpressions forces the result to depend on their evaluation order.
Instead, you SHOULD perform the mutation in a separate statement before using the resulting value.
"#,
        example: {
            reported: r#"
function advance(): int32 {
    let value: int32 = 0;
    return value++ + value;
}
"#,
            accepted: r#"
function advance(): int32 {
    let value: int32 = 0;
    const previous = value;
    value++;
    return previous + value;
}
"#,
        },
        provenance: [Clippy("mixed_read_write_in_expression")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report combined expressions whose subexpressions read and mutate overlapping storage.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut accesses: FxIndexMap<
        (Option<dir::LocalNodeId<dir::Expression>>, dir::AccessRoot),
        Vec<dir::AccessOccurrence>,
    > = FxIndexMap::default();
    let mut reported = FxIndexSet::default();
    let mut output = LintOutput::default();

    // partition accesses by callable and stable root before comparing paths
    for occurrence in module.flows.access_occurrences() {
        let callable = module.enclosing_callable_body(occurrence.node);
        accesses
            .entry((callable, occurrence.path.root()))
            .or_default()
            .push(occurrence);
    }

    // compare each mutation with reads sharing its callable and root
    for occurrences in accesses.values() {
        for mutation in occurrences
            .iter()
            .filter(|occurrence| occurrence.uses.may_mutate())
        {
            for read in occurrences.iter().filter(|occurrence| {
                occurrence.uses.contains(dir::BindingUse::READ)
                    && !is_projection_receiver(module, occurrence)
            }) {
                if mutation.node == read.node
                    || !(mutation.path.starts_with(&read.path)
                        || read.path.starts_with(&mutation.path))
                {
                    continue;
                }
                let Some(expression) =
                    view.common_ancestor::<dir::Expression>(mutation.node, read.node)
                else {
                    continue;
                };
                if !combines_evaluated_values(view.get(expression))
                    || are_alternative_branches(&view, expression, mutation.node, read.node)
                    || !reported.insert(expression)
                {
                    continue;
                }

                // report the nearest expression combining the conflicting evaluations
                let span = module.source_extent(expression.into_any())?;
                let diagnostic =
                    lint.diagnostic("expression reads and mutates the same place", span);
                output.report(diagnostic);
            }
        }
    }

    Ok(output)
}

/// Return whether an access only selects the receiver of a more specific place.
fn is_projection_receiver(module: &DirModule<'_>, occurrence: &dir::AccessOccurrence) -> bool {
    let Ok(expression) = occurrence.node.try_into_typed::<dir::Expression>() else {
        return false;
    };
    let Some(parent) = module.view().get_parent_for(expression) else {
        return false;
    };
    let Ok(parent) = parent.try_into_typed::<dir::Expression>() else {
        return false;
    };
    let is_receiver = matches!(
        module.view().get(parent),
        dir::Expression::Member { left, .. } | dir::Expression::Index { left, .. }
            if *left == expression
    );
    if !is_receiver {
        return false;
    }
    let Some(projected) = module.access_resolution(parent) else {
        return false;
    };

    projected.path() != &occurrence.path && projected.path().starts_with(&occurrence.path)
}

/// Return whether one expression directly combines independently authored values.
fn combines_evaluated_values(expression: &dir::Expression) -> bool {
    matches!(
        expression,
        dir::Expression::RangeExpression { .. }
            | dir::Expression::TemplateExpression { .. }
            | dir::Expression::TaggedTemplateExpression { .. }
            | dir::Expression::ArrayExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TupleExpression { .. }
            | dir::Expression::ObjectExpression { .. }
            | dir::Expression::StructExpression { .. }
            | dir::Expression::TreeExpression { .. }
            | dir::Expression::InstanceOf { .. }
            | dir::Expression::Index { .. }
            | dir::Expression::Call { .. }
            | dir::Expression::New { .. }
            | dir::Expression::Binary { .. }
            | dir::Expression::If {
                form: dir::IfForm::Ternary,
                ..
            }
    )
}

/// Return whether two nodes occur in opposite branches of one ternary.
fn are_alternative_branches(
    view: &dir::View<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    left: dir::LocalNodeIdAny,
    right: dir::LocalNodeIdAny,
) -> bool {
    let dir::Expression::If {
        form: dir::IfForm::Ternary,
        then_expression,
        else_expression: Some(else_expression),
        ..
    } = view.get(expression)
    else {
        return false;
    };

    // compare both possible branch orientations
    view.is_inside(left, then_expression.into_any())
        && view.is_inside(right, else_expression.into_any())
        || view.is_inside(right, then_expression.into_any())
            && view.is_inside(left, else_expression.into_any())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an update and read combined by a binary expression.
    #[test]
    fn test_reports_binary_update_and_read() {
        let session = TestSession::dir(
            &MIXED_READ_WRITE_EXPRESSION,
            r#"
function advance(): int32 {
    let value: int32 = 0;
    return value++ + value;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[mixed-read-write-expression]: expression reads and mutates the same place
 ──▶ main.tspp:3:12
  │
1 │ function advance(): int32 {
2 │     let value: int32 = 0;
3 │     return value++ + value;
  │            ^^^^^^^^^^^^^^^
4 │ }
  │
"#,
        );
    }

    /// Report a mutation and read split across call arguments.
    #[test]
    fn test_reports_call_arguments() {
        let session = TestSession::dir(
            &MIXED_READ_WRITE_EXPRESSION,
            r#"
function combine(left: int32, right: int32): int32 {
    return left + right;
}

function advance(): int32 {
    let value: int32 = 0;
    return combine(value++, value);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[mixed-read-write-expression]: expression reads and mutates the same place
 ──▶ main.tspp:7:12
  │
5 │ function advance(): int32 {
6 │     let value: int32 = 0;
7 │     return combine(value++, value);
  │            ^^^^^^^^^^^^^^^^^^^^^^^
8 │ }
  │
"#,
        );
    }

    /// Report a mutation in a ternary condition followed by a branch read.
    #[test]
    fn test_reports_ternary_update_and_read() {
        let session = TestSession::dir(
            &MIXED_READ_WRITE_EXPRESSION,
            r#"
function advance(): int32 {
    let value: int32 = 0;
    return value++ > 0 ? value : 0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[mixed-read-write-expression]: expression reads and mutates the same place
 ──▶ main.tspp:3:12
  │
1 │ function advance(): int32 {
2 │     let value: int32 = 0;
3 │     return value++ > 0 ? value : 0;
  │            ^^^^^^^^^^^^^^^^^^^^^^^
4 │ }
  │
"#,
        );
    }

    /// Accept reads and mutations in mutually exclusive ternary branches.
    #[test]
    fn test_accepts_alternative_ternary_branches() {
        let session = TestSession::dir(
            &MIXED_READ_WRITE_EXPRESSION,
            r#"
function advance(condition: boolean): int32 {
    let value: int32 = 0;
    return condition ? value++ : value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an ordinary assignment that reads its previous value.
    #[test]
    fn test_accepts_assignment_read() {
        let session = TestSession::dir(
            &MIXED_READ_WRITE_EXPRESSION,
            r#"
function advance(): int32 {
    let value: int32 = 0;
    value = value + 1;
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept reads and mutations of distinct fields.
    #[test]
    fn test_accepts_distinct_fields() {
        let session = TestSession::dir(
            &MIXED_READ_WRITE_EXPRESSION,
            r#"
struct Point {
    x: int32;
    y: int32;
}

function advance(point: Point): int32 {
    return point.x++ + point.y;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept mutations deferred inside a lambda body.
    #[test]
    fn test_accepts_deferred_lambda_mutation() {
        let session = TestSession::dir(
            &MIXED_READ_WRITE_EXPRESSION,
            r#"
function evaluate(callback: () => int32, value: int32): int32 {
    return callback() + value;
}

function advance(): int32 {
    let value: int32 = 0;
    return evaluate((): int32 => value++, value);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
