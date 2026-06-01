use destack_dir::{self as dir};
use destack_source::LabeledSpan;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    binary_expression_chain_members, binary_expression_is_nested_same_operator,
    expression_is_in_type_position, expression_type_map, normalized_flow_type_id,
};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate constituents in union and intersection types.
    ///
    /// Duplicate constituents add noise and do not change type meaning.
    /// Example: `A | A | B` should be simplified to `A | B`.
    #[lint(
        id = "no-duplicate-type-constituents",
        code = "LY072",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoDuplicateTypeConstituents,
    "Disallow duplicate constituents in union and intersection types"
}

impl LintRule for NoDuplicateTypeConstituents {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoDuplicateTypeConstituents::meta()
    }

    /// Check module DIR nodes for duplicate type constituent chains.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inspect top-level type union and intersection chains
        for expression_id in ctx.dir.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.dir.get(expression_id);
            let dir::Expression::Binary { operator, .. } = expression else {
                continue;
            };

            // keep non-type constituent operators out of this rule
            if !matches!(
                operator,
                dir::BinaryOperator::ElementwiseOr | dir::BinaryOperator::ElementwiseAnd
            ) {
                continue;
            }
            if binary_expression_is_nested_same_operator(ctx.dir.tree(), expression_id, *operator) {
                continue;
            }
            if !expression_is_in_type_position(ctx.dir.tree(), expression_id) {
                continue;
            }

            report_duplicate_constituents(ctx, meta, expression_id, *operator);
        }
    }
}

/// One type constituent with normalized type metadata.
#[derive(Debug, Clone, Copy)]
struct Constituent {
    /// The expression node id.
    expression_id: dir::LocalNodeId<dir::Expression>,
    /// The normalized type id for semantic duplicate checks.
    normalized_type_id: dir::GlobalTypeId,
}

