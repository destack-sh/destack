use destack_ast::{Block, BlockFormat};
use destack_source::{FileId, ModuleId};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow empty block statements.
    ///
    /// Empty blocks are often a sign of incomplete code or accidental deletion.
    /// If intentional, add a comment explaining why the block is empty.
    #[lint(
        id = "no-empty",
        code = "LC002",
        category = Suspicious,
        level = Ast
    )]
    pub NoEmpty,
    "Disallow empty block statements"
}

impl LintRule for NoEmpty {
    fn meta(&self) -> &'static crate::LintMeta {
        NoEmpty::meta()
    }

    fn check_module_ast(
        &self,
        file_id: FileId,
        _module_id: ModuleId,
        severity: LintSeverity,
        ctx: &mut LintModuleAstContext,
    ) {
        ctx.for_each::<Block, _>(|_tree, block, span| {
            // only flag explicit blocks (not implicit module-level blocks)
            if block.format == BlockFormat::Explicit && block.expressions.is_empty() {
                Some(
                    LintDiagnostic::new(
                        NO_EMPTY.id,
                        NO_EMPTY.code,
                        NO_EMPTY.category,
                        severity,
                        "empty block statement",
                        file_id,
                        span,
                    )
                    .with_label("this block is empty"),
                )
            } else {
                None
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_block() {
        let test = TestProgram::for_rule(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
{}
"#,
        );
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_detects_empty_if_block() {
        let test = TestProgram::for_rule(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (true) {}
"#,
        );
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_detects_empty_function_body() {
        let test = TestProgram::for_rule(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-empty");
    }

    #[test]
    fn test_no_empty_with_content() {
        let test = TestProgram::for_rule(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
{ let x = 1; }
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }

    #[test]
    fn test_no_empty_module_level() {
        // implicit module-level blocks should not trigger
        let test = TestProgram::for_rule(NoEmpty);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1;
"#,
        );
        test.result(result).assert_no_lint("no-empty");
    }
}
