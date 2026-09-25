use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer one condition over nested if statements without alternatives.
    pub NO_COLLAPSIBLE_IF {
        id: "no-collapsible-if",
        summary: "Prefer one condition over nested if statements without alternatives",
        explanation: r#"
Nested `if` statements without `else` branches require both conditions before running the same body.
Instead, you SHOULD join the conditions with `&&` in one `if` statement.
"#,
        example: {
            reported: r#"
declare function run(): void;

function runWhen(ready: boolean, enabled: boolean): void {
    if (ready) {
        if (enabled) {
            run();
        }
    }
}
"#,
            accepted: r#"
declare function run(): void;

function runWhen(ready: boolean, enabled: boolean): void {
    if (ready && enabled) {
        run();
    }
}
"#,
        },
        provenance: [Clippy("collapsible_if")],
        category: Style,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// Report nested if statements that share one unconditional body path.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect regular if statements without alternatives
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::If {
            form: dir::IfForm::If,
            condition: enclosing_condition,
            then_expression,
            else_expression: None,
        } = node
        else {
            continue;
        };
        let dir::Expression::Block(block) = view.get(*then_expression) else {
            continue;
        };
        let Some(nested) = view.get(*block).only_expression() else {
            continue;
        };
        let dir::Expression::If {
            form: dir::IfForm::If,
            condition: nested_condition,
            then_expression: nested_body,
            else_expression: None,
        } = view.get(nested)
        else {
            continue;
        };
        let nested_declarators = nested_condition
            .declarators()
            .map(dir::LocalNodeId::into_any);
        let enclosing_declarators = enclosing_condition
            .declarators()
            .map(dir::LocalNodeId::into_any);
        if module.shadows_bindings(nested_declarators, enclosing_declarators) {
            continue;
        }

        // report the nested conditional and offer one combined statement
        let span = module.source_extent(expression.into_any())?;
        let nested_span = module.source_extent(nested.into_any())?;
        let mut diagnostic = lint.diagnostic("nested if conditions can be joined", nested_span);
        if let Some(suggestion) = suggestion(
            module,
            lint,
            expression,
            *then_expression,
            nested,
            *nested_body,
            span,
        )? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one combined if statement.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    enclosing: dir::LocalNodeId<dir::Expression>,
    enclosing_body: dir::LocalNodeId<dir::Expression>,
    nested: dir::LocalNodeId<dir::Expression>,
    nested_body: dir::LocalNodeId<dir::Expression>,
    extent: Span,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let nested_extent = module.source_extent(nested.into_any())?;
    let enclosing_body_extent = module.source_extent(enclosing_body.into_any())?;
    if module.has_unretained_comment(enclosing_body_extent, &[nested_extent])? {
        return Ok(None);
    }

    // retain both authored conditions
    let Some(enclosing_condition) = condition_source(module, enclosing, enclosing_body)? else {
        return Ok(None);
    };
    let Some(nested_condition) = condition_source(module, nested, nested_body)? else {
        return Ok(None);
    };

    // dedent the surviving body to the enclosing statement column
    let enclosing_position = module.file(extent.file)?.get_position(extent.start);
    let nested_body_extent = module.source_extent(nested_body.into_any())?;
    let nested_position = module.file(extent.file)?.get_position(nested_extent.start);
    let (Some((_, enclosing_column)), Some((_, nested_column))) =
        (enclosing_position, nested_position)
    else {
        return Err(ProviderError::internal(
            "collapsible if extent is outside its authored source file",
        ));
    };
    let indentation = nested_column
        .checked_sub(enclosing_column)
        .ok_or_else(|| ProviderError::internal("nested if begins before its enclosing if"))?;
    let Some(body) = module.dedent_source(nested_body_extent, indentation)? else {
        return Ok(None);
    };

    // replace both conditionals with one statement
    let replacement = format!("if ({enclosing_condition} && {nested_condition}) {body}");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("join the nested conditions", patch)?;

    Ok(Some(suggestion))
}

