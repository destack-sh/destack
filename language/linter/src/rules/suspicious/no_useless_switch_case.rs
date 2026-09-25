use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow empty switch cases adjacent to the default case.
    pub NO_USELESS_SWITCH_CASE {
        id: "no-useless-switch-case",
        summary: "Disallow empty switch cases adjacent to the default case",
        explanation: r#"
An empty case adjacent to `default` reaches the same statements already selected for every unmatched value.
Instead, you SHOULD remove the redundant case selector.
"#,
        example: {
            reported: r#"
function classify(value: int32): void {
    switch (value) {
        case 0:
        default:
            value;
    }
}
"#,
            accepted: r#"
function classify(value: int32): void {
    switch (value) {
        default:
            value;
    }
}
"#,
        },
        provenance: [Unicorn("no-useless-switch-case")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report case selectors made redundant by an adjacent default selector.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect each default and its contiguous fallthrough selectors
    for (_, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Switch { cases, .. } = node else {
            continue;
        };
        let Some(default) = cases
            .iter()
            .position(|case| matches!(view.get(*case).selector, dir::SwitchSelector::Default))
        else {
            continue;
        };
        let mut redundant = Vec::new();

        // collect empty case bodies that fall forward into default
        for case in cases[..default].iter().rev() {
            let case_node = view.get(*case);
            if !view.get(case_node.body).is_empty() {
                break;
            }
            if !matches!(case_node.selector, dir::SwitchSelector::Case(_)) {
                continue;
            }
            redundant.push(*case);
        }
        redundant.reverse();

        // collect cases reached by an empty default body
        if view.get(view.get(cases[default]).body).is_empty() {
            for case in &cases[default + 1..] {
                let case_node = view.get(*case);
                if !matches!(case_node.selector, dir::SwitchSelector::Case(_)) {
                    continue;
                }
                redundant.push(*case);
                if !view.get(case_node.body).is_empty() {
                    break;
                }
            }
        }

        // report every redundant selector in source order
        for case in redundant {
            let span = module.main_span(case.into_any())?;
            output.report(lint.diagnostic("case shares the default behavior", span));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an empty case immediately before default.
    #[test]
    fn test_reports_case_before_default() {
        let session = TestSession::dir(
            &NO_USELESS_SWITCH_CASE,
            r#"
function classify(value: int32): void {
    switch (value) {
        case 0:
        default:
            value;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-switch-case]: case shares the default behavior
 ──▶ main.tspp:3:9
  │
1 │ function classify(value: int32): void {
2 │     switch (value) {
3 │         case 0:
  │         ^^^^^^
4 │         default:
5 │             value;
  │
"#,
        );
    }

    /// Report a case immediately after an empty default.
    #[test]
    fn test_reports_case_after_default() {
        let session = TestSession::dir(
            &NO_USELESS_SWITCH_CASE,
            r#"
function classify(value: int32): void {
    switch (value) {
        default:
        case 0:
            value;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-useless-switch-case]: case shares the default behavior
 ──▶ main.tspp:4:9
  │
2 │     switch (value) {
3 │         default:
4 │         case 0:
  │         ^^^^^^
5 │             value;
6 │     }
  │
"#,
        );
    }

    /// Accept adjacent non-default cases sharing one body.
    #[test]
    fn test_accepts_shared_case_body() {
        let session = TestSession::dir(
            &NO_USELESS_SWITCH_CASE,
            r#"
function classify(value: int32): void {
    switch (value) {
        case 0:
        case 1:
            value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
