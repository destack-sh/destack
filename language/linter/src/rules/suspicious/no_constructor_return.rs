use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow returning a value from a constructor.
    ///
    /// Returning a value from a constructor is suspicious because it can
    /// override the newly created object. Use a factory function instead.
    #[lint(
        id = "no-constructor-return",
        code = "LU010",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoConstructorReturn,
    "Disallow return with value in constructor"
}

impl LintRule for NoConstructorReturn {
    fn meta(&self) -> &'static crate::LintMeta {
        NoConstructorReturn::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // find all constructor methods
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

            let Some(ast::FunctionMode::Constructor) = signature.mode else {
                continue;
            };

            // check for return statements with values in the body
            let mut visitor = ConstructorReturnVisitor {
                options: NodeVisitorOptions::default(),
                return_nodes: Vec::new(),
            };

            let body_expr = ctx.tree.get(*body_id);
            visitor.visit_expression(ctx.tree, *body_id, body_expr);

            for return_id in visitor.return_nodes {
                let severity = ctx.get_effective_severity(meta, return_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_CONSTRUCTOR_RETURN.id,
                        NO_CONSTRUCTOR_RETURN.code,
                        NO_CONSTRUCTOR_RETURN.category,
                        severity,
                        "return with value in constructor",
                        ctx.module.file_id,
                        ctx.tree.get_span(return_id),
                    )
                    .with_label("constructors should not return values"),
                );
            }
        }
    }
}

/// NodeVisitor that finds return statements with values, but stops at nested functions.
struct ConstructorReturnVisitor {
    options: NodeVisitorOptions,
    return_nodes: Vec<LocalNodeId<Expression>>,
}

impl NodeVisitor for ConstructorReturnVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // check for return with value
        if matches!(expression, Expression::Return { value: Some(_) }) {
            self.return_nodes.push(id);
        }

        // don't descend into nested function declarations
        if let Expression::Declaration(declaration_id) = expression {
            let declaration = tree.get(*declaration_id);
            if matches!(declaration, ast::Declaration::Function { .. }) {
                return; // stop here - don't check nested functions
            }
        }

        // walk children
        destack_base::ensure_sufficient_stack(|| walk_expression(self, tree, id, expression));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_return_value_in_constructor() {
        let test = TestProgram::for_rule(NoConstructorReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {
    constructor() {
        return { x: 1 }
    }
}
"#,
        );
        test.result(result).assert_lint("no-constructor-return");
    }

    #[test]
    fn test_detects_return_value_in_if() {
        let test = TestProgram::for_rule(NoConstructorReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {
    constructor(x: boolean) {
        if (x) {
            return { special: true }
        }
    }
}
"#,
        );
        test.result(result).assert_lint("no-constructor-return");
    }

    #[test]
    fn test_allows_bare_return() {
        let test = TestProgram::for_rule(NoConstructorReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {
    constructor(x: boolean) {
        if (x) {
            return
        }
        this.value = 1
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-constructor-return");
    }

    #[test]
    fn test_allows_constructor_without_return() {
        let test = TestProgram::for_rule(NoConstructorReturn);
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
        test.result(result).assert_no_lint("no-constructor-return");
    }

    #[test]
    fn test_allows_return_in_nested_function() {
        let test = TestProgram::for_rule(NoConstructorReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {
    constructor() {
        const helper = () => {
            return { x: 1 }
        }
        this.helper = helper
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-constructor-return");
    }
}
