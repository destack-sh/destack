use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow redundant `return await` in async callables.
    ///
    /// In async functions, `return await value` is usually redundant and can be
    /// simplified to `return value` when it is not inside a try context.
    #[lint(
        id = "return-await",
        code = "LU045",
        category = Suspicious,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable,
        declarations = Exclude
    )]
    pub ReturnAwait,
    "Disallow redundant `return await` in async callables"
}

impl LintRule for ReturnAwait {
    fn meta(&self) -> &'static LintMeta {
        ReturnAwait::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect return expressions
        for return_expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let return_expression = ctx.tree.get(return_expression_id);
            let dir::Expression::Return {
                value: Some(value_expression_id),
            } = return_expression
            else {
                continue;
            };

            let Some((await_expression_id, awaited_value_id)) =
                explicit_await_expression(ctx.tree, *value_expression_id)
            else {
                continue;
            };

            // keep async callable returns only
            if !return_is_inside_async_callable(ctx.tree, return_expression_id) {
                continue;
            }

            // skip try contexts where await may affect control flow
            if return_is_inside_try_context(ctx.tree, return_expression_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, return_expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // build safe replacement from awaited expression text
            let await_span = ctx.get_span(await_expression_id);
            let awaited_span = ctx.get_span(awaited_value_id);
            let awaited_text = ctx.get_span_text(awaited_span).to_string();
            let edits = ctx
                .edit_builder()
                .replace(await_span, awaited_text)
                .into_edits();
            let fix = LintFix::safe("Remove redundant await in return").with_edits(edits);

            // report redundant return await
            ctx.report(
                LintDiagnostic::new(
                    RETURN_AWAIT.id,
                    RETURN_AWAIT.code,
                    RETURN_AWAIT.category,
                    severity,
                    "redundant return await",
                    ctx.module.file_id,
                    await_span,
                )
                .with_label("this await is redundant in an async return outside try/catch/finally")
                .with_fix(fix),
            );
        }
    }
}

/// Return the explicit await expression and its awaited value when present.
fn explicit_await_expression(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<(
    dir::LocalNodeId<dir::Expression>,
    dir::LocalNodeId<dir::Expression>,
)> {
    let expression = tree.get(expression_id);

    // direct await expression
    if let dir::Expression::Await { expression } = expression {
        return Some((expression_id, *expression));
    }

    // recurse through parenthesized wrappers
    let dir::Expression::Parenthesized { expression } = expression else {
        return None;
    };

    explicit_await_expression(tree, *expression)
}

/// Return true when one return expression belongs to an async callable.
fn return_is_inside_async_callable(
    tree: &dir::NodeTree,
    return_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current = tree.get_parent(return_expression_id.id);

    // walk ancestors until the owning callable boundary
    while let Some(parent_id) = current {
        // async functions satisfy this check
        if parent_id.ty == dir::NodeType::Declaration {
            let declaration = tree.get(parent_id.into_typed::<dir::Declaration>());
            if let dir::Declaration::Function { signature, .. } = declaration {
                return signature.asynchrony == dir::Asynchrony::Async;
            }
        }

        // async methods satisfy this check
        if parent_id.ty == dir::NodeType::Member {
            let member = tree.get(parent_id.into_typed::<dir::Member>());
            if let dir::Member::Method { signature, .. } = member {
                return signature.asynchrony == dir::Asynchrony::Async;
            }
        }

        current = tree.get_parent(parent_id.id);
    }

    false
}

/// Return true when one return expression is nested in a try context.
fn return_is_inside_try_context(
    tree: &dir::NodeTree,
    return_expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let mut current = tree.get_parent(return_expression_id.id);

    // walk ancestors and detect any enclosing try node
    while let Some(parent_id) = current {
        // enclosing try means keep return await
        if parent_id.ty == dir::NodeType::Expression {
            let expression = tree.get(parent_id.into_typed::<dir::Expression>());
            if matches!(expression, dir::Expression::Try { .. }) {
                return true;
            }
        }

        // callable boundary without try means no try context
        if parent_id.ty == dir::NodeType::Declaration {
            let declaration = tree.get(parent_id.into_typed::<dir::Declaration>());
            if matches!(declaration, dir::Declaration::Function { .. }) {
                return false;
            }
        }
        if parent_id.ty == dir::NodeType::Member {
            let member = tree.get(parent_id.into_typed::<dir::Member>());
            if matches!(member, dir::Member::Method { .. }) {
                return false;
            }
        }

        current = tree.get_parent(parent_id.id);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag redundant return-await in async functions.
    #[test]
    fn test_flags_redundant_return_await() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_redundant_return_await.ds",
            r#"
async function load(): Promise<int32> {
    return await fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Safely remove redundant return-await.
    #[test]
    fn test_fix_redundant_return_await() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_fix_redundant_return_await.ds",
            r#"
async function load(): Promise<int32> {
    return await fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("return-await")
            .assert_has_fix("return-await")
            .assert_safe_fixed(
                r#"
async function load(): Promise<int32> {
    return fetchValue();
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
            );
    }

    /// Allow return-await inside try contexts.
    #[test]
    fn test_allows_return_await_inside_try() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_allows_return_await_inside_try.ds",
            r#"
async function load(): Promise<int32> {
    try {
        return await fetchValue();
    } catch (error) {
        return 0;
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Ignore synchronous functions.
    #[test]
    fn test_ignores_sync_function() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_ignores_sync_function.ds",
            r#"
function load(): int32 {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Flag redundant return-await in async methods.
    #[test]
    fn test_flags_redundant_return_await_in_async_method() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_flags_redundant_return_await_in_async_method.ds",
            r#"
class Loader {
    async load(): Promise<int32> {
        return await fetchValue();
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_lint("return-await");
    }

    /// Keep return-await in finally blocks inside try expressions.
    #[test]
    fn test_allows_return_await_inside_finally() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_allows_return_await_inside_finally.ds",
            r#"
async function load(): Promise<int32> {
    try {
        return 0;
    } finally {
        return await fetchValue();
    }
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("return-await");
    }

    /// Fix redundant return-await when wrapped in parentheses.
    #[test]
    fn test_fix_parenthesized_return_await() {
        let test = TestProgram::for_rule_with_prelude(ReturnAwait);
        let result = test.lint_dir(
            "return_await/test_fix_parenthesized_return_await.ds",
            r#"
async function load(): Promise<int32> {
    return (await fetchValue());
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
        );
        test.result(result)
            .assert_lint("return-await")
            .assert_has_fix("return-await")
            .assert_safe_fixed(
                r#"
async function load(): Promise<int32> {
    return (fetchValue());
}

async function fetchValue(): Promise<int32> {
    return 1;
}
"#,
            );
    }
}
