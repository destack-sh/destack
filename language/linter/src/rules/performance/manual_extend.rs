use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer extend over loops that append every iterated value.
    pub MANUAL_EXTEND {
        id: "manual-extend",
        summary: "Prefer extend over loops that append every iterated value",
        explanation: r#"
A loop that only pushes each source value repeats the collection's bulk append operation.
Instead, you SHOULD call `extend` with the source iterable.
"#,
        example: {
            reported: r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value);
    }
}
"#,
            accepted: r#"
function append(target: int32[], source: int32[]): void {
    target.extend(source);
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report for-of loops whose sole action pushes the bound value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect synchronous for-of loops with one direct binding
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(loop_) = module.for_of(expression) else {
            continue;
        };
        if loop_.asynchrony != dir::Asynchrony::Sync {
            continue;
        }
        let dir::ForEachBinding::Pattern {
            pattern,
            keyword: Some(_),
        } = loop_.binding
        else {
            continue;
        };
        if !matches!(
            view.get(*pattern),
            dir::Pattern::Binding { pattern: None, .. }
        ) {
            continue;
        }
        let Some(action) = view.get(loop_.body).only_expression() else {
            continue;
        };

        // require one canonical push of the exact loop binding
        let Some(push) = module.member_call(action) else {
            continue;
        };
        if push.is_optional() {
            continue;
        }
        if module.language_member(action)? != Some(dir::LanguageItem::Array.member("push")) {
            continue;
        }
        let [argument] = push.arguments else {
            continue;
        };
        let dir::Argument::Positional { value } = view.get(*argument) else {
            continue;
        };
        let binding = module.declaration_symbol(*pattern)?;
        if module.selected_symbol(*value)? != Some(binding)
            || !module.is_repeatable_expression(push.receiver)?
        {
            continue;
        }
        let target = module.access_resolution(push.receiver);
        let source = module.access_resolution(loop_.iterator);
        if target.is_some() && target == source {
            continue;
        }

        // replace the loop with the canonical bulk append
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("loop pushes every source value", span);
        if let Some(suggestion) =
            suggestion(module, lint, expression, push.receiver, loop_.iterator)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one extend call from an append loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::Expression>,
    source: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let target_extent = module.source_extent(target.into_any())?;
    let source_extent = module.source_extent(source.into_any())?;
    if module.has_unretained_comment(extent, &[target_extent, source_extent])? {
        return Ok(None);
    }

    // preserve authored target and source expressions
    let target = module.expression_source(target, dir::OperatorPrecedence::Postfix)?;
    let source = module.source(source_extent)?;
    let replacement = format!("{target}.extend({source});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("append the iterable in one operation", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an Array push loop with extend.
    #[test]
    fn test_replaces_array_push_loop() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-extend]: loop pushes every source value
 ──▶ main.ds:2:5
  │
1 │ function append(target: int32[], source: int32[]): void {
2 │     for (const value of source) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │

 = suggestion: append the iterable in one operation (requires review)
--- a/main.ds
+++ b/main.ds

    1│ function append(target: int32[], source: int32[]): void {
-   2│     for (const value of source) {
-   3│         target.push(value);
-   4│     }
+   2│     target.extend(source);
"#,
        );
        session.assert_suggestions(
            r#"
function append(target: int32[], source: int32[]): void {
    target.extend(source);
}
"#,
        );
    }

    /// Accept a loop that transforms values before pushing them.
    #[test]
    fn test_accepts_transformed_push() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value + 1);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with another action.
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value);
        target.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined push method.
    #[test]
    fn test_accepts_user_push() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
class Values {
    push(value: int32): void {
        // intentionally empty
    }
}

function append(target: Values, source: int32[]): void {
    for (const value of source) {
        target.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
