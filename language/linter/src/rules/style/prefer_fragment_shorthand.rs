use destack_ast::{self as ast, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

// #Correctness: prefer-fragment-shorthand works but would be better with canonical DIR symbols?

declare_lint! {
    /// Prefer `<>` shorthand over `<Fragment>`.
    ///
    /// When creating fragments in tree expressions (JSX-like syntax), prefer
    /// the shorthand `<>...</>` over the explicit `<Fragment>...</Fragment>`.
    ///
    /// The shorthand is more concise and idiomatic.
    ///
    /// ```
    /// // bad
    /// <Fragment>
    ///     <Child1 />
    ///     <Child2 />
    /// </Fragment>
    ///
    /// // good
    /// <>
    ///     <Child1 />
    ///     <Child2 />
    /// </>
    /// ```
    #[lint(
        id = "prefer-fragment-shorthand",
        code = "LY053",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferFragmentShorthand,
    "Prefer <> shorthand over <Fragment>"
}

impl LintRule for PreferFragmentShorthand {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferFragmentShorthand::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let Expression::TreeExpression {
                left,
                arguments,
                elements: _,
            } = expression
            else {
                continue;
            };

            // check if the tag is Fragment
            if !is_fragment_tag(ctx, *left) {
                continue;
            }

            // only suggest shorthand if there are no attributes/arguments
            // Fragment with key prop cannot use shorthand: <Fragment key={id}>
            if let Some(args) = arguments
                && !args.is_empty()
            {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    PREFER_FRAGMENT_SHORTHAND.id,
                    PREFER_FRAGMENT_SHORTHAND.code,
                    PREFER_FRAGMENT_SHORTHAND.category,
                    severity,
                    "use `<>...</>` shorthand instead of `<Fragment>...</Fragment>`",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("replace with `<>...</>`"),
            );
        }
    }
}

/// Check if the left expression is `Fragment`.
fn is_fragment_tag(
    ctx: &LintModuleAstContext<'_>,
    left: Option<ast::LocalNodeId<Expression>>,
) -> bool {
    let Some(left_id) = left else {
        return false;
    };

    let left_expr = ctx.tree.get(left_id);

    // check for simple path like `Fragment`
    let Expression::Path { path, .. } = left_expr else {
        return false;
    };

    // check if the path is "Fragment" (single segment)
    if path.segments.len() == 1 {
        let name_str = ctx.strings.get(path.segments[0]);
        return name_str.as_ref() == "Fragment";
    }

    // also check for fully qualified React.Fragment or similar
    if path.segments.len() == 2 {
        let last_segment = ctx.strings.get(path.segments[1]);
        return last_segment.as_ref() == "Fragment";
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_fragment_detected() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Fragment><Child /></Fragment>
"#,
        );
        test.result(result).assert_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_shorthand_allowed() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <><Child /></>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_empty_fragment_detected() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Fragment></Fragment>
"#,
        );
        test.result(result).assert_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_empty_shorthand_allowed() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <></>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_fragment_with_key_allowed() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Fragment key={id}><Child /></Fragment>
"#,
        );
        // Fragment with key prop cannot use shorthand, so it's allowed
        test.result(result)
            .assert_no_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_other_element_allowed() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <div><Child /></div>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_multiple_children_detected() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Fragment>
    <Child1 />
    <Child2 />
</Fragment>
"#,
        );
        test.result(result).assert_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_qualified_fragment_detected() {
        let test = TestProgram::for_rule(PreferFragmentShorthand);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <React.Fragment><Child /></React.Fragment>
"#,
        );
        test.result(result).assert_lint("prefer-fragment-shorthand");
    }
}