/// Report duplicate constituents for one top-level type chain.
fn report_duplicate_constituents(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    expression_id: dir::LocalNodeId<dir::Expression>,
    operator: dir::BinaryOperator,
) {
    let mut expression_constituents = Vec::new();
    binary_expression_chain_members(
        ctx.dir.tree(),
        expression_id,
        operator,
        &mut expression_constituents,
    );
    if expression_constituents.len() < 2 {
        return;
    }

    // resolve normalized type ids for every constituent first
    let mut constituents = Vec::new();
    for constituent_expression_id in expression_constituents {
        let Some(type_id) =
            expression_type_map(ctx, constituent_expression_id, |_ctx, type_id| type_id)
        else {
            return;
        };
        let normalized_type_id = normalized_flow_type_id(ctx, type_id);
        constituents.push(Constituent {
            expression_id: constituent_expression_id,
            normalized_type_id,
        });
    }

    // keep only later constituents with the same normalized type
    let duplicate_pairs = duplicate_constituent_pairs(&constituents);
    if duplicate_pairs.is_empty() {
        return;
    }

    // build one deduplicated replacement for a non-overlapping auto-fix
    let replacement =
        deduplicated_constituent_replacement(ctx, &constituents, operator, &duplicate_pairs);

    for (duplicate_position, (duplicate_index, first_index)) in duplicate_pairs.iter().enumerate() {
        let duplicate_constituent = constituents[*duplicate_index];
        let first_constituent = constituents[*first_index];

        // honor per node severity
        let severity = ctx.get_effective_severity(meta, duplicate_constituent.expression_id);
        if !severity.is_enabled() {
            continue;
        }

        // report the duplicate constituent span and the first occurrence
        let duplicate_span = ctx.get_span(duplicate_constituent.expression_id);
        let first_span = ctx.get_span(first_constituent.expression_id);
        let mut diagnostic = LintReport::new(
            NO_DUPLICATE_TYPE_CONSTITUENTS.id,
            NO_DUPLICATE_TYPE_CONSTITUENTS.code,
            NO_DUPLICATE_TYPE_CONSTITUENTS.category,
            severity,
            "duplicate type constituent",
            duplicate_span,
        )
        .label("this constituent duplicates an earlier constituent")
        .secondary(LabeledSpan::new(first_span, "first occurrence is here"));

        // attach one replacement fix once to avoid overlapping edit conflicts
        if duplicate_position == 0
            && ctx.compute_fixes
            && let Some(replacement_text) = replacement.as_ref()
        {
            let chain_span = ctx.get_span(expression_id);
            let edits = ctx
                .edit_builder()
                .replace(chain_span, replacement_text)
                .into_edits();
            let fix = LintFix::safe("Remove duplicate type constituents and keep unique members")
                .with_edits(edits);
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
    }
}

/// Return duplicate constituent pairs as `(duplicate_index, first_index)`.
fn duplicate_constituent_pairs(constituents: &[Constituent]) -> Vec<(usize, usize)> {
    let mut duplicate_pairs = Vec::new();

    // remove later matching constituents and preserve source order
    for left_index in 0..constituents.len() {
        let left_type_id = constituents[left_index].normalized_type_id;

        for (right_index, right_constituent) in constituents.iter().enumerate().skip(left_index + 1)
        {
            let right_type_id = right_constituent.normalized_type_id;
            if left_type_id != right_type_id {
                continue;
            }

            duplicate_pairs.push((right_index, left_index));
        }
    }

    // keep only the first earlier match for each duplicate position
    duplicate_pairs.sort_unstable();
    duplicate_pairs.dedup_by_key(|(duplicate_index, _)| *duplicate_index);
    duplicate_pairs
}

/// Build one deduplicated replacement expression text.
fn deduplicated_constituent_replacement(
    ctx: &LintModuleContext<'_>,
    constituents: &[Constituent],
    operator: dir::BinaryOperator,
    duplicate_pairs: &[(usize, usize)],
) -> Option<String> {
    if duplicate_pairs.is_empty() {
        return None;
    }

    // mark duplicate positions once and keep the first occurrence order stable
    let mut duplicate_index_flags = vec![false; constituents.len()];
    for (duplicate_index, _) in duplicate_pairs {
        duplicate_index_flags[*duplicate_index] = true;
    }

    let mut unique_texts = Vec::<String>::new();
    for (index, constituent) in constituents.iter().enumerate() {
        if duplicate_index_flags[index] {
            continue;
        }

        let constituent_text = ctx
            .get_span_text(ctx.get_span(constituent.expression_id))
            .trim()
            .to_string();
        if constituent_text.is_empty() {
            continue;
        }

        unique_texts.push(constituent_text);
    }

    if unique_texts.is_empty() || unique_texts.len() == constituents.len() {
        return None;
    }

    let separator = match operator {
        dir::BinaryOperator::ElementwiseOr => " | ",
        dir::BinaryOperator::ElementwiseAnd => " & ",
        _ => return None,
    };
    Some(unique_texts.join(separator))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_duplicate_union_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_flags_duplicate_union_constituents.ds",
            r#"
type Value = string | number | string;
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    #[test]
    fn test_flags_duplicate_intersection_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_flags_duplicate_intersection_constituents.ds",
            r#"
interface Readable {
    read(): string;
}

interface Writable {
    write(value: string): void;
}

type Value = Readable & Writable & Readable;
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    #[test]
    fn test_allows_unique_type_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_allows_unique_type_constituents.ds",
            r#"
type Value = string | number | boolean;
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("no-duplicate-type-constituents");
    }

    #[test]
    fn test_ignores_value_level_elementwise_expressions() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_ignores_value_level_elementwise_expressions.ds",
            r#"
let a = 1;
let b = 2;
let result = a | b | a;
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_no_lint("no-duplicate-type-constituents");
    }

    #[test]
    fn test_fix_deduplicates_union_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_fix_deduplicates_union_constituents.ds",
            r#"
type Value = string | number | string | boolean | number;
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("no-duplicate-type-constituents", 2)
            .assert_safe_fixed(
                r#"
type Value = string | number | boolean;
"#,
            );
    }

    #[test]
    fn test_fix_deduplicates_intersection_constituents() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_fix_deduplicates_intersection_constituents.ds",
            r#"
interface Readable {
    read(): string;
}

interface Writable {
    write(value: string): void;
}

type Value = Readable & Writable & Readable & Writable;
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint_count("no-duplicate-type-constituents", 2)
            .assert_safe_fixed(
                r#"
interface Readable {
    read(): string;
}

interface Writable {
    write(value: string): void;
}

type Value = Readable & Writable;
"#,
            );
    }

    #[test]
    fn test_flags_duplicate_constituents_in_function_return_type() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_flags_duplicate_constituents_in_function_return_type.ds",
            r#"
function value(): string | string {
    return "ok";
}
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    #[test]
    fn test_flags_duplicate_string_literal_constituents_with_different_quote_style() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_flags_duplicate_string_literal_constituents_with_different_quote_style.ds",
            r#"
type Value = "ok" | 'ok' | "next";
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }

    #[test]
    fn test_flags_nested_duplicate_constituent_chain() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateTypeConstituents);
        let result = test.lint_dir(
            "no_duplicate_type_constituents/test_flags_nested_duplicate_constituent_chain.ds",
            r#"
type Alpha = "alpha";
type Beta = "beta";
type Value = (Alpha | Beta) | Alpha;
"#,
        );
        test.check_clean();
        test.result(result)
            .assert_lint("no-duplicate-type-constituents");
    }
}
