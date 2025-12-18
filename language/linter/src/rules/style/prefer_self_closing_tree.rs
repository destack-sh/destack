use destack_ast::{self as ast, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer self-closing tree elements when possible.
    ///
    /// Tree elements without children should use the self-closing syntax
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
        code = "LY058",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferSelfClosingTree,
    "Prefer self-closing tree elements"
}

impl LintRule for PreferSelfClosingTree {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferSelfClosingTree::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let Expression::TreeExpression {
                left,
                arguments: _,
                elements,
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

            ctx.report(
                LintDiagnostic::new(
                    PREFER_SELF_CLOSING_TREE.id,
                    PREFER_SELF_CLOSING_TREE.code,
                    PREFER_SELF_CLOSING_TREE.category,
                    severity,
                    "use self-closing syntax `<Component />` for elements without children",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("replace with `<... />`"),
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
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Component></Component>
"#,
        );
        test.result(result).assert_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_empty_element_with_attrs_detected() {
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Component name="foo"></Component>
"#,
        );
        test.result(result).assert_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_self_closing_allowed() {
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Component />
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_element_with_children_allowed() {
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Component><Child /></Component>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_element_with_text_allowed() {
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Component>"Hello"</Component>
"#,
        );
        test.result(result)
            .assert_no_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_fragment_allowed() {
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <div></div>
"#,
        );
        test.result(result).assert_lint("prefer-self-closing-tree");
    }

    #[test]
    fn test_nested_empty_detected() {
        let test = TestProgram::for_rule(PreferSelfClosingTree);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Parent><Child></Child></Parent>
"#,
        );
        // the inner Child element is empty and should be self-closing
        test.result(result).assert_lint("prefer-self-closing-tree");
    }
}
