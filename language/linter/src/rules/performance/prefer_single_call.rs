use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Combine consecutive Array.push or Array.unshift calls.
    pub PREFER_SINGLE_CALL {
        id: "prefer-single-call",
        summary: "Combine consecutive Array.push or Array.unshift calls",
        explanation: r#"
Consecutive calls that append or prepend values to the same array repeat dispatch and capacity work.
Instead, you SHOULD pass all values to one `push` or `unshift` call.

Calls are combined only when the arguments cannot observe the intermediate array state.
"#,
        example: {
            reported: r#"
function append(values: int32[]): void {
    values.push(1);
    values.push(2);
}
"#,
            accepted: r#"
function append(values: int32[]): void {
    values.push(1, 2);
}
"#,
        },
        provenance: [Unicorn("prefer-single-call")],
        category: Performance,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One canonical variadic array mutation call.
#[derive(Debug, Clone, Copy)]
struct ArrayCall<'a> {
    /// The complete call expression.
    expression: dir::LocalNodeId<dir::Expression>,
    /// The mutated array expression.
    receiver: dir::LocalNodeId<dir::Expression>,
    /// The mutation method.
    method: ArrayMethod,
    /// The authored arguments.
    arguments: &'a [dir::LocalNodeId<dir::Argument>],
}

/// One variadic array mutation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrayMethod {
    /// Append values in call order.
    Push,
    /// Prepend values in call order.
    Unshift,
}

impl ArrayMethod {
    /// Return the authored method name.
    fn name(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Unshift => "unshift",
        }
    }
}

/// Report consecutive variadic mutations of one array.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let accesses = module.flows.access_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect maximal consecutive call runs within each block
    for (_, block) in view.iter_nodes::<dir::Block>() {
        let expressions = block.iter_expressions().collect::<Vec<_>>();
        let mut start = 0;
        while start < expressions.len() {
            let Some(first) = select_array_call(module, expressions[start])? else {
                start += 1;
                continue;
            };
            let mut calls = vec![first];

            // extend through calls with identical safe mutation behavior
            let mut end = start + 1;
            while let Some(expression) = expressions.get(end).copied() {
                let Some(next) = select_array_call(module, expression)? else {
                    break;
                };
                if !can_combine(module, first, next, &accesses)? {
                    break;
                }

                calls.push(next);
                end += 1;
            }

            // report runs containing more than one call
            if let [_, .., last] = calls.as_slice() {
                let first_span = module.source_extent(first.expression.into_any())?;
                let last_span = module.source_extent(last.expression.into_any())?;
                let span = first_span.merge(last_span);
                let message = format!("array calls `{}` repeatedly", first.method.name());
                let mut diagnostic = lint.diagnostic(message, span);
                if let Some(fix) = fix(module, lint, &calls)? {
                    diagnostic = diagnostic.suggestion(fix);
                }
                output.report(diagnostic);
            }

            start = end;
        }
    }

    Ok(output)
}

/// Select one direct canonical Array.push or Array.unshift statement.
fn select_array_call<'a>(
    module: &'a DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<ArrayCall<'a>>, ProviderError> {
    let Some(call) = module.member_call(expression) else {
        return Ok(None);
    };
    if call.is_optional() || !call.generic_arguments.is_empty() {
        return Ok(None);
    }

    // select only the canonical variadic array methods
    let member = module.language_member(expression)?;
    let method = if member == Some(dir::LanguageItem::Array.member("push")) {
        ArrayMethod::Push
    } else if member == Some(dir::LanguageItem::Array.member("unshift")) {
        ArrayMethod::Unshift
    } else {
        return Ok(None);
    };

    Ok(Some(ArrayCall {
        expression,
        receiver: call.receiver,
        method,
        arguments: call.arguments,
    }))
}

