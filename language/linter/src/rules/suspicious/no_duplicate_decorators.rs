use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{arguments_are_equal, paths_equal};
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate decorators on the same target.
    ///
    /// Only flags decorators that are structurally identical (same name and arguments).
    /// Decorators with different arguments are allowed, supporting stackable decorators.
    #[lint(
        id = "no-duplicate-decorators",
        code = "LU012",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateDecorators,
    "Disallow duplicate decorators on the same target"
}

impl LintRule for NoDuplicateDecorators {
    fn meta(&self) -> &'static crate::LintMeta {
        NoDuplicateDecorators::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // check each annotated node
        for annotations in ctx.tree.get_all_annotations().values() {
            // collect decorator annotation ids for comparison
            let mut seen: Vec<ast::LocalNodeId<ast::Annotation>> = Vec::new();

            for annotation_id in annotations {
                let annotation = ctx.tree.get(*annotation_id);
                let ast::Annotation::Decorator { node, .. } = annotation else {
                    continue;
                };

                // check against all previously seen decorators
                let duplicate = seen.iter().find(|seen_id| {
                    let seen_annotation = ctx.tree.get(**seen_id);
                    let ast::Annotation::Decorator {
                        node: seen_node, ..
                    } = seen_annotation
                    else {
                        return false;
                    };
                    decorators_equal(ctx, *node, *seen_node)
                });

                // report if we found a duplicate
                if let Some(first_id) = duplicate {
                    let severity = ctx.get_effective_severity(meta, *first_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    // extract decorator name for the message
                    let decorator = ctx.tree.get(*node);
                    let name = decorator
                        .left
                        .segments
                        .first()
                        .map(|id| ctx.strings.get(*id))
                        .map(|s| s.as_ref().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    let span = ctx.tree.get_span(*annotation_id);
                    let fix = LintFix::safe("Remove duplicate decorator").delete(span);

                    ctx.report(
                        LintDiagnostic::new(
                            NO_DUPLICATE_DECORATORS.id,
                            NO_DUPLICATE_DECORATORS.code,
                            NO_DUPLICATE_DECORATORS.category,
                            severity,
                            format!("duplicate decorator '@{name}'"),
                            ctx.module.file_id,
                            span,
                        )
                        .with_label("this decorator is already applied with identical arguments")
                        .with_fix(fix),
                    );
                } else {
                    seen.push(*annotation_id);
                }
            }
        }
    }
}

/// Compare two decorators for structural equality.
fn decorators_equal(
    ctx: &LintModuleAstContext<'_>,
    left_id: ast::LocalNodeId<ast::Decorator>,
    right_id: ast::LocalNodeId<ast::Decorator>,
) -> bool {
    let left = ctx.tree.get(left_id);
    let right = ctx.tree.get(right_id);

    // compare paths
    if !paths_equal(ctx, &left.left, &right.left) {
        return false;
    }

    // compare static arguments
    match (&left.static_arguments, &right.static_arguments) {
        (None, None) => {}
        (Some(left_args), Some(right_args)) => {
            if !arguments_are_equal(ctx, left_args, right_args) {
                return false;
            }
        }
        _ => return false,
    }

    // compare arguments
    match (&left.arguments, &right.arguments) {
        (None, None) => true,
        (Some(left_args), Some(right_args)) => arguments_are_equal(ctx, left_args, right_args),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_duplicate_decorator() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@inline
@inline
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_flags_duplicate_decorator_with_same_args() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@cache(100)
@cache(100)
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_different_decorators() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@inline
@deprecated
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_same_decorator_different_args() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@validate({ min: 1 })
@validate({ max: 100 })
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_same_decorator_different_numeric_args() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@cache(10)
@cache(20)
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_allows_single_decorator() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@inline
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-duplicate-decorators");
    }

    #[test]
    fn test_flags_triple_decorator() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@inline
@inline
@inline
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint_count("no-duplicate-decorators", 2);
    }

    #[test]
    fn test_flags_only_exact_duplicates_among_multiple() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateDecorators);
        let result = test.lint_ast(
            "test.ds",
            r#"
@cache(10)
@cache(20)
@cache(10)
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint_count("no-duplicate-decorators", 1);
    }
}
