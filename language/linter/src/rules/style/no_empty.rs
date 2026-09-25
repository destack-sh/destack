use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow uncommented empty block statements.
    pub NO_EMPTY {
        id: "no-empty",
        summary: "Disallow uncommented empty block statements",
        explanation: r#"
An empty control-flow block and an accidentally unfinished block have the same source shape.
Instead, you SHOULD remove the construct, implement its body, or explain the intentional empty body with a comment.
"#,
        example: {
            reported: r#"
declare function ready(): boolean;

if (ready()) {}
"#,
            accepted: r#"
declare function ready(): boolean;

if (ready()) {
    // no action is required while ready
}
"#,
        },
        provenance: [Eslint("no-empty")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report explicit empty control-flow blocks without comments.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect authored statement blocks outside function bodies
    for (block, node) in view.iter_nodes::<dir::Block>() {
        if node.form != dir::BlockForm::Explicit || !node.is_empty() {
            continue;
        }
        let body = view
            .get_parent_any(block.into_any())
            .and_then(|parent| parent.try_into_typed::<dir::Expression>().ok())
            .filter(|expression| matches!(view.get(*expression), dir::Expression::Block(_)));
        if let Some(body) = body
            && let Some(owner) = view.get_parent_any(body.into_any())
        {
            // leave callable bodies to the dedicated function rule
            if module.callable_body(owner) == Some(body) {
                continue;
            }

            // leave initialization bodies to the dedicated static block rule
            if owner.try_into_typed::<dir::Member>().is_ok_and(|member| {
                matches!(
                    view.get(member),
                    dir::Member::StaticBlock { body: member_body }
                        | dir::Member::ConstBlock { body: member_body }
                        if *member_body == body
                )
            }) {
                continue;
            }
        }

        // accept comments as the explicit explanation for an empty body
        let span = module.source_extent(block.into_any())?;
        if module.has_unretained_comment(span, &[])? {
            continue;
        }
        output.report(lint.diagnostic("block is empty", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an empty conditional body.
    #[test]
    fn test_reports_empty_conditional() {
        let session = TestSession::dir(
            &NO_EMPTY,
            r#"
declare function ready(): boolean;

if (ready()) {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty]: block is empty
 ──▶ main.tspp:3:14
  │
1 │ declare function ready(): boolean;
2 │
3 │ if (ready()) {}
  │              ^^
  │
"#,
        );
    }

    /// Accept an explanatory comment inside an empty block.
    #[test]
    fn test_accepts_commented_block() {
        let session = TestSession::dir(
            &NO_EMPTY,
            r#"
declare function ready(): boolean;

if (ready()) {
    // no action is required while ready
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Leave empty function bodies to the dedicated function rule.
    #[test]
    fn test_accepts_function_body() {
        let session = TestSession::dir(
            &NO_EMPTY,
            r#"
function initialize(): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Leave empty static bodies to the dedicated initialization rule.
    #[test]
    fn test_accepts_static_initialization_body() {
        let session = TestSession::dir(
            &NO_EMPTY,
            r#"
class Registry {
    static {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report an empty loop body.
    #[test]
    fn test_reports_empty_loop() {
        let session = TestSession::dir(
            &NO_EMPTY,
            r#"
declare function ready(): boolean;

while (ready()) {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-empty]: block is empty
 ──▶ main.tspp:3:17
  │
1 │ declare function ready(): boolean;
2 │
3 │ while (ready()) {}
  │                 ^^
  │
"#,
        );
    }
}
