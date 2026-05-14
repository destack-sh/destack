use crate::LintMeta;
use destack_dir::{self as dir, Expression};
use destack_workspace::LintSeverity;

use crate::rules::common::expression_path_segments;
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

// #Correctness: prefer-fragment-shorthand works but would be better with canonical DIR symbols?

declare_lint! {
    /// Prefer `<>` shorthand over `<Fragment>`.
    ///
    /// When creating fragments in tree expressions, prefer
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
        code = "LY039",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferFragmentShorthand,
    "Prefer <> shorthand over <Fragment>"
}

impl LintRule for PreferFragmentShorthand {
    fn meta(&self) -> &'static LintMeta {
        PreferFragmentShorthand::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let Expression::TreeExpression {
                left,
                arguments,
                elements: _,
                generic_arguments: _,
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

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // make fix: <Fragment>...</Fragment> -> <>...</>
            let expression_span = ctx.dir.get_span(node_id);
            let expr_text = ctx.get_span_text(expression_span);
            let replacement = convert_fragment_to_shorthand(expr_text);
            let edits = ctx
                .edit_builder()
                .replace(expression_span, replacement)
                .into_edits();
            let fix = LintFix::safe("Convert to fragment shorthand").with_edits(edits);

            ctx.report(
                LintReport::new(
                    PREFER_FRAGMENT_SHORTHAND.id,
                    PREFER_FRAGMENT_SHORTHAND.code,
                    PREFER_FRAGMENT_SHORTHAND.category,
                    severity,
                    "use `<>...</>` shorthand instead of `<Fragment>...</Fragment>`",
                    expression_span,
                )
                .label("replace with `<>...</>`")
                .fix(fix),
            );
        }
    }
}

/// Convert `<Fragment>...</Fragment>` to `<>...</>`.
fn convert_fragment_to_shorthand(text: &str) -> String {
    let mut result = text.to_string();

    // replace opening tag: <Fragment>
    // find the first >
    if let Some(end) = result.find('>') {
        // check if it's self-closing />
        if result[..end].ends_with('/') {
            // <Fragment /> -> </>
            result = format!("</{}", &result[end..]);
        } else {
            // <Fragment> -> <>
            result = format!("<{}", &result[end..]);
        }
    }

    // replace closing tag: </Fragment>
    // find </Fragment> at the end
    if let Some(close_start) = result.rfind("</") {
        let close_end = result[close_start..]
            .find('>')
            .unwrap_or(result.len() - close_start);
        let before_close = &result[..close_start];
        let after_close = &result[close_start + close_end + 1..];
        result = format!("{before_close}</>{after_close}");
    }

    result
}

/// Check if the left expression is `Fragment`.
fn is_fragment_tag(
    ctx: &LintModuleContext<'_>,
    left: Option<dir::LocalNodeId<Expression>>,
) -> bool {
    let Some(left_id) = left else {
        return false;
    };

    let Some(path_segments) = expression_path_segments(ctx.dir.tree(), left_id) else {
        return false;
    };

    // check if the path is "Fragment" (single segment)
    if path_segments.len() == 1 {
        let name_str = ctx.strings.get(path_segments[0]);
        return name_str == "Fragment";
    }

    // also check for fully qualified React.Fragment or similar
    if path_segments.len() == 2 {
        let last_segment = ctx.strings.get(path_segments[1]);
        return last_segment == "Fragment";
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_fragment_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_fragment_detected.ds",
            r#"
let elem = <Fragment><Child /></Fragment>
"#,
        );
        test.result(result).assert_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_shorthand_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_shorthand_allowed.ds",
            r#"
let elem = <><Child /></>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_empty_fragment_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_empty_fragment_detected.ds",
            r#"
let elem = <Fragment></Fragment>
"#,
        );
        test.result(result).assert_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_empty_shorthand_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_empty_shorthand_allowed.ds",
            r#"
let elem = <></>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_fragment_with_key_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_fragment_with_key_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_other_element_allowed.ds",
            r#"
let elem = <div><Child /></div>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_multiple_children_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_multiple_children_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_qualified_fragment_detected.ds",
            r#"
let elem = <React.Fragment><Child /></React.Fragment>
"#,
        );
        test.result(result).assert_lint("prefer-fragment-shorthand");
    }

    #[test]
    fn test_fix_fragment_to_shorthand() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_fix_fragment_to_shorthand.ds",
            r#"
let elem = <Fragment><Child /></Fragment>;
"#,
        );
        test.result(result)
            .assert_lint("prefer-fragment-shorthand")
            .assert_safe_fixed(
                r#"
let elem = (
    <>
        <Child />
    </>
);
"#,
            );
    }

    #[test]
    fn test_fix_empty_fragment() {
        let test = TestProgram::for_rule_without_prelude(PreferFragmentShorthand);
        let result = test.lint(
            "prefer_fragment_shorthand/test_fix_empty_fragment.ds",
            r#"
let elem = <Fragment></Fragment>;
"#,
        );
        test.result(result)
            .assert_lint("prefer-fragment-shorthand")
            .assert_safe_fixed(
                r#"
let elem = <></>;
"#,
            );
    }
}
