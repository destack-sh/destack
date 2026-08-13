use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer loop over a while loop with a constant true condition.
    pub PREFER_LOOP {
        id: "prefer-loop",
        summary: "Prefer loop over a while loop with a constant true condition",
        explanation: r#"
A `while` loop with a constant true condition disguises unconditional iteration as a conditional loop.
Instead, you SHOULD use `loop` to state that only control transfers inside the body can stop iteration.
"#,
        example: {
            reported: r#"
while (true) {
    // wait for an external interrupt
}
"#,
            accepted: r#"
loop {
    // wait for an external interrupt
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report regular while loops whose checked condition is constant true.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect regular while loops with a constant true condition
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::While {
            form: dir::WhileForm::While,
            condition,
            body,
            ..
        } = node
        else {
            continue;
        };
        if module.scalar_constant(*condition)? != Some(dir::ScalarLiteral::Boolean(true)) {
            continue;
        }

        // replace the conditional header when every comment is retained
        let span = module.source_extent(condition.into_any())?;
        let mut diagnostic = lint.diagnostic("while loop has a constant true condition", span);
        if let Some(suggestion) = suggestion(module, lint, expression, *body)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Replace one constant true while header with loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    body: dir::LocalNodeId<dir::Block>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let loop_extent = module.source_extent(expression.into_any())?;
    let body_extent = module.source_extent(body.into_any())?;
    let header = Span::new(loop_extent.file, loop_extent.start, body_extent.start);
    if module.has_unretained_comment(header, &[])? {
        return Ok(None);
    }

    // replace the complete conditional header
    let mut file = FilePatch::new(header.file);
    let keyword = module.main_span(expression.into_any())?;
    let conditional_header = Span::new(keyword.file, keyword.start, body_extent.start);
    file.replace(conditional_header, "loop ");
    file.sort();
    let suggestion = lint.fix("use an unconditional loop", file)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace a constant true while loop.
    #[test]
    fn test_replaces_true_while() {
        let session = TestSession::dir(&PREFER_LOOP, PREFER_LOOP.example.reported());

        session.assert_diagnostics(
            r#"
warning[prefer-loop]: while loop has a constant true condition
 ──▶ main.ds:1:8
  │
1 │ while (true) {
  │        ^^^^
2 │     // wait for an external interrupt
3 │ }
  │

 = fix: use an unconditional loop
--- a/main.ds
+++ b/main.ds

-   1│ while (true) {
+   1│ loop {
"#,
        );
        session.assert_fixes(PREFER_LOOP.example.accepted());
    }

    /// Accept a while loop whose condition is not constant.
    #[test]
    fn test_accepts_runtime_condition() {
        let session = TestSession::dir(
            &PREFER_LOOP,
            r#"
declare function ready(): boolean;

while (ready()) {
    // wait until work is complete
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report but retain a comment inside the constant condition.
    #[test]
    fn test_retains_condition_comment() {
        let session = TestSession::dir(
            &PREFER_LOOP,
            r#"
while (/* forever */ true) {
    // run forever
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-loop]: while loop has a constant true condition
 ──▶ main.ds:1:22
  │
1 │ while (/* forever */ true) {
  │                      ^^^^
2 │     // run forever
3 │ }
  │
"#,
        );
    }

    /// Accept a do-while loop because its authored form has different sequencing.
    #[test]
    fn test_accepts_do_while() {
        let session = TestSession::dir(
            &PREFER_LOOP,
            r#"
declare function ready(): boolean;

do {
    // run once before checking
} while (ready());
"#,
        );

        session.assert_no_diagnostics();
    }
}
