use crate::LintMeta;
use destack_dir::{self as dir, Expression};
use destack_repository::LintSeverity;

use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Prefer self-closing tree elements when possible.
    ///
    /// Tree elements without children should use the self-closing form
    /// for brevity and clarity.
    ///
    /// ```
    /// // bad
    /// <Component></Component>
    /// <div attr="value"></div>
    ///
    /// // good
    /// <Component />
    /// <div attr="value" />
    /// ```
    #[lint(
        id = "prefer-self-closing-tree",
        code = "LY053",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferSelfClosingTree,
    "Prefer self-closing tree elements"
}

impl LintRule for PreferSelfClosingTree {
    fn meta(&self) -> &'static LintMeta {
        PreferSelfClosingTree::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let Expression::TreeExpression {
                left,
                arguments: _,
                elements,
                generic_arguments: _,
            } = expression
            else {
                continue;
            };

            // skip fragment shorthand (no left element)
            if left.is_none() {
                continue;
            }

            // self-closing elements have `elements: None`
            // elements with open/close tags have `elements: Some([...])`
            // we want to flag `elements: Some([])` (empty children, but not self-closing)
            let is_empty_non_self_closing = matches!(elements, Some(e) if e.is_empty());

            if !is_empty_non_self_closing {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.dir.get_span(node_id);
            let expr_text = ctx.get_span_text(expression_span);

            // build fix: <Component></Component> -> <Component />
            // find `></` pattern (close of opening tag, start of closing tag)
            let replacement = if let Some(pos) = expr_text.find("></") {
                // take everything up to and not including >, then add />
                format!("{} />", &expr_text[..pos])
            } else {
                expr_text.to_string()
            };

            let edits = ctx
                .edit_builder()
                .replace(expression_span, replacement)
                .into_edits();
            let fix = LintFix::safe("Convert to self-closing").with_edits(edits);

            ctx.report(
                LintReport::new(
                    PREFER_SELF_CLOSING_TREE.id,
                    PREFER_SELF_CLOSING_TREE.code,
                    PREFER_SELF_CLOSING_TREE.category,
                    severity,
                    "use self-closing form `<Component />` for elements without children",
                    expression_span,
                )
                .label("replace with `<... />`")
                .fix(fix),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_empty_element_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_empty_element_detected.ds",
            r#"
let elem = <Component></Component>
"#,
        );
        test.result(result).assert_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_empty_element_with_attrs_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_empty_element_with_attrs_detected.ds",
            r#"
let elem = <Component name="foo"></Component>
"#,
        );
        test.result(result).assert_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_self_closing_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_self_closing_allowed.ds",
            r#"
let elem = <Component />
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_element_with_children_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_element_with_children_allowed.ds",
            r#"
let elem = <Component><Child /></Component>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_element_with_text_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_element_with_text_allowed.ds",
            r#"
let elem = <Component>"Hello"</Component>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_fragment_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_fragment_allowed.ds",
            r#"
let elem = <></>
"#,
        );
        // fragments don't need to be self-closing
        test.result(result)
            .assert_no_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_html_element_empty_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_html_element_empty_detected.ds",
            r#"
let elem = <div></div>
"#,
        );
        test.result(result).assert_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_nested_empty_detected() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_nested_empty_detected.ds",
            r#"
let elem = <Parent><Child></Child></Parent>
"#,
        );
        // the inner Child element is empty and should be self-closing
        test.result(result).assert_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_fix_to_self_closing() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_fix_to_self_closing.ds",
            r#"
let elem = <Component></Component>;
"#,
        );
        test.result(result)
            .assert_lint("prefer-self-closing-tree")
            .assert_safe_fixed(
                r#"
let elem = <Component />;
"#,
            );
    }

    #[test]
    fn test_fix_with_attrs() {
        let test = TestProgram::for_rule_without_prelude(PreferSelfClosingTree);
        let result = test.lint(
            "prefer_self_closing_tree/test_fix_with_attrs.ds",
            r#"
let elem = <Component name="foo"></Component>;
"#,
        );
        test.result(result)
            .assert_lint("prefer-self-closing-tree")
            .assert_safe_fixed(
                r#"
let elem = <Component name="foo" />;
"#,
            );
    }
}
