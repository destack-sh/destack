use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{Applicability, DiagnosticSuggestion, FilePatch, PatchSet};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow debugger statements.
    pub NO_DEBUGGER {
        id: "no-debugger",
        summary: "Disallow debugger statements",
        explanation: "The `debugger` statement interrupts execution only when an attached debugger honors it and otherwise has no useful runtime effect. It is normally an accidental development artifact and should not remain in checked source.",
        example: {
            reported: r#"
if (true) debugger;
"#,
            accepted: r#"
if (true) {}
"#,
        },
        category: Suspicious,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report debugger statements.
fn check(module: &DirModule, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // report every visible debugger expression
    for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
        // skip other expressions
        if !matches!(expression, dir::Expression::Debugger) {
            continue;
        }

        let span = module.span(expression_id.into_any())?;
        let suggestion = suggest_removal(module, view, expression_id)?;
        let diagnostic = lint
            .diagnostic("`debugger` statement is not allowed", span)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build an automatic debugger removal.
fn suggest_removal(
    module: &DirModule,
    view: dir::View<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let replacement = match view.get_parent_for(expression_id) {
        // root statements can disappear completely
        None => "",
        // classify the statement's structural position
        Some(parent) => match parent.ty {
            // preserve required implicit control bodies
            dir::NodeType::Block => {
                let block_id = dir::LocalNodeId::<dir::Block>::new(parent.id);
                let block = view.get(block_id);
                let is_implicit_body = block.form == dir::BlockForm::Implicit
                    && block.only_expression() == Some(expression_id);

                // remove statements from explicit or multi-statement blocks
                if !is_implicit_body {
                    ""
                }
                // preserve required bodies except switch case statements
                else {
                    let owner = view.get_parent_for(block_id).ok_or_else(|| {
                        ProviderError::internal(format!(
                            "debugger block {} in module {:?} has no DIR parent",
                            block_id.id, module.id
                        ))
                    })?;

                    match owner.ty {
                        dir::NodeType::SwitchCase => "",
                        _ => "{}",
                    }
                }
            }
            // preserve required match arm expressions
            dir::NodeType::MatchArm => "{}",
            // catch and finally clauses require bodies
            dir::NodeType::Catch => "{}",
            dir::NodeType::Expression
                if matches!(
                    view.get(dir::LocalNodeId::<dir::Expression>::new(parent.id)),
                    dir::Expression::Try {
                        finally: Some(finally),
                        ..
                    } if *finally == expression_id
                ) =>
            {
                "{}"
            }
            _ => {
                return Err(ProviderError::internal(format!(
                    "debugger statement {} in module {:?} has invalid DIR parent {parent:?}",
                    expression_id.id, module.id
                )));
            }
        },
    };

    // replace the complete statement
    let span = if replacement.is_empty() {
        module.statement_removal_span(expression_id)?
    } else {
        module.statement_span(expression_id)?
    };
    let mut file_patch = FilePatch::new(span.file);
    file_patch.replace(span, replacement);
    let patches = PatchSet::single(file_patch);
    let suggestion = DiagnosticSuggestion::new(
        "remove the debugger statement",
        patches,
        Applicability::Automatic,
    );

    Ok(suggestion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report the complete debugger diagnostic and source suggestion.
    #[test]
    fn test_reports_debugger_statement() {
        let session = TestSession::new(&NO_DEBUGGER, NO_DEBUGGER.example.reported());

        session.assert_diagnostics(
            r#"
warning[no-debugger]: `debugger` statement is not allowed
 ──▶ main.ds:1:11
  │
1 │ if (true) debugger;
  │           ^^^^^^^^
  │

 = help: remove the debugger statement (machine-applicable)
--- a/main.ds
+++ b/main.ds

-   1│ if (true) debugger;
+   1│ if (true) {}
"#,
        );

        session.assert_fixes(NO_DEBUGGER.example.accepted());
    }

    /// Suppress the lint through its canonical id.
    #[test]
    fn test_allows_debugger_by_id() {
        let session = TestSession::new(
            &NO_DEBUGGER,
            r#"@allow("no-debugger")
debugger;
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Remove the debugger statement without changing surrounding source.
    #[test]
    fn test_removes_debugger_statement() {
        let session = TestSession::new(
            &NO_DEBUGGER,
            r#"const before = 1;
debugger;
const after = 2;
"#,
        );

        session.assert_fixes(
            r#"const before = 1;

const after = 2;
"#,
        );
    }

    /// Preserve the required body of a catch clause.
    #[test]
    fn test_replaces_catch_body() {
        let session = TestSession::new(
            &NO_DEBUGGER,
            r#"try {} catch (error) debugger;
"#,
        );

        session.assert_fixes(
            r#"try {} catch (error) {}
"#,
        );
    }

    /// Preserve the required body of a finally clause.
    #[test]
    fn test_replaces_finally_body() {
        let session = TestSession::new(
            &NO_DEBUGGER,
            r#"try {} finally debugger;
"#,
        );

        session.assert_fixes(
            r#"try {} finally {}
"#,
        );
    }

    /// Remove the debugger statement from a switch case.
    #[test]
    fn test_removes_switch_case_statement() {
        let session = TestSession::new(
            &NO_DEBUGGER,
            r#"switch (1) {
    case 1: debugger;
}
"#,
        );

        session.assert_fixes(
            r#"switch (1) {
    case 1:
}
"#,
        );
    }

    /// Preserve the required value of a direct match arm.
    #[test]
    fn test_replaces_match_arm() {
        let session = TestSession::new(
            &NO_DEBUGGER,
            r#"match (undefined) {
    _ => debugger
}
"#,
        );

        session.assert_fixes(
            r#"match (undefined) {
    _ => {}
}
"#,
        );
    }

    /// Ignore property declarations and accesses named debugger.
    #[test]
    fn test_ignores_debugger_property() {
        let session = TestSession::new(
            &NO_DEBUGGER,
            r#"const value = { debugger: true };
value.debugger;
"#,
        );

        session.assert_no_diagnostics();
    }
}
