use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::callable_return_usage;
use crate::{LintFix, LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow returning a value from a constructor.
    ///
    /// Returning a value from a constructor is suspicious because it can
    /// override the newly created object.
    /// Use a factory function instead.
    #[lint(
        id = "no-constructor-return",
        code = "LU007",
        category = Suspicious,
        level = Dir,
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
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoConstructorReturn::meta()
    }

    /// Check module DIR members for constructor return values.
    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // inspect constructor methods with concrete bodies
        for member_id in ctx.tree.iter_node_ids_of_type::<dir::Member>() {
            let member = ctx.tree.get(member_id);
            let dir::Member::Method {
                signature,
                body: Some(body_expression_id),
                ..
            } = member
            else {
                continue;
            };

            // keep non-constructors out of this rule
            if signature.role != Some(dir::FunctionRole::Constructor) {
                continue;
            }

            // analyze explicit constructor return values
            let return_usage =
                callable_return_usage(ctx.tree, signature, Some(*body_expression_id));
            if return_usage.return_value_nodes.is_empty() {
                continue;
            }

            for return_expression_id in return_usage.return_value_nodes {
                let severity = ctx.get_effective_severity(meta, return_expression_id);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintReport::new(
                    NO_CONSTRUCTOR_RETURN.id,
                    NO_CONSTRUCTOR_RETURN.code,
                    NO_CONSTRUCTOR_RETURN.category,
                    severity,
                    "return with value in constructor",
                    ctx.get_span(return_expression_id),
                )
                .label("constructors should not return values");

                // attach the unsafe rewrite when requested
                if ctx.include_fixes
                    && let Some(fix) = no_constructor_return_fix(ctx, return_expression_id)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build an unsafe fix for one constructor return value.
fn no_constructor_return_fix(
    ctx: &LintModuleDirContext<'_>,
    return_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    let return_expression = ctx.tree.get(return_expression_id);
    let dir::Expression::Return {
        value: Some(value_expression_id),
    } = return_expression
    else {
        return None;
    };

    // keep empty value slices out of the rewrite
    let value_span = ctx.get_span(*value_expression_id);
    let value_text = ctx.get_span_text(value_span);
    if value_text.trim().is_empty() {
        return None;
    }

    // preserve evaluation order, then return bare
    let replacement_value =
        statement_safe_return_value_text(ctx, *value_expression_id, return_expression, value_text);
    let replacement = format!("{{ {replacement_value}; return; }}");
    let edits = ctx
        .edit_builder()
        .replace(ctx.get_span(return_expression_id), replacement)
        .into_edits();

    Some(LintFix::r#unsafe("Drop constructor return value").with_edits(edits))
}

/// Return statement-safe text for one constructor return value expression.
fn statement_safe_return_value_text(
    ctx: &LintModuleDirContext<'_>,
    value_expression_id: dir::LocalNodeId<dir::Expression>,
    _return_expression: &dir::Expression,
    value_text: &str,
) -> String {
    let value_expression = ctx.tree.get(value_expression_id);
    if matches!(
        value_expression,
        dir::Expression::ObjectExpression { .. } | dir::Expression::Declaration { .. }
    ) {
        return format!("({value_text})");
    }

    value_text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Flag constructor returns with values.
    #[test]
    fn test_detects_return_value_in_constructor() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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

    /// Flag constructor returns nested in control flow.
    #[test]
    fn test_detects_return_value_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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

    /// Allow bare constructor returns.
    #[test]
    fn test_allows_bare_return() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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

    /// Allow constructors without return statements.
    #[test]
    fn test_allows_constructor_without_return() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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

    /// Ignore returns in nested functions.
    #[test]
    fn test_allows_return_in_nested_function() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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

    /// Ignore returns in nested class bodies.
    #[test]
    fn test_allows_return_in_nested_class_method() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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

    /// Rewrite constructor return values into side effect preserving blocks.
    #[test]
    fn test_fix_rewrites_constructor_return_value() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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
            makeValue();
            return;
        }
    }
}
"#,
            );
    }

    /// Preserve object literal side effects in the fix.
    #[test]
    fn test_fix_preserves_object_literal_side_effect_expression() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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

    /// Preserve declaration-valued returns as expressions in the fix.
    #[test]
    fn test_fix_parenthesizes_function_expression_return_value() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
            "no_constructor_return/test_fix_parenthesizes_function_expression_return_value.ds",
            r#"
class Foo {
    constructor() {
        return function buildValue(): int32 {
            return 1
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
    constructor() {
        {
            (function buildValue(): int32 {
                return 1
            });
            return;
        }
    }
}
"#,
            );
    }

    /// Detect conditional constructor returns through DIR callable analysis.
    #[test]
    fn test_mutation_detects_constructor_return_in_conditional() {
        let test = TestProgram::for_rule_without_prelude(NoConstructorReturn);
        let result = test.lint_dir(
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
                makeValue();
                return;
            }
        }
    }
}
"#,
            );
    }
}
