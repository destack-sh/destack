use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer for-of over sequential forEach callbacks.
    pub PREFER_FOR_OF_OVER_FOR_EACH {
        id: "prefer-for-of-over-for-each",
        summary: "Prefer for-of over sequential forEach callbacks",
        explanation: r#"
A `forEach` callback introduces a function boundary for ordinary sequential iteration.
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
        provenance: [Unicorn("no-for-each")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report canonical sequential forEach calls.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let occurrences = module.flows.binding_occurrences().collect::<Vec<_>>();
    let mut output = LintOutput::default();

    // inspect canonical Array and Iterator forEach calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        let Some(member) = module.language_member(expression)? else {
            continue;
        };
        if member != dir::LanguageItem::Array.member("forEach")
            && member != dir::LanguageItem::Iterator.member("forEach")
        {
            continue;
        }

        // report the callback iteration and offer a loop when control flow permits
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("iteration uses a forEach callback", span);
        if !call.is_optional()
            && let Some(suggestion) = suggestion(
                module,
                lint,
                expression,
                call.receiver,
                call.arguments,
                member.owner,
                &occurrences,
            )?
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
    owner: dir::LanguageItem,
    occurrences: &[dir::BindingOccurrence],
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let view = module.view();

    // require the call to be a direct statement with one lambda argument
    if !module.is_discarded_expression(expression) {
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
    let parameters = lambda.signature.parameters.as_slice();
    if parameters.len() > 2 {
        return Ok(None);
    }
    let Some(body) = lambda.body else {
        return Ok(None);
    };
    if !matches!(view.get(body), dir::Expression::Block(_)) {
        return Ok(None);
    }

    // preserve only parameters that map directly to loop binding patterns
    let mut parameter_spans = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let Some(span) = parameter_pattern_span(module, *parameter)? else {
            return Ok(None);
        };

        parameter_spans.push(span);
    }

    // omit an index parameter that flow proves unused
    let is_index_used = parameters.get(1).is_some_and(|parameter| {
        !module
            .declared_binding_uses(parameter.into_any(), occurrences)
            .is_empty()
    });
    let retained_parameter_count = if parameters.len() == 2 && !is_index_used {
        1
    } else {
        parameters.len()
    };
    let retained_parameters = &parameters[..retained_parameter_count];
    let retained_parameter_spans = &parameter_spans[..retained_parameter_count];

    // reject callback control whose target would change after inlining
    if module.uses_enclosing_control(body.into_any())? {
        return Ok(None);
    }

    // retain comments only when they remain inside the parameter, receiver, or body
    let replacement_extent = module.statement_span(expression)?;
    let receiver_extent = module.source_extent(receiver.into_any())?;
    let body = module.source_extent(body.into_any())?;
    let mut retained = retained_parameter_spans.to_vec();
    retained.extend([receiver_extent, body]);
    if module.has_unretained_comment(replacement_extent, &retained)? {
        return Ok(None);
    }

    // preserve mutable callback parameters as mutable loop bindings
    let keyword = match parameter_binding_keyword(module, retained_parameters, occurrences) {
        dir::BindingKeyword::Const => "const",
        dir::BindingKeyword::Let => "let",
    };

    // compose the equivalent iteration binding and source
    let parameters = retained_parameter_spans
        .iter()
        .map(|span| module.source(*span))
        .collect::<Result<Vec<_>, _>>()?;
    let receiver = module.expression_source(receiver, dir::OperatorPrecedence::Lowest)?;
    let (binding, iterator) = match parameters.as_slice() {
        [] => ("_".to_owned(), receiver.into_owned()),
        [value] => ((*value).to_owned(), receiver.into_owned()),
        [value, index] => {
            let iterator = match owner {
                dir::LanguageItem::Array => format!("{receiver}.entries()"),
                dir::LanguageItem::Iterator => format!("{receiver}.enumerate()"),
                _ => return Ok(None),
            };

            (format!("({index}, {value})"), iterator)
        }
        _ => return Ok(None),
    };
    let body = module.source(body)?;
    let replacement = format!("for ({keyword} {binding} of {iterator}) {body}");
    let patch = Patch::replace(replacement_extent, replacement);
    let suggestion = lint.suggestion("use a for-of loop", patch)?;

    Ok(Some(suggestion))
}

/// Return one callback parameter's direct loop binding span.
fn parameter_pattern_span(
    module: &DirModule<'_>,
    parameter: dir::LocalNodeId<dir::Parameter>,
) -> Result<Option<Span>, ProviderError> {
    let span = match module.view().get(parameter) {
        dir::Parameter::Named {
            declared_type: None,
            default: None,
            is_optional: false,
            ..
        } => module.main_span(parameter.into_any())?,
        dir::Parameter::Pattern {
            pattern,
            declared_type: None,
            default: None,
            is_optional: false,
        } => module.source_extent(pattern.into_any())?,
        _ => return Ok(None),
    };

    Ok(Some(span))
}

