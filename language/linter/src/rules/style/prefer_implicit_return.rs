use destack_ast::{self as ast, Block, Declaration, Expression, FunctionKind};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer implicit return for simple arrow functions.
    ///
    /// Use `() => x` instead of `() => { return x }` for concise arrow functions.
    #[lint(
        id = "prefer-implicit-return",
        code = "LY048",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferImplicitReturn,
    "Prefer implicit return for arrow functions"
}

impl LintRule for PreferImplicitReturn {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferImplicitReturn::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let decl = ctx.tree.get(node_id);

            // look for arrow functions
            let Declaration::Function {
                signature,
                body: Some(body_id),
                ..
            } = decl
            else {
                continue;
            };

            // only check lambda functions
            if signature.kind != FunctionKind::Lambda {
                continue;
            }

            let body_expr = ctx.tree.get(*body_id);

            // check if body is a block with single return statement
            let Expression::Block(block_id) = body_expr else {
                continue;
            };

            let block = ctx.tree.get(*block_id);

            // check for block with single expression that is a return
            if !is_single_return_block(ctx.tree, block) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    PREFER_IMPLICIT_RETURN.id,
                    PREFER_IMPLICIT_RETURN.code,
                    PREFER_IMPLICIT_RETURN.category,
                    severity,
                    "use implicit return instead of block with return",
                    ctx.module.file_id,
                    ctx.tree.get_span(*body_id),
                )
                .with_label("use `() => x` instead of `() => { return x }`"),
            );
        }
    }
}

/// Check if a block contains only a single return statement with a value.
fn is_single_return_block(tree: &ast::NodeTree, block: &Block) -> bool {
    if block.expressions.len() != 1 {
        return false;
    }

    let expr_id = block.expressions[0];
    let expr = tree.get(expr_id);

    // unwrap statement wrapper
    let inner = match expr {
        Expression::Statement(inner_id) => tree.get(*inner_id),
        other => other,
    };

    // check if it's a return with a value
    matches!(inner, Expression::Return { value: Some(_), .. })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_block_with_return() {
        let test = TestProgram::for_rule_without_builtins(PreferImplicitReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
const double = (x) => { return x * 2 }
"#,
        );
        test.result(result).assert_lint("prefer-implicit-return");
    }

    #[test]
    fn test_allows_implicit_return() {
        let test = TestProgram::for_rule_without_builtins(PreferImplicitReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
const double = (x) => x * 2
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }

    #[test]
    fn test_allows_multi_statement_block() {
        let test = TestProgram::for_rule_without_builtins(PreferImplicitReturn);
        // multi-statement blocks can't use implicit return
        let result = test.lint_ast(
            "test.ds",
            r#"
const double = (x) => {
    const y = x * 2
    return y
}
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }

    #[test]
    fn test_allows_function_declaration() {
        let test = TestProgram::for_rule_without_builtins(PreferImplicitReturn);
        // traditional functions always need blocks
        let result = test.lint_ast(
            "test.ds",
            r#"
function double(x) {
    return x * 2
}
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }

    #[test]
    fn test_allows_void_return() {
        let test = TestProgram::for_rule_without_builtins(PreferImplicitReturn);
        // void returns can't be implicit
        let result = test.lint_ast(
            "test.ds",
            r#"
const log = (x) => { return }
"#,
        );
        test.result(result).assert_no_lint("prefer-implicit-return");
    }
}
