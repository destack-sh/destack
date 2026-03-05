use destack_ast::{
    self as ast, Expression, LocalNodeId, NodeTree, NodeVisitor, NodeVisitorOptions,
    walk_expression,
};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_starts_nested_declaration_scope;
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow returning a value from a constructor.
    ///
    /// Returning a value from a constructor is suspicious because it can
    /// override the newly created object. Use a factory function instead.
    #[lint(
        id = "no-constructor-return",
        code = "LU007",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
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

                let mut diagnostic = LintDiagnostic::new(
                    NO_CONSTRUCTOR_RETURN.id,
                    NO_CONSTRUCTOR_RETURN.code,
                    NO_CONSTRUCTOR_RETURN.category,
                    severity,
                    "return with value in constructor",
                    ctx.module.file_id,
                    ctx.tree.get_span(return_id),
                )
                .with_label("constructors should not return values");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = no_constructor_return_fix(ctx, return_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// NodeVisitor that finds return statements with values in one constructor body scope.
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

        // don't descend into nested declaration scopes
        if expression_starts_nested_declaration_scope(expression) {
            return;
        }

        // walk children
        walk_expression(self, tree, id, expression);
    }
}

/// Build an unsafe fix for one constructor return value.
fn no_constructor_return_fix(
    ctx: &LintModuleAstContext<'_>,
    return_id: ast::LocalNodeId<ast::Expression>,
) -> Option<LintFix> {
    let return_expression = ctx.tree.get(return_id);
    let Expression::Return {
        value: Some(value_id),
    } = return_expression
    else {
        return None;
    };

    let value_span = ctx.tree.get_span(*value_id);
    let value_text = ctx.get_span_text(value_span);
    if value_text.trim().is_empty() {
        return None;
    }

    let replacement = format!("{{ ({value_text}); return; }}");
    let return_span = ctx.tree.get_span(return_id);
    let edits = ctx
        .edit_builder()
        .replace(return_span, replacement)
        .into_edits();

    Some(LintFix::r#unsafe("Drop constructor return value").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_return_value_in_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_detects_return_value_in_constructor.ds",
            r#"
class Foo {
    constructor() {
        return { x: 1 }
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-constructor-return")
            .assert_has_fix("no-constructor-return");
    }

    #[test]
    fn test_detects_return_value_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_detects_return_value_in_if.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_allows_bare_return.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_allows_constructor_without_return.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_allows_return_in_nested_function.ds",
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

    #[test]
    fn test_allows_return_in_nested_class_method() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_allows_return_in_nested_class_method.ds",
            r#"
class Foo {
    constructor() {
        class Nested {
            method() {
                return 1
            }
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-constructor-return");
    }

    #[test]
    fn test_fix_rewrites_constructor_return_value() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_fix_rewrites_constructor_return_value.ds",
            r#"
class Foo {
    constructor() {
        return makeValue()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-constructor-return")
            .assert_unsafe_fixed(
                r#"
class Foo {
    constructor() {
        {
            (makeValue());
            return;
        }
    }
}
"#,
            );
    }

    #[test]
    fn test_fix_preserves_object_literal_side_effect_expression() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_fix_preserves_object_literal_side_effect_expression.ds",
            r#"
class Foo {
    constructor() {
        return { value: buildValue() }
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-constructor-return")
            .assert_unsafe_fixed(
                r#"
class Foo {
    constructor() {
        {
            ({ value: buildValue() });
            return;
        }
    }
}
"#,
            );
    }

    #[test]
    fn test_mutation_detects_constructor_return_in_conditional() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_ast(
            "no_constructor_return/test_mutation_detects_constructor_return_in_conditional.ds",
            r#"
class Foo {
    constructor(shouldReturn: boolean) {
        if (shouldReturn) {
            return makeValue()
        }
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-constructor-return")
            .assert_unsafe_fixed(
                r#"
class Foo {
    constructor(shouldReturn: boolean) {
        if (shouldReturn) {
            {
                (makeValue());
                return;
            }
        }
    }
}
"#,
            );
    }
}
