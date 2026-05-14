use crate::LintMeta;
use destack_dir::{self as dir, Declaration};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_or_declaration_docs;
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Require return type documentation.
    ///
    /// Functions with non-void return types should document what they return.
    #[lint(
        id = "require-returns-doc",
        code = "LY063",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireReturnsDoc,
    "Require return documentation"
}

impl LintRule for RequireReturnsDoc {
    fn meta(&self) -> &'static LintMeta {
        RequireReturnsDoc::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // iterate over Expression nodes to find function declarations
        // (annotations are attached to Expression nodes, not Declaration nodes)
        for expr_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let dir::Expression::Declaration(decl_id) = ctx.dir.get(expr_id) else {
                continue;
            };

            let declaration_id = *decl_id;
            let declaration = ctx.dir.get(declaration_id);

            let Declaration::Function(declaration) = declaration else {
                continue;
            };

            // only check exported functions
            if declaration.export.is_none() {
                continue;
            }

            // skip functions without a return type
            if declaration.signature.return_type.is_none() {
                continue;
            }

            let docs = expression_or_declaration_docs(ctx, expr_id, declaration_id);
            if docs.is_empty() {
                continue;
            }

            let has_returns = docs.into_iter().any(|comment| {
                let doc_content = dir::normalize_comment_payload(ctx.get_span_text(comment.span));
                let doc_lowercase = doc_content.to_ascii_lowercase();
                doc_lowercase.contains("@returns")
                    || doc_lowercase.contains("@return")
                    || doc_lowercase.contains("returns")
            });
            if !has_returns {
                let severity = ctx.get_effective_severity(meta, expr_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        REQUIRE_RETURNS_DOC.id,
                        REQUIRE_RETURNS_DOC.code,
                        REQUIRE_RETURNS_DOC.category,
                        severity,
                        "function with return type lacks @returns documentation",
                        ctx.dir.get_span(expr_id),
                    )
                    .label("add @returns to documentation"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_function_with_return_no_doc_detected() {
        let test = TestProgram::for_rule_without_prelude(RequireReturnsDoc);
        let result = test.lint(
            "require_returns_doc/test_function_with_return_no_doc_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(RequireReturnsDoc);
        let result = test.lint(
            "require_returns_doc/test_function_with_returns_doc_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(RequireReturnsDoc);
        let result = test.lint(
            "require_returns_doc/test_void_function_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(RequireReturnsDoc);
        let result = test.lint(
            "require_returns_doc/test_private_function_allowed.ds",
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
