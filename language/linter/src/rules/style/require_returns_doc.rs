use destack_ast::{self as ast, Declaration};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require return type documentation.
    ///
    /// Functions with non-void return types should document what they return.
    #[lint(
        id = "require-returns-doc",
        code = "LY041",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireReturnsDoc,
    "Require return documentation"
}

impl LintRule for RequireReturnsDoc {
    fn meta(&self) -> &'static crate::LintMeta {
        RequireReturnsDoc::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // iterate over Expression nodes to find function declarations
        // (annotations are attached to Expression nodes, not Declaration nodes)
        for expr_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Declaration(decl_id) = ctx.tree.get(expr_id) else {
                continue;
            };

            let declaration = ctx.tree.get(*decl_id);

            let Declaration::Function {
                descriptor,
                signature,
                ..
            } = declaration
            else {
                continue;
            };

            // only check exported functions
            if descriptor.export.is_none() {
                continue;
            }

            // skip functions without a return type
            if signature.return_type.is_none() {
                continue;
            }

            // check if there's documentation (on the expression node)
            let docs = ctx.tree.get_docs_for(expr_id.id);

            if docs.is_empty() {
                continue;
            }

            // get the doc content and check for @returns
            let doc_content = get_doc_content(ctx, expr_id);
            let has_returns = doc_content
                .as_ref()
                .map(|d| d.contains("@returns") || d.contains("@return") || d.contains("Returns"))
                .unwrap_or(false);

            if !has_returns {
                let severity = ctx.get_effective_severity(meta, expr_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        REQUIRE_RETURNS_DOC.id,
                        REQUIRE_RETURNS_DOC.code,
                        REQUIRE_RETURNS_DOC.category,
                        severity,
                        "function with return type lacks @returns documentation",
                        ctx.module.file_id,
                        ctx.tree.get_span(expr_id),
                    )
                    .with_label("add @returns to documentation"),
                );
            }
        }
    }
}

fn get_doc_content(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> Option<String> {
    // get first doc annotation for this expression node
    let docs = ctx.tree.get_docs_for(expr_id.id);
    let (doc_id, _position) = docs.into_iter().next()?;
    let doc = ctx.tree.get(doc_id);
    let text = ctx.strings.get(doc.string);
    Some(text.as_ref().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_function_with_return_no_doc_detected() {
        let test = TestProgram::for_rule_without_builtins(RequireReturnsDoc);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Does something.
export function foo(): int32 {
    return 42
}
"#,
        );
        test.result(result).assert_lint("require-returns-doc");
    }

    #[test]
    fn test_function_with_returns_doc_allowed() {
        let test = TestProgram::for_rule_without_builtins(RequireReturnsDoc);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Does something.
/// @returns The answer.
export function foo(): int32 {
    return 42
}
"#,
        );
        test.result(result).assert_no_lint("require-returns-doc");
    }

    #[test]
    fn test_void_function_allowed() {
        let test = TestProgram::for_rule_without_builtins(RequireReturnsDoc);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Does something.
export function foo() {
    console.log("hi")
}
"#,
        );
        test.result(result).assert_no_lint("require-returns-doc");
    }

    #[test]
    fn test_private_function_allowed() {
        let test = TestProgram::for_rule_without_builtins(RequireReturnsDoc);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Does something.
function foo(): int32 {
    return 42
}
"#,
        );
        test.result(result).assert_no_lint("require-returns-doc");
    }
}
