use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow length checks duplicated by the guarded array operation.
    pub NO_USELESS_LENGTH_CHECK {
        id: "no-useless-length-check",
        summary: "Disallow length checks duplicated by the guarded array operation",
        explanation: r#"
`every` returns true for an empty array, and `some` returns false.
Separate length guards duplicate these results.
Instead, you SHOULD use the array operation directly.
"#,
        example: {
            reported: r#"
function allPositive(values: int32[]): boolean {
    return values.length === 0 || values.every((value) => value > 0);
}
"#,
            accepted: r#"
function allPositive(values: int32[]): boolean {
    return values.every((value) => value > 0);
}
"#,
        },
        provenance: [Unicorn("no-useless-length-check")],
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One array predicate whose result determines an empty or nonempty guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrayPredicate {
    /// Whether at least one element is accepted.
    Some,
    /// Whether every element is accepted.
    Every,
}

/// Report redundant length guards around canonical Array predicates.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect builtin conjunctions and disjunctions
    for expression in module.operator_expressions() {
        let expression = expression?;
        let Some((logical, [left, right])) = module.builtin_binary(expression)? else {
            continue;
        };
        if !matches!(logical, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
            continue;
        }

        // align one length guard with one canonical array predicate
        let mut redundant = None;
        for (guard, predicate) in [
            (left.source.local_id, right.source.local_id),
            (right.source.local_id, left.source.local_id),
        ] {
            let Some(test) = module.emptiness_test(guard)? else {
                continue;
            };
            let Some((receiver, predicate_kind)) = array_predicate(module, predicate)? else {
                continue;
            };
            let is_redundant = matches!(
                (logical, test.is_empty, predicate_kind),
                (dir::BinaryOperator::Or, true, ArrayPredicate::Every)
                    | (dir::BinaryOperator::And, false, ArrayPredicate::Some)
            );
            if !is_redundant
                || !module.is_same_computation(test.receiver, receiver)?
                || !module.is_duplicable_expression(receiver)?
            {
                continue;
            }

            redundant = Some(predicate);
            break;
        }
        let Some(predicate) = redundant else {
            continue;
        };

        // replace the guarded expression with the retained predicate call
        let extent = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("length guard repeats the array result", extent);
        if let Some(fix) = fix(module, lint, extent, predicate)? {
            diagnostic = diagnostic.suggestion(fix);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the receiver and kind of one canonical Array predicate call.
fn array_predicate(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<(dir::LocalNodeId<dir::Expression>, ArrayPredicate)>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional() || call.arguments.len() != 1 {
        return Ok(None);
    }

    // select the canonical Array method
    let predicate = match module.language_member(expression)? {
        Some(member) if member == dir::LanguageItem::Array.member("some") => ArrayPredicate::Some,
        Some(member) if member == dir::LanguageItem::Array.member("every") => ArrayPredicate::Every,
        _ => return Ok(None),
    };

    Ok(Some((call.receiver, predicate)))
}

/// Build the unguarded array predicate call.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    extent: tspp_source::Span,
    predicate: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let retained = module.source_extent(predicate.into_any())?;
    if module.has_unretained_comment(extent, &[retained])? {
        return Ok(None);
    }

    // retain the complete predicate call
    let replacement = module.expression_source(predicate, dir::OperatorPrecedence::Postfix)?;
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix("remove the redundant length guard", patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace empty and nonempty guards in either operand order.
    #[test]
    fn test_replaces_redundant_guards() {
        let session = TestSession::dir(
            &NO_USELESS_LENGTH_CHECK,
            r#"
function accepted(values: int32[]): boolean {
    const all = values.length === 0 || values.every((value) => value > 0);
    const any = values.some((value) => value > 0) && 0 < values.length;
    return all && any;
}
"#,
        );

        session.assert_fixes(
            r#"
function accepted(values: int32[]): boolean {
    const all = values.every((value) => value > 0);
    const any = values.some((value) => value > 0);
    return all && any;
}
"#,
        );
    }

    /// Replace equivalent zero and one bounds around array predicates.
    #[test]
    fn test_replaces_equivalent_length_bounds() {
        let session = TestSession::dir(
            &NO_USELESS_LENGTH_CHECK,
            r#"
function accepted(values: int32[]): boolean {
    const all = values.length < 1 || values.every((value) => value > 0);
    const any = values.length >= 1 && values.some((value) => value > 0);
    return all && any;
}
"#,
        );

        session.assert_fixes(
            r#"
function accepted(values: int32[]): boolean {
    const all = values.every((value) => value > 0);
    const any = values.some((value) => value > 0);
    return all && any;
}
"#,
        );
    }

    /// Keep guards that do not follow the predicate's empty-array result.
    #[test]
    fn test_accepts_distinct_empty_results() {
        let session = TestSession::dir(
            &NO_USELESS_LENGTH_CHECK,
            r#"
function accepted(values: int32[]): boolean {
    const allNonempty = values.length !== 0 && values.every((value) => value > 0);
    const anyOrEmpty = values.length === 0 || values.some((value) => value > 0);
    return allNonempty || anyOrEmpty;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep a guard and predicate over different arrays.
    #[test]
    fn test_accepts_distinct_arrays() {
        let session = TestSession::dir(
            &NO_USELESS_LENGTH_CHECK,
            r#"
function accepted(left: int32[], right: int32[]): boolean {
    return left.length === 0 || right.every((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep repeated effectful receivers.
    #[test]
    fn test_accepts_effectful_receiver() {
        let session = TestSession::dir(
            &NO_USELESS_LENGTH_CHECK,
            r#"
declare function values(): int32[];
function accepted(): boolean {
    return values().length === 0 || values().every((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept user-defined length and every members.
    #[test]
    fn test_accepts_user_members() {
        let session = TestSession::dir(
            &NO_USELESS_LENGTH_CHECK,
            r#"
class Values {
    length: isize = 0;

    every(predicate: (value: int32) => boolean): boolean {
        return predicate(0);
    }
}

function accepted(values: Values): boolean {
    return values.length === 0 || values.every((value) => value > 0);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve a comment in the removed length guard by omitting the fix.
    #[test]
    fn test_reports_commented_guard_without_fix() {
        let session = TestSession::dir(
            &NO_USELESS_LENGTH_CHECK,
            r#"
function accepted(values: int32[]): boolean {
    return values.length === 0 /* retain */ || values.every((value) => value > 0);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-length-check]: length guard repeats the array result
 ──▶ main.tspp:2:12
  │
1 │ function accepted(values: int32[]): boolean {
2 │     return values.length === 0 /* retain */ || values.every((value) => value > 0);
  │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
