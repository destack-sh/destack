use std::collections::HashMap;

use destack_ast::{self as ast, Argument, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest spreading repeated tree props.
    ///
    /// When multiple sibling tree elements share the same prop with the same
    /// value, consider extracting the common props to a spread.
    ///
    /// ```
    /// // bad - repeated props on siblings
    /// <Parent>
    ///     <Child theme={dark} size="large" />
    ///     <Child theme={dark} size="large" />
    /// </Parent>
    ///
    /// // good - common props extracted
    /// const childProps = { theme: dark, size: "large" }
    /// <Parent>
    ///     <Child ...childProps />
    ///     <Child ...childProps />
    /// </Parent>
    /// ```
    #[lint(
        id = "tree-prop-spread-candidate",
        code = "LY063",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub TreePropSpreadCandidate,
    "Suggest spreading repeated tree props"
}

impl LintRule for TreePropSpreadCandidate {
    fn meta(&self) -> &'static crate::LintMeta {
        TreePropSpreadCandidate::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let Expression::TreeExpression { elements, .. } = expression else {
                continue;
            };

            let Some(elements) = elements else {
                continue;
            };

            // collect named props from all child tree elements
            // map: prop_name -> count_of_children_with_prop
            let mut prop_counts: HashMap<String, usize> = HashMap::new();
            let mut child_tree_count = 0;

            for element_id in elements {
                let element = ctx.tree.get(*element_id);

                // elements can be tree expressions themselves (as Arguments)
                let value_id = match element {
                    Argument::Positional { value } => value,
                    _ => continue,
                };

                let child_expression = ctx.tree.get(*value_id);
                let Expression::TreeExpression { arguments, .. } = child_expression else {
                    continue;
                };

                child_tree_count += 1;

                let Some(args) = arguments else {
                    continue;
                };

                for arg_id in args {
                    let arg = ctx.tree.get(*arg_id);
                    if let Argument::Named { name, .. } = arg {
                        let name_str = ctx.strings.get(name.string()).to_string();
                        *prop_counts.entry(name_str).or_insert(0) += 1;
                    }
                }
            }

            // if we have 2+ child tree elements and any prop appears on 2+ of them,
            // it's a candidate for spreading
            if child_tree_count >= 2 {
                for (prop_name, count) in &prop_counts {
                    if *count >= 2 {
                        ctx.report(
                            LintDiagnostic::new(
                                TREE_PROP_SPREAD_CANDIDATE.id,
                                TREE_PROP_SPREAD_CANDIDATE.code,
                                TREE_PROP_SPREAD_CANDIDATE.category,
                                severity,
                                format!(
                                    "prop `{prop_name}` is repeated on {count} sibling elements; consider extracting to a spread"
                                ),
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("repeated props could be extracted"),
                        );
                        // only report once per parent element
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_repeated_props_detected() {
        let test = TestProgram::for_rule(TreePropSpreadCandidate);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Parent>
    <Child name="foo" />
    <Child name="bar" />
</Parent>
"#,
        );
        test.result(result)
            .assert_lint("tree-prop-spread-candidate");
    }

    #[test]
    fn test_no_repeated_props_allowed() {
        let test = TestProgram::for_rule(TreePropSpreadCandidate);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Parent>
    <Child name="foo" />
    <Child value="bar" />
</Parent>
"#,
        );
        test.result(result)
            .assert_no_lint("tree-prop-spread-candidate");
    }

    #[test]
    fn test_single_child_allowed() {
        let test = TestProgram::for_rule(TreePropSpreadCandidate);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Parent>
    <Child name="foo" />
</Parent>
"#,
        );
        test.result(result)
            .assert_no_lint("tree-prop-spread-candidate");
    }

    #[test]
    fn test_empty_props_allowed() {
        let test = TestProgram::for_rule(TreePropSpreadCandidate);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Parent>
    <Child />
    <Child />
</Parent>
"#,
        );
        test.result(result)
            .assert_no_lint("tree-prop-spread-candidate");
    }

    #[test]
    fn test_three_children_detected() {
        let test = TestProgram::for_rule(TreePropSpreadCandidate);
        let result = test.lint_ast(
            "test.ds",
            r#"
let elem = <Parent>
    <Child theme={dark} />
    <Child theme={dark} />
    <Child theme={dark} />
</Parent>
"#,
        );
        test.result(result)
            .assert_lint("tree-prop-spread-candidate");
    }
}