/// Return one if statement's authored condition when its parentheses can be replaced.
fn condition_source(
    module: &DirModule<'_>,
    expression: dir::LocalNodeId<dir::Expression>,
    body: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<String>, ProviderError> {
    let dir::Expression::If { condition, .. } = module.view().get(expression) else {
        return Err(ProviderError::internal(
            "collapsible condition source does not belong to an if expression",
        ));
    };

    // extract text inside the authored condition parentheses
    let keyword = module.main_span(expression.into_any())?;
    let body = module.source_extent(body.into_any())?;
    let span = Span::new(keyword.file, keyword.end, body.start);
    let source = module.source(span)?.trim();
    let Some(source) = source
        .strip_prefix('(')
        .and_then(|source| source.strip_suffix(')'))
    else {
        if module.has_unretained_comment(span, &[])? {
            return Ok(None);
        }

        return Err(ProviderError::internal(
            "if condition has no authored parentheses",
        ));
    };

    // retain grouping around one expression weaker than logical conjunction
    let source = source.trim();
    let source = if condition.as_expression().is_some_and(|expression| {
        module.view().get(expression).precedence() < dir::OperatorPrecedence::LogicalAnd
    }) {
        format!("({source})")
    } else {
        source.to_string()
    };

    Ok(Some(source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Accept a nested if with an alternative branch.
    #[test]
    fn test_accepts_nested_alternative() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_IF,
            r#"
declare function run(): void;

function runWhen(ready: boolean, enabled: boolean): void {
    if (ready) {
        if (enabled) {
            run();
        } else {
            return;
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve grouping around disjunctions in both conditions.
    #[test]
    fn test_groups_disjunctions() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_IF,
            r#"
declare function run(): void;

function runWhen(
    ready: boolean,
    fallback: boolean,
    enabled: boolean,
    forced: boolean,
): void {
    if (ready || fallback) {
        if (enabled || forced) {
            run();
        }
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function run(): void;

function runWhen(
    ready: boolean,
    fallback: boolean,
    enabled: boolean,
    forced: boolean,
): void {
    if ((ready || fallback) && (enabled || forced)) {
        run();
    }
}
"#,
        );
    }

    /// Preserve a condition binding across the joined nested condition.
    #[test]
    fn test_preserves_condition_binding_scope() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_IF,
            r#"
declare function run(): void;

function runWhen(user: { enabled: boolean } | null, ready: boolean): void {
    if (let { enabled } = user) {
        if (ready && enabled) {
            run();
        }
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
declare function run(): void;

function runWhen(user: { enabled: boolean } | null, ready: boolean): void {
    if (let { enabled } = user && ready && enabled) {
        run();
    }
}
"#,
        );
    }

    /// Accept nested conditions that shadow one binding name.
    #[test]
    fn test_accepts_shadowed_condition_binding() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_IF,
            r#"
function runWhen(
    first: { ready: boolean } | null,
    second: { ready: boolean } | null,
): void {
    if (let { ready } = first) {
        if (let { ready } = second) {
            ready;
        }
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report without a suggestion when the enclosing block owns a comment.
    #[test]
    fn test_preserves_enclosing_comment() {
        let session = TestSession::dir(
            &NO_COLLAPSIBLE_IF,
            r#"
declare function run(): void;

function runWhen(ready: boolean, enabled: boolean): void {
    if (ready) {
        // explain the second condition
        if (enabled) {
            run();
        }
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-collapsible-if]: nested if conditions can be joined
  ──▶ main.tspp:6:9
   │
 4 │     if (ready) {
 5 │         // explain the second condition
 6 │         if (enabled) {
   │         ^^^^^^^^^^^^^^
 7 │             run();
   │             ^^^^^^
 8 │         }
   │         ^
 9 │     }
10 │ }
   │
"#,
        );
    }
}
