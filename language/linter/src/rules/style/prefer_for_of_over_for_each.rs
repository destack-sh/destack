use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer for-of over Array.forEach callbacks.
    pub PREFER_FOR_OF_OVER_FOR_EACH {
        id: "prefer-for-of-over-for-each",
        summary: "Prefer for-of over Array.forEach callbacks",
        explanation: r#"
An Array.forEach callback introduces a function boundary for ordinary sequential iteration.
Instead, you SHOULD use a for-of loop when the callback boundary is unnecessary.

A `return` inside the callback exits only that callback and requires manual restructuring.
"#,
        example: {
            reported: r#"
function copy(values: int32[], output: int32[]): void {
    values.forEach((value) => {
        output.push(value);
    });
}
"#,
            accepted: r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical Array.forEach calls.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // inspect canonical Array.forEach calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if module.language_member(expression)? != Some(dir::LanguageItem::Array.member("forEach")) {
            continue;
        }

        // report the callback iteration and offer a loop when control flow permits
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("array iteration uses a forEach callback", span);
        if !call.is_optional
            && !call.is_member_optional
            && let Some(suggestion) =
                suggestion(module, lint, expression, call.receiver, call.arguments)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build a for-of loop for one structurally direct callback.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    receiver: dir::LocalNodeId<dir::Expression>,
    arguments: &[dir::LocalNodeId<dir::Argument>],
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let view = module.view();

    // require the call to be a direct statement with one lambda argument
    if !view
        .get_parent_for(expression)
        .is_some_and(|parent| parent.ty == dir::NodeType::Block)
    {
        return Ok(None);
    }
    let [argument] = arguments else {
        return Ok(None);
    };
    let dir::Argument::Positional { value: callback } = view.get(*argument) else {
        return Ok(None);
    };
    let Some(lambda) = module.lambda(*callback) else {
        return Ok(None);
    };
    if lambda.signature.asynchrony != dir::Asynchrony::Sync || lambda.signature.is_generator {
        return Ok(None);
    }
    let [parameter] = lambda.signature.parameters.as_slice() else {
        return Ok(None);
    };
    if !matches!(view.get(*parameter), dir::Parameter::Named { .. }) {
        return Ok(None);
    }
    let Some(body) = lambda.body else {
        return Ok(None);
    };
    if !matches!(view.get(body), dir::Expression::Block(_)) {
        return Ok(None);
    }

    // reject callback-local returns whose meaning changes after inlining
    let has_return = view
        .iter_nodes::<dir::Expression>()
        .any(|(node, expression)| {
            matches!(expression, dir::Expression::Return { .. })
                && module.enclosing_callable_body(node.into_any()) == lambda.body
        });
    if has_return {
        return Ok(None);
    }

    // retain comments only when they remain inside the parameter, receiver, or body
    let replacement_extent = module.statement_span(expression)?;
    let parameter = module.main_span(parameter.into_any())?;
    let receiver_extent = module.source_extent(receiver.into_any())?;
    let body = module.source_extent(body.into_any())?;
    if module.has_unretained_comment(replacement_extent, &[parameter, receiver_extent, body])? {
        return Ok(None);
    }

    // compose the equivalent source loop from authored subexpressions
    let parameter = module.source(parameter)?;
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Lowest)?;
    let body = module.source(body)?;
    let replacement = format!("for (const {parameter} of {receiver}) {body}");
    let patch = Patch::replace(replacement_extent, replacement);
    let suggestion = lint.suggestion("use a for-of loop", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a direct one-parameter Array.forEach statement.
    #[test]
    fn test_replaces_array_for_each() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: int32[], output: int32[]): void {
    values.forEach((value) => {
        output.push(value);
    });
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-for-of-over-for-each]: array iteration uses a forEach callback
 ──▶ main.ds:2:5
  │
1 │ function copy(values: int32[], output: int32[]): void {
2 │     values.forEach((value) => {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         output.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
4 │     });
  │     ^^
5 │ }
  │

 = suggestion: use a for-of loop (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function copy(values: int32[], output: int32[]): void {
-   2│     values.forEach((value) => {
+   2│     for (const value of values) {

    3│         output.push(value);
-   4│     });
+   4│     }
"#,
        );
        session.assert_suggestions(PREFER_FOR_OF_OVER_FOR_EACH.example.accepted());
    }

    /// Report callback-local return without offering an unsafe rewrite.
    #[test]
    fn test_reports_callback_return_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: int32[], output: int32[]): void {
    values.forEach((value) => {
        if (value < 0) {
            return;
        }
        output.push(value);
    });
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-for-of-over-for-each]: array iteration uses a forEach callback
 ──▶ main.ds:2:5
  │
1 │ function copy(values: int32[], output: int32[]): void {
2 │     values.forEach((value) => {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         if (value < 0) {
  │         ^^^^^^^^^^^^^^^^
4 │             return;
  │             ^^^^^^^
5 │         }
  │         ^
6 │         output.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
7 │     });
  │     ^^
8 │ }
  │
"#,
        );
    }

    /// Report use of the index parameter without guessing a loop rewrite.
    #[test]
    fn test_reports_indexed_callback_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: int32[], indexes: isize[], output: int32[]): void {
    values.forEach((value, index) => {
        indexes.push(index);
        output.push(value);
    });
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-for-of-over-for-each]: array iteration uses a forEach callback
 ──▶ main.ds:2:5
  │
1 │ function copy(values: int32[], indexes: isize[], output: int32[]): void {
2 │     values.forEach((value, index) => {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         indexes.push(index);
  │         ^^^^^^^^^^^^^^^^^^^^
4 │         output.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
5 │     });
  │     ^^
6 │ }
  │
"#,
        );
    }

    /// Accept a user-defined forEach method.
    #[test]
    fn test_accepts_user_for_each() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
class Values {
    forEach(visit: (value: int32) => void): void {
        visit(1);
    }
}

function copy(values: Values, output: int32[]): void {
    values.forEach((value) => {
        output.push(value);
    });
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an async callback without offering a sequential rewrite.
    #[test]
    fn test_reports_async_callback_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: int32[], output: int32[]): void {
    values.forEach(async (value) => {
        output.push(value);
    });
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-for-of-over-for-each]: array iteration uses a forEach callback
 ──▶ main.ds:2:5
  │
1 │ function copy(values: int32[], output: int32[]): void {
2 │     values.forEach(async (value) => {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         output.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
4 │     });
  │     ^^
5 │ }
  │
"#,
        );
    }
}
