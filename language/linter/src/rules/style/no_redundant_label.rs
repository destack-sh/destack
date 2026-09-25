use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow labels on transfers to the target selected without a label.
    pub NO_REDUNDANT_LABEL {
        id: "no-redundant-label",
        summary: "Disallow labels on transfers to the target selected without a label",
        explanation: r#"
A labeled `break` or `continue` is redundant when the unlabeled statement selects the same control target.
Instead, you SHOULD omit the label and reserve labeled transfers for nonlocal control flow.
"#,
        example: {
            reported: r#"
outer: loop {
    break outer;
}
"#,
            accepted: r#"
outer: loop {
    break;
}
"#,
        },
        provenance: [Eslint("no-extra-label")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report labeled transfers whose selected target is already innermost.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect labeled transfers with a control target
    for (global, selected) in module.decisions.transfer_entries() {
        if global.module_id != module.id {
            return Err(ProviderError::internal(format!(
                "control transfer {global:?} belongs to another module"
            )));
        }
        let expression = global
            .local_id
            .try_into_typed::<dir::Expression>()
            .map_err(ProviderError::internal)?;
        let Some(label) = view.get(expression).transfer_label() else {
            continue;
        };
        let Some(target) = module.unlabeled_transfer_target(expression) else {
            continue;
        };

        // require the unlabeled transfer to select the same expression
        if selected != target.into_global(module.id) {
            continue;
        }

        // remove the label and its preceding separator
        let span = module.main_span(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("label does not change the control target", span);
        if let Some(suggestion) = suggestion(module, lint, expression, label, span)? {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Remove one redundant transfer label.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    label: dir::StringId,
    span: Span,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    if module.dir.strings.get(label) != module.source(span)? {
        return Err(ProviderError::internal(
            "labeled transfer main span does not contain its label",
        ));
    }
    let extent = module.source_extent(expression.into_any())?;
    let source = module.file(span.file)?.text().as_bytes();
    let mut start = span.start as usize;
    while start > extent.start as usize && matches!(source[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    let removal = Span::new(span.file, start as u32, span.end);
    if module.has_unretained_comment(removal, &[])? {
        return Ok(None);
    }

    // delete the authored label and its horizontal separator
    let patch = Patch::delete(removal);
    let suggestion = lint.fix("remove the redundant label", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Remove a break label that selects the innermost loop.
    #[test]
    fn test_removes_innermost_break_label() {
        let session = TestSession::dir(
            &NO_REDUNDANT_LABEL,
            r#"
outer: loop {
    break outer;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-redundant-label]: label does not change the control target
 ──▶ main.tspp:2:11
  │
1 │ outer: loop {
2 │     break outer;
  │           ^^^^^
3 │ }
  │

 = fix: remove the redundant label
--- a/main.tspp
+++ b/main.tspp

    1│ outer: loop {
-   2│     break outer;
+   2│     break;
    3│ }
"#,
        );
        session.assert_fixes(
            r#"
outer: loop {
    break;
}
"#,
        );
    }

    /// Remove a continue label that selects the innermost loop.
    #[test]
    fn test_removes_innermost_continue_label() {
        let session = TestSession::dir(
            &NO_REDUNDANT_LABEL,
            r#"
declare function ready(): boolean;

outer: while (ready()) {
    continue outer;
}
"#,
        );

        session.assert_fixes(
            r#"
declare function ready(): boolean;

outer: while (ready()) {
    continue;
}
"#,
        );
    }

    /// Accept a break label that selects an outer loop.
    #[test]
    fn test_accepts_outer_break_label() {
        let session = TestSession::dir(
            &NO_REDUNDANT_LABEL,
            r#"
outer: loop {
    loop {
        break outer;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a label on a break from a switch nested in its loop.
    #[test]
    fn test_accepts_label_across_switch() {
        let session = TestSession::dir(
            &NO_REDUNDANT_LABEL,
            r#"
outer: loop {
    switch (1) {
        default: break outer;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
