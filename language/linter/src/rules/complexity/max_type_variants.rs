use destack_ast::{self as ast, BinaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of variants in a union type or enum.
    ///
    /// Types with many variants can be hard to understand and maintain.
    /// Consider grouping related variants or simplifying the design.
    #[lint(
        id = "max-type-variants",
        code = "LX015",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxTypeVariants,
    "Limit type variants"
}

impl LintRule for MaxTypeVariants {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxTypeVariants::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_type_variants = ctx.options.max_type_variants;

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            match expression {
                // check union types (A | B | C)
                ast::Expression::Binary {
                    operator: BinaryOperator::ElementwiseOr,
                    ..
                } => {
                    // only check top-level unions (not nested in another union)
                    if is_nested_in_union(ctx, node_id) {
                        continue;
                    }
                    let variant_count = count_union_members(ctx, node_id);
                    if variant_count > max_type_variants {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
                        ctx.report(
                            LintDiagnostic::new(
                                MAX_TYPE_VARIANTS.id,
                                MAX_TYPE_VARIANTS.code,
                                MAX_TYPE_VARIANTS.category,
                                severity,
                                format!(
                                    "union type has {variant_count} variants (max {max_type_variants})"
                                ),
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("consider grouping related variants"),
                        );
                    }
                }

                // check enum declarations
                ast::Expression::Declaration(declaration_id) => {
                    let declaration = ctx.tree.get(*declaration_id);
                    let ast::Declaration::Enum { fields, .. } = declaration else {
                        continue;
                    };
                    let variant_count = fields.len();
                    if variant_count > max_type_variants {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
                        ctx.report(
                            LintDiagnostic::new(
                                MAX_TYPE_VARIANTS.id,
                                MAX_TYPE_VARIANTS.code,
                                MAX_TYPE_VARIANTS.category,
                                severity,
                                format!(
                                    "enum has {variant_count} variants (max {max_type_variants})"
                                ),
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("consider grouping related variants"),
                        );
                    }
                }

                _ => continue,
            }
        }
    }
}

/// Check if an expression is nested inside another union.
fn is_nested_in_union(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    // check if parent is also a union (ElementwiseOr binary expression)
    if let Some(parent_raw_id) = ctx.parents.get_by_id(expr_id.id) {
        let parent_type = ctx.tree.get_node_type(parent_raw_id);
        if parent_type != ast::NodeType::Expression {
            return false;
        }
        let parent_id = ast::LocalNodeId::<ast::Expression>::new(parent_raw_id);
        let parent = ctx.tree.get(parent_id);
        if let ast::Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        } = parent
        {
            return true;
        }
    }
    false
}

/// Count the number of members in a union type.
fn count_union_members(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> usize {
    let expression = ctx.tree.get(expr_id);

    match expression {
        ast::Expression::Binary {
            left,
            operator: BinaryOperator::ElementwiseOr,
            right,
        } => {
            // recursively count members on both sides
            count_union_members(ctx, *left) + count_union_members(ctx, *right)
        }
        // any other expression is a single member
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_union_members() {
        let test = TestProgram::for_rule_without_builtins(MaxTypeVariants)
            .with_options(|options| options.max_type_variants = 5);
        let result = test.lint_ast(
            "test.ds",
            r#"
type BigUnion = A | B | C | D | E | F;
"#,
        );
        test.result(result).assert_lint("max-type-variants");
    }

    #[test]
    fn test_allows_few_union_members() {
        let test = TestProgram::for_rule_without_builtins(MaxTypeVariants);
        let result = test.lint_ast(
            "test.ds",
            r#"
type SmallUnion = A | B | C;
"#,
        );
        test.result(result).assert_no_lint("max-type-variants");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_builtins(MaxTypeVariants)
            .with_options(|options| options.max_type_variants = 5);
        let result = test.lint_ast(
            "test.ds",
            r#"
type AtLimit = A | B | C | D | E;
"#,
        );
        test.result(result).assert_no_lint("max-type-variants");
    }

    #[test]
    fn test_counts_in_function_parameter() {
        let test = TestProgram::for_rule_without_builtins(MaxTypeVariants)
            .with_options(|options| options.max_type_variants = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
function test(x: A | B | C | D): void {}
"#,
        );
        test.result(result).assert_lint("max-type-variants");
    }

    #[test]
    fn test_detects_too_many_enum_variants() {
        let test = TestProgram::for_rule_without_builtins(MaxTypeVariants)
            .with_options(|options| options.max_type_variants = 3);
        let result = test.lint_ast(
            "test.ds",
            r#"
enum TooMany {
    A,
    B,
    C,
    D,
}
"#,
        );
        test.result(result).assert_lint("max-type-variants");
    }

    #[test]
    fn test_allows_few_enum_variants() {
        let test = TestProgram::for_rule_without_builtins(MaxTypeVariants);
        let result = test.lint_ast(
            "test.ds",
            r#"
enum SmallEnum {
    A,
    B,
    C,
}
"#,
        );
        test.result(result).assert_no_lint("max-type-variants");
    }
}
