use std::collections::HashMap;

use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate decorators on the same target.
    ///
    /// Duplicate decorators are usually a mistake from copy-paste or accidental
    /// repetition. Most decorators should only appear once on any given target.
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
            // group decorators by name
            let mut seen: HashMap<ast::StringId, ast::LocalNodeId<ast::Annotation>> =
                HashMap::new();

            for annotation_id in annotations {
                let annotation = ctx.tree.get(*annotation_id);
                let ast::Annotation::Decorator { node, .. } = annotation else {
                    continue;
                };

                // extract decorator name
                let decorator = ctx.tree.get(*node);
                let Some(name_id) = decorator.left.segments.first() else {
                    continue;
                };

                // check for duplicate
                if let Some(first_id) = seen.get(name_id) {
                    let severity = ctx.get_effective_severity(meta, *first_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let name_ref = ctx.strings.get(*name_id);
                    let name = name_ref.as_ref();
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
                        .with_label("this decorator is already applied")
                        .with_fix(fix),
                    );
                } else {
                    seen.insert(*name_id, *annotation_id);
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
}
