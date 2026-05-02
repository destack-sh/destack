use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    ExpressionDuplicateTracker, expression_numeric_value, match_selector_expression_id,
};
use crate::{ConstValue, LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow duplicate case labels in switch statements.
    ///
    /// Having duplicate case labels in a switch statement is almost always a mistake.
    /// Only the first matching case will be executed.
    #[lint(
        id = "no-duplicate-case",
        code = "LC012",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoDuplicateCase,
    "Disallow duplicate case labels"
}

impl LintRule for NoDuplicateCase {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoDuplicateCase::meta()
    }

    /// Check module AST nodes for duplicate switch case selectors.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Match { kind, cases, .. } = ctx.tree.get(node_id) else {
                continue;
            };

            // only check switch statements, not match expressions
            if *kind != ast::MatchKind::Switch {
                continue;
            }

            // collect case expression IDs and check for duplicates
            let mut seen = ExpressionDuplicateTracker::new();
            let mut seen_constant_values: Vec<ConstValue> = Vec::new();
            let mut seen_number_values: Vec<f64> = Vec::new();
            for case_id in cases {
                let case = ctx.tree.get(*case_id);
                let Some(expr_id) = match_selector_expression_id(ctx, case.selector()) else {
                    continue;
                };

                // check against all previously seen expressions
                let mut has_duplicate = seen.find_duplicate_or_insert(ctx, expr_id).is_some();
                let mut constant_value: Option<ConstValue> = None;

                // cache exact constant value lookup
                if !has_duplicate {
                    constant_value = ctx.const_value(expr_id);
                }

                // check exact constant value duplicates
                if !has_duplicate && let Some(constant_value) = constant_value {
                    if seen_constant_values.contains(&constant_value)
                        || matches!(
                            constant_value,
                            ConstValue::Integer(value)
                                if seen_number_values.contains(&(value as f64))
                        )
                        || matches!(
                            constant_value,
                            ConstValue::Float(value) if seen_number_values.contains(&value)
                        )
                    {
                        has_duplicate = true;
                    } else {
                        seen_constant_values.push(constant_value);

                        // enforce this lint guard
                        if let Some(number_value) = number_const_value(constant_value) {
                            seen_number_values.push(number_value);
                        }
                    }
                }

                // check numeric folded duplicate values for arithmetic expressions
                if !has_duplicate
                    && constant_value.is_none()
                    && let Some(number_value) = expression_numeric_value(ctx, expr_id)
                {
                    if seen_number_values.contains(&number_value) {
                        has_duplicate = true;
                    } else {
                        seen_number_values.push(number_value);
                    }
                }

                // enforce this lint guard
                if has_duplicate {
                    let severity = ctx.get_effective_severity(meta, expr_id);
                    if !severity.is_enabled() {
                        continue;
                    }
                    let mut diagnostic = LintReport::new(
                        NO_DUPLICATE_CASE.id,
                        NO_DUPLICATE_CASE.code,
                        NO_DUPLICATE_CASE.category,
                        severity,
                        "duplicate case label",
                        ctx.tree.get_span(*case_id),
                    )
                    .label("this case was already handled");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = duplicate_case_fix(ctx, *case_id)
                    {
                        diagnostic = diagnostic.fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

/// Return numeric representation for number-like constant values.
fn number_const_value(value: ConstValue) -> Option<f64> {
    match value {
        ConstValue::Integer(value) => Some(value as f64),
        ConstValue::Float(value) => Some(value),
        _ => None,
    }
}

/// Build an unsafe fix that removes the duplicate switch case.
fn duplicate_case_fix(
    ctx: &LintAstContext<'_>,
    case_id: ast::LocalNodeId<ast::MatchCase>,
) -> Option<LintFix> {
    let case_span = ctx.tree.get_span(case_id);
    let edits = ctx.edit_builder().replace(case_span, "").into_edits();
    Some(LintFix::r#unsafe("Remove duplicate switch case").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_duplicate_integer_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_detects_duplicate_integer_case.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 1: break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_fix_removes_duplicate_integer_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_fix_removes_duplicate_integer_case.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 1: break;
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-case")
            .assert_unsafe_fixed(
                r#"
let x = 1;
switch (x) {
    case 1: break
    case 2: break
}
"#,
            );
    }

    #[test]
    fn test_detects_duplicate_string_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_detects_duplicate_string_case.ds",
            r#"
let x = "a";
switch (x) {
    case "a": break;
    case "b": break;
    case "a": break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_detects_duplicate_boolean_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_detects_duplicate_boolean_case.ds",
            r#"
let x = true;
switch (x) {
    case true: break;
    case false: break;
    case true: break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }

    #[test]
    fn test_allows_unique_cases() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_allows_unique_cases.ds",
            r#"
let x = 1;
switch (x) {
    case 1: break;
    case 2: break;
    case 3: break;
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-case");
    }

    #[test]
    fn test_ignores_match_expression() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_ignores_match_expression.ds",
            r#"
let x = 1;
match (x) {
    1 => 1
    2 => 2
}
"#,
        );
        test.result(result).assert_no_lint("no-duplicate-case");
    }

    #[test]
    fn test_mutation_fix_removes_duplicate_string_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_mutation_fix_removes_duplicate_string_case.ds",
            r#"
let x = "a";
switch (x) {
    case "a": break;
    case "b": break;
    case "a": break;
}
"#,
        );
        test.result(result)
            .assert_lint("no-duplicate-case")
            .assert_unsafe_fixed(
                r#"
let x = 'a';
switch (x) {
    case 'a': break
    case 'b': break
}
"#,
            );
    }

    #[test]
    fn test_detects_duplicate_constant_folded_case() {
        let test = TestProgram::for_rule_without_prelude(NoDuplicateCase);
        let result = test.lint_ast(
            "no_duplicate_case/test_detects_duplicate_constant_folded_case.ds",
            r#"
let x = 2;
switch (x) {
    case 1 + 1: break;
    case 2: break;
}
"#,
        );
        test.result(result).assert_lint("no-duplicate-case");
    }
}
