use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary constructors.
    ///
    /// An empty constructor is unnecessary and can be removed.
    #[lint(
        id = "no-useless-constructor",
        code = "LU053",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessConstructor,
    "Disallow useless constructors"
}

impl LintRule for NoUselessConstructor {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessConstructor::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Member>() {
            let member = ctx.tree.get(node_id);
            let ast::Member::Method {
                key,
                signature,
                body: Some(body_id),
                ..
            } = member
            else {
                continue;
            };

            // check if this is a constructor (no key, mode is Constructor)
            if key.is_some() {
                continue;
            }

            // check if mode is constructor
            let Some(ast::FunctionMode::Constructor) = signature.mode else {
                continue;
            };

            // check if the body is empty
            if is_empty_body(ctx, *body_id) && signature.dynamic_parameters.is_empty() {
                let severity = ctx.get_effective_severity(meta, *body_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_CONSTRUCTOR.id,
                        NO_USELESS_CONSTRUCTOR.code,
                        NO_USELESS_CONSTRUCTOR.category,
                        severity,
                        "useless constructor",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("empty constructor can be removed"),
                );
            }
        }
    }
}

/// Check if a body is empty.
fn is_empty_body(
    ctx: &LintModuleAstContext<'_>,
    body_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let body = ctx.tree.get(body_id);
    match body {
        ast::Expression::Block(block_id) => {
            let block = ctx.tree.get(*block_id);
            block.expressions.is_empty()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_empty_constructor() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConstructor);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {
    constructor() {}
}
"#,
        );
        test.result(result).assert_lint("no-useless-constructor");
    }

    #[test]
    fn test_allows_constructor_with_initialization() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConstructor);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {
    constructor() {
        this.x = 1
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }

    #[test]
    fn test_allows_constructor_with_params() {
        let test = TestProgram::for_rule_without_builtins(NoUselessConstructor);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {
    constructor(x: int32) {}
}
"#,
        );
        test.result(result).assert_no_lint("no-useless-constructor");
    }
}
