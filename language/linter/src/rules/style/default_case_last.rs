use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require the default switch case last.
    pub DEFAULT_CASE_LAST {
        id: "default-case-last",
        summary: "Require the default switch case last",
        explanation: r#"
A default case followed by another case makes the switch order harder to scan and permits unusual fallthrough from the fallback body.
Instead, you SHOULD place the default case last.
"#,
        example: {
            reported: r#"
function classify(value: int32): string {
    switch (value) {
        default:
            return "other";
        case 1:
            return "one";
    }
}
"#,
            accepted: r#"
function classify(value: int32): string {
    switch (value) {
        case 1:
            return "one";
        default:
            return "other";
    }
}
"#,
        },
        provenance: [Eslint("default-case-last")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report default switch cases followed by another case.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored switch case order
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Switch { cases, .. } = expression else {
            continue;
        };
        let Some((_, preceding)) = cases.split_last() else {
            continue;
        };
        let Some(default) = preceding
            .iter()
            .find(|case| matches!(view.get(**case).selector, dir::SwitchSelector::Default))
        else {
            continue;
        };

        let span = module.main_span(default.into_any())?;
        output.report(lint.diagnostic("default case precedes another case", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Highlight the misplaced default selector.
    #[test]
    fn test_reports_default_selector() {
        let session = TestSession::dir(
            &DEFAULT_CASE_LAST,
            r#"
function classify(value: int32): string {
    switch (value) {
        default:
            return "other";
        case 1:
            return "one";
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[default-case-last]: default case precedes another case
 ──▶ main.tspp:3:9
  │
1 │ function classify(value: int32): string {
2 │     switch (value) {
3 │         default:
  │         ^^^^^^^
4 │             return "other";
5 │         case 1:
  │
"#,
        );
    }

    /// Accept a switch without a default case.
    #[test]
    fn test_accepts_switch_without_default() {
        let session = TestSession::dir(
            &DEFAULT_CASE_LAST,
            r#"
function classify(value: int32): string {
    switch (value) {
        case 1:
            return "one";
    }
    return "other";
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