/// Return the loop keyword required by callback parameter storage.
fn parameter_binding_keyword(
    module: &DirModule<'_>,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    occurrences: &[dir::BindingOccurrence],
) -> dir::BindingKeyword {
    let is_mutable = parameters.iter().any(|parameter| {
        module
            .declared_binding_uses(parameter.into_any(), occurrences)
            .may_mutate()
    });

    match is_mutable {
        true => dir::BindingKeyword::Let,
        false => dir::BindingKeyword::Const,
    }
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
warning[prefer-for-of-over-for-each]: iteration uses a forEach callback
 ──▶ main.tspp:2:5
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
--- a/main.tspp
+++ b/main.tspp

    1│ function copy(values: int32[], output: int32[]): void {
-   2│     values.forEach((value) => {
+   2│     for (const value of values) {
    3│         output.push(value);
-   4│     });
+   4│     }
    5│ }
"#,
        );
        session.assert_suggestions(
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Bind a wildcard when the callback ignores each value.
    #[test]
    fn test_replaces_parameterless_array_for_each() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
declare function visit(): void;

function visitAll(values: int32[]): void {
    values.forEach(() => {
        visit();
    });
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function visit(): void;

function visitAll(values: int32[]): void {
    for (const _ of values) {
        visit();
    }
}
"#,
        );
    }

    /// Bind the Array.forEach index from Array.entries.
    #[test]
    fn test_replaces_indexed_array_for_each() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: int32[], output: isize[]): void {
    values.forEach((value, index) => {
        value;
        output.push(index);
    });
}
"#,
        );

        session.assert_suggestions(
            r#"
function copy(values: int32[], output: isize[]): void {
    for (const (index, value) of values.entries()) {
        value;
        output.push(index);
    }
}
"#,
        );
    }

    /// Omit an unused Array.forEach index from the replacement.
    #[test]
    fn test_replaces_unused_array_for_each_index() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: int32[], output: int32[]): void {
    values.forEach((value, index) => {
        output.push(value);
    });
}
"#,
        );

        session.assert_suggestions(
            r#"
function copy(values: int32[], output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
"#,
        );
    }

    /// Preserve callback parameter mutation with a mutable loop binding.
    #[test]
    fn test_replaces_mutated_callback_parameter() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
declare function mutate(value: &int32): void;

function increment(values: int32[], output: int32[]): void {
    values.forEach((value) => {
        mutate(&value);
        output.push(value);
    });
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function mutate(value: &int32): void;

function increment(values: int32[], output: int32[]): void {
    for (let value of values) {
        mutate(&value);
        output.push(value);
    }
}
"#,
        );
    }

    /// Preserve a destructured callback parameter as the loop pattern.
    #[test]
    fn test_replaces_destructured_callback_parameter() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: { value: int32 }[], output: int32[]): void {
    values.forEach(({ value }) => {
        output.push(value);
    });
}
"#,
        );

        session.assert_suggestions(
            r#"
function copy(values: { value: int32 }[], output: int32[]): void {
    for (const { value } of values) {
        output.push(value);
    }
}
"#,
        );
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
warning[prefer-for-of-over-for-each]: iteration uses a forEach callback
 ──▶ main.tspp:2:5
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

    /// Report a defaulted callback parameter without changing its binding semantics.
    #[test]
    fn test_reports_defaulted_callback_without_suggestion() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
function copy(values: int32[], output: int32[]): void {
    values.forEach((value = 0) => {
        output.push(value);
    });
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-for-of-over-for-each]: iteration uses a forEach callback
 ──▶ main.tspp:2:5
  │
1 │ function copy(values: int32[], output: int32[]): void {
2 │     values.forEach((value = 0) => {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         output.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
4 │     });
  │     ^^
5 │ }
  │
"#,
        );
    }

    /// Replace a direct one-parameter Iterator.forEach statement.
    #[test]
    fn test_replaces_iterator_for_each() {
        let session = TestSession::dir(
            &PREFER_FOR_OF_OVER_FOR_EACH,
            r#"
import { Iterator } from "tspp:iter";

function copy(values: Iterator<int32>, output: int32[]): void {
    values.forEach((value) => {
        output.push(value);
    });
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Iterator } from "tspp:iter";

function copy(values: Iterator<int32>, output: int32[]): void {
    for (const value of values) {
        output.push(value);
    }
}
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
warning[prefer-for-of-over-for-each]: iteration uses a forEach callback
 ──▶ main.tspp:2:5
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
