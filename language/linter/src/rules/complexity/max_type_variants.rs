use crate::LintMeta;
use destack_dir::{self as dir, TypeExpression};
use destack_workspace::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of variants in a union type or enum.
    ///
    /// Types with many variants are harder to understand and evolve.
    /// Consider grouping related variants or splitting large unions.
    #[lint(
        id = "max-type-variants",
        code = "LX014",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxTypeVariants,
    "Limit type variants"
}

impl LintRule for MaxTypeVariants {
    fn meta(&self) -> &'static LintMeta {
        MaxTypeVariants::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_type_variants = ctx.options.complexity.max_type_variants;

        // check top level union type expressions only
        for type_expression_id in ctx.dir.iter_nodes::<dir::TypeExpression>() {
            let dir::TypeExpression::Union { .. } = ctx.dir.get(type_expression_id) else {
                continue;
            };
            if union_has_parent_union(ctx, type_expression_id) {
                continue;
            }

            let variant_count = count_union_variants(ctx, type_expression_id);
            if variant_count > max_type_variants {
                report_variant_overflow(
                    ctx,
                    meta,
                    type_expression_id,
                    "union type",
                    variant_count,
                    max_type_variants,
                );
            }
        }

        // check enum variant counts
        for declaration_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(declaration_id);
            let dir::Declaration::Enum(declaration) = declaration else {
                continue;
            };
            let variant_count = declaration.fields.len();
            if variant_count > max_type_variants {
                report_variant_overflow(
                    ctx,
                    meta,
                    declaration_id,
                    "enum",
                    variant_count,
                    max_type_variants,
                );
            }
        }
    }
}

/// Return true when one union expression has an outer union parent.
fn union_has_parent_union(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> bool {
    let Some(parent_id) = ctx.dir.get_parent_id(type_expression_id.id) else {
        return false;
    };
    if ctx.dir.get_node_type(parent_id) != dir::NodeType::TypeExpression {
        return false;
    }

    // keep only parent union expressions
    let parent_expression_id = dir::LocalNodeId::<dir::TypeExpression>::new(parent_id);
    matches!(
        ctx.dir.get(parent_expression_id),
        dir::TypeExpression::Union { .. }
    )
}

/// Count flattened union variants for one union expression.
fn count_union_variants(
    ctx: &LintModuleContext<'_>,
    type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
) -> usize {
    let type_expression = ctx.dir.get(type_expression_id);
    let TypeExpression::Union { elements } = type_expression else {
        return 1;
    };

    elements
        .iter()
        .map(|element_id| count_union_variants(ctx, *element_id))
        .sum()
}

/// Report one variant count overflow diagnostic.
fn report_variant_overflow<T: dir::Node + Clone>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: dir::LocalNodeId<T>,
    type_kind: &str,
    variant_count: usize,
    max_type_variants: usize,
) where
    dir::Tree: dir::TreeStore<T>,
{
    // resolve owner span before moving owner id into severity lookup
    let owner_span = ctx.dir.get_span(owner_id);

    // resolve effective severity
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // emit one variant count overflow diagnostic
    ctx.report(
        LintReport::new(
            MAX_TYPE_VARIANTS.id,
            MAX_TYPE_VARIANTS.code,
            MAX_TYPE_VARIANTS.category,
            severity,
            format!("{type_kind} has {variant_count} variants (max {max_type_variants})"),
            owner_span,
        )
        .label("consider grouping related variants"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_union_members() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeVariants)
            .with_options(|options| options.complexity.max_type_variants = 5);
        let result = test.lint(
            "max_type_variants/test_detects_too_many_union_members.ds",
            r#"
type BigUnion = A | B | C | D | E | F;
"#,
        );
        test.result(result).assert_lint("max-type-variants");
    }

    #[test]
    fn test_allows_few_union_members() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeVariants);
        let result = test.lint(
            "max_type_variants/test_allows_few_union_members.ds",
            r#"
type SmallUnion = A | B | C;
"#,
        );
        test.result(result).assert_no_lint("max-type-variants");
    }

    #[test]
    fn test_counts_unions_in_parameter_types() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeVariants)
            .with_options(|options| options.complexity.max_type_variants = 3);
        let result = test.lint(
            "max_type_variants/test_counts_unions_in_parameter_types.ds",
            r#"
function test(x: A | B | C | D): void {}
"#,
        );
        test.result(result).assert_lint("max-type-variants");
    }

    #[test]
    fn test_ignores_runtime_bitwise_or_expressions() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeVariants)
            .with_options(|options| options.complexity.max_type_variants = 1);
        let result = test.lint(
            "max_type_variants/test_ignores_runtime_bitwise_or_expressions.ds",
            r#"
let x = a | b | c | d;
"#,
        );
        test.result(result).assert_no_lint("max-type-variants");
    }

    #[test]
    fn test_detects_too_many_enum_variants() {
        let test = TestProgram::for_rule_without_prelude(MaxTypeVariants)
            .with_options(|options| options.complexity.max_type_variants = 3);
        let result = test.lint(
            "max_type_variants/test_detects_too_many_enum_variants.ds",
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
}