/// Return whether two calls can share one mutation without changing observations.
fn can_combine(
    module: &DirModule<'_>,
    first: ArrayCall<'_>,
    next: ArrayCall<'_>,
    accesses: &[dir::AccessOccurrence],
) -> Result<bool, ProviderError> {
    // require the same stable receiver and mutation method
    if first.method != next.method
        || !module.is_duplicable_expression(first.receiver)?
        || !module.is_duplicable_expression(next.receiver)?
        || !module.is_same_computation(first.receiver, next.receiver)?
    {
        return Ok(false);
    }
    let Some(receiver) = module.access_resolution(first.receiver) else {
        return Ok(false);
    };
    let view = module.view();

    // require arguments moved across a mutation or one another to remain stable
    let calls = [first, next];
    let reordered = match first.method {
        ArrayMethod::Push => &calls[1..],
        ArrayMethod::Unshift => &calls[..],
    };
    for call in reordered {
        for argument in call.arguments {
            let argument = view.get(*argument);
            let Some(value) = argument.value() else {
                return Ok(false);
            };
            if !module.is_speculatable_expression(value)? {
                return Ok(false);
            }

            // reject projected reads because distinct managed roots may alias
            let has_projected_read = accesses.iter().any(|occurrence| {
                view.is_inside(occurrence.node, value.into_any())
                    && occurrence.uses.contains(dir::BindingUse::READ)
                    && !occurrence.path.keys().is_empty()
            });
            if has_projected_read {
                return Ok(false);
            }

            // require spread iteration over direct owned storage
            if matches!(argument, dir::Argument::Spread { .. }) {
                let Some(spread) = module.access_resolution(value) else {
                    return Ok(false);
                };
                let is_direct = spread.path().keys().is_empty();
                let is_owned = matches!(
                    module.node_type(value.into_any())?,
                    dir::Type::Form(form) if form.form == dir::Form::Owned
                );
                if !is_direct || !is_owned || spread.path() == receiver.path() {
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

/// Combine one run into its first call.
fn fix(
    module: &DirModule<'_>,
    lint: &Lint,
    calls: &[ArrayCall<'_>],
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    // require one combinable run
    let [first, .., last] = calls else {
        return Err(ProviderError::internal(
            "combined array call run contains fewer than two calls",
        ));
    };
    let first_span = module.source_extent(first.expression.into_any())?;
    let last_statement = module.statement_span(last.expression)?;
    let extent = Span::new(first_span.file, first_span.start, last_statement.end);

    // retain the receiver and every authored argument
    let receiver_span = module.source_extent(first.receiver.into_any())?;
    let mut retained = vec![receiver_span];
    for call in calls {
        for argument in call.arguments {
            retained.push(module.source_extent(argument.into_any())?);
        }
    }
    if module.has_unretained_comment(extent, &retained)? {
        return Ok(None);
    }

    // preserve argument order for push and reverse call groups for unshift
    let mut arguments = Vec::new();
    match first.method {
        ArrayMethod::Push => {
            for call in calls {
                for argument in call.arguments {
                    let span = module.source_extent(argument.into_any())?;
                    arguments.push(module.source(span)?.to_string());
                }
            }
        }
        ArrayMethod::Unshift => {
            for call in calls.iter().rev() {
                for argument in call.arguments {
                    let span = module.source_extent(argument.into_any())?;
                    arguments.push(module.source(span)?.to_string());
                }
            }
        }
    }

    // replace every statement with one canonical call
    let receiver = module.expression_source(first.receiver, dir::OperatorPrecedence::Postfix)?;
    let method = first.method.name();
    let replacement = format!("{receiver}.{method}({});", arguments.join(", "));
    let patch = Patch::replace(extent, replacement);
    let fix = lint.fix(format!("combine the `{method}` calls"), patch)?;

    Ok(Some(fix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Combine consecutive array pushes.
    #[test]
    fn test_combines_push_calls() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
function append(values: int32[]): void {
    values.push(1);
    values.push(2);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-single-call]: array calls `push` repeatedly
 ──▶ main.tspp:2:5
  │
1 │ function append(values: int32[]): void {
2 │     values.push(1);
  │     ^^^^^^^^^^^^^^^
3 │     values.push(2);
  │     ^^^^^^^^^^^^^^
4 │ }
  │

 = fix: combine the `push` calls
--- a/main.tspp
+++ b/main.tspp

    1│ function append(values: int32[]): void {
-   2│     values.push(1);
-   3│     values.push(2);
+   2│     values.push(1, 2);
    4│ }
"#,
        );
        session.assert_fixes(
            r#"
function append(values: int32[]): void {
    values.push(1, 2);
}
"#,
        );
    }

    /// Combine complete push runs and retain spread arguments.
    #[test]
    fn test_combines_push_run() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
function append(values: int32[], more: ^int32[]): void {
    values.push(1, 2);
    values.push(...more);
    values.push(3);
}
"#,
        );

        session.assert_fixes(
            r#"
function append(values: int32[], more: ^int32[]): void {
    values.push(1, 2, ...more, 3);
}
"#,
        );
    }

    /// Reverse call groups when combining prepends.
    #[test]
    fn test_combines_unshift_calls() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
function prepend(values: int32[]): void {
    values.unshift(1, 2);
    values.unshift(3, 4);
}
"#,
        );

        session.assert_fixes(
            r#"
function prepend(values: int32[]): void {
    values.unshift(3, 4, 1, 2);
}
"#,
        );
    }

    /// Retain unshift calls whose argument evaluation order is observable.
    #[test]
    fn test_accepts_effectful_unshift_arguments() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
function prepend(values: isize[]): void {
    let value: isize = 1;
    values.unshift(value = 2);
    values.unshift(value);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Retain effectful arguments evaluated before the first mutation.
    #[test]
    fn test_combines_effectful_first_arguments() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
declare function next(): int32;

function append(values: int32[]): void {
    values.push(next());
    values.push(1);
}
"#,
        );

        session.assert_fixes(
            r#"
declare function next(): int32;

function append(values: int32[]): void {
    values.push(next(), 1);
}
"#,
        );
    }

    /// Accept calls whose arguments observe intermediate mutation state.
    #[test]
    fn test_accepts_observing_arguments() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
function append(values: isize[]): void {
    values.push(1);
    values.push(values.length);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept projected reads through roots that may alias the receiver.
    #[test]
    fn test_accepts_projected_alias() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
function append(values: isize[], alias: isize[]): void {
    values.push(1);
    values.push(alias.length);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept managed spread storage that may alias the receiver.
    #[test]
    fn test_accepts_managed_spread_alias() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
function append(values: isize[], alias: isize[]): void {
    values.push(2);
    values.push(...alias);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept effectful arguments and distinct receivers.
    #[test]
    fn test_accepts_effectful_or_distinct_calls() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
declare function next(): int32;

function append(first: int32[], second: int32[]): void {
    first.push(1);
    first.push(next());
    second.push(3);
    first.push(2);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept consecutive calls to a user-defined push method.
    #[test]
    fn test_accepts_user_method() {
        let session = TestSession::dir(
            &PREFER_SINGLE_CALL,
            r#"
class Values {
    push(value: int32): void {
        value;
    }
}

function append(values: Values): void {
    values.push(1);
    values.push(2);
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
