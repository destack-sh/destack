use tspp_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow nested blocks with no scoping effect.
    pub NO_LONE_BLOCK {
        id: "no-lone-block",
        summary: "Disallow nested blocks with no scoping effect",
        explanation: r#"
A standalone block without scoped declarations adds nesting without changing the lifetime or visibility of any value.
Instead, you SHOULD remove the redundant braces and keep its statements in the enclosing block.
"#,
        example: {
            reported: r#"
declare function work(): void;

function run(): void {
    {
        work();
    }
}
"#,
            accepted: r#"
declare function work(): void;

function run(): void {
    work();
}
"#,
        },
        provenance: [Eslint("no-lone-blocks")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report standalone blocks that introduce no scoped declarations.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect explicit block expressions used directly as statements
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Block(block) = node else {
            continue;
        };
        let block = view.get(*block);
        let is_statement = view
            .get_parent_any(expression.into_any())
            .is_some_and(|parent| parent.ty == dir::NodeType::Block);
        if block.form != dir::BlockForm::Explicit || block.is_empty() || !is_statement {
            continue;
        }

        // retain blocks that delimit bindings or declarations
        let has_scoped_declaration = block.iter_expressions().any(|expression| {
            matches!(
                view.get(expression),
                dir::Expression::Declaration(_)
                    | dir::Expression::Let { .. }
                    | dir::Expression::LetElse { .. }
                    | dir::Expression::Using { .. }
            )
        });
        if has_scoped_declaration {
            continue;
        }

        let span = module.source_extent(expression.into_any())?;
        output.report(lint.diagnostic("nested block has no scoping effect", span));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a standalone block without declarations.
    #[test]
    fn test_reports_lone_block() {
        let session = TestSession::dir(
            &NO_LONE_BLOCK,
            r#"
declare function work(): void;

function run(): void {
    {
        work();
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-lone-block]: nested block has no scoping effect
 ──▶ main.tspp:4:5
  │
2 │
3 │ function run(): void {
4 │     {
  │     ^
5 │         work();
  │         ^^^^^^^
6 │     }
  │     ^
7 │ }
  │
"#,
        );
    }

    /// Accept a block that limits a binding's visibility.
    #[test]
    fn test_accepts_scoped_binding() {
        let session = TestSession::dir(
            &NO_LONE_BLOCK,
            r#"
function run(): void {
    {
        const value = 1;
        value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a block used as a conditional body.
    #[test]
    fn test_accepts_control_body() {
        let session = TestSession::dir(
            &NO_LONE_BLOCK,
            r#"
declare function ready(): boolean;

if (ready()) {
    // wait
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an explicit do expression because it produces a value.
    #[test]
    fn test_accepts_do_expression() {
        let session = TestSession::dir(
            &NO_LONE_BLOCK,
            r#"
const value = do {
    1
};
"#,
        );

        session.assert_no_diagnostics();
    }
}
